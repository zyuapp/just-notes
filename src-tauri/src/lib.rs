use std::{
    collections::VecDeque,
    env,
    ffi::{c_void, CStr},
    fs,
    fs::OpenOptions,
    io::{BufRead, BufReader, Write},
    mem,
    path::{Path, PathBuf},
    ptr::{self, NonNull},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, SampleFormat, Stream, StreamConfig, SupportedStreamConfig,
};
use objc2::AnyThread;
use objc2_core_audio::{
    kAudioAggregateDeviceIsPrivateKey, kAudioAggregateDeviceNameKey,
    kAudioAggregateDevicePropertyTapList, kAudioAggregateDeviceUIDKey, kAudioHardwareNoError,
    kAudioObjectPropertyElementMain, kAudioObjectPropertyScopeGlobal, kAudioTapPropertyUID,
    AudioHardwareCreateAggregateDevice, AudioHardwareCreateProcessTap,
    AudioHardwareDestroyAggregateDevice, AudioHardwareDestroyProcessTap,
    AudioObjectGetPropertyData, AudioObjectID, AudioObjectPropertyAddress,
    AudioObjectSetPropertyData, CATapDescription, CATapMuteBehavior,
};
use objc2_core_foundation::{CFArray, CFBoolean, CFDictionary, CFString, CFType};
use objc2_foundation::{NSArray, NSNumber, NSString};
use tauri::{AppHandle, Emitter, Manager};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

const LIVE_TRANSCRIPTION_STEP_MS: u64 = 2_000;
const LIVE_TRANSCRIPTION_WINDOW_MS: u64 = 12_000;
const LIVE_TRANSCRIPTION_MAX_AGREEMENT_BUFFER_MS: u64 = 30_000;
const LIVE_TRANSCRIPTION_STABILITY_DELAY_MS: u64 = 2_000;
const LIVE_TRANSCRIPTION_POLL_MS: u64 = 250;
const LIVE_SILENCE_RMS_THRESHOLD: f32 = 0.005;
const LIVE_DUPLICATE_RECENT_SEGMENTS: usize = 8;
const LIVE_DUPLICATE_NGRAM_SIZE: usize = 3;
const LIVE_DUPLICATE_COVERAGE_THRESHOLD: f32 = 0.72;
const LIVE_DUPLICATE_MIN_KEEP_WORDS: usize = 4;
const LIVE_DUPLICATE_MIN_TRIM_WORDS: usize = 6;
const MAX_ROLLING_BUFFER_MS: u64 = 120_000;
#[cfg(any(debug_assertions, feature = "qa-fixtures"))]
const FIXTURE_CHUNK_MS: u64 = 50;
const SYSTEM_CAPTURE_DEVICE_NAME: &str = "Just Notes System Audio";
const WHISPER_MODEL_CANDIDATES: [WhisperModelCandidate; 3] = [
    WhisperModelCandidate {
        name: "medium.en",
        filename: "ggml-medium.en.bin",
    },
    WhisperModelCandidate {
        name: "small.en",
        filename: "ggml-small.en.bin",
    },
    WhisperModelCandidate {
        name: "base.en",
        filename: "ggml-base.en.bin",
    },
];

struct WhisperModelCandidate {
    name: &'static str,
    filename: &'static str,
}

#[derive(Clone)]
struct AppPaths {
    data_dir: PathBuf,
    threads_dir: PathBuf,
}

impl AppPaths {
    fn discover() -> Result<Self, String> {
        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| "HOME is not set; cannot locate ~/.just-notes".to_string())?;
        let data_dir = home.join(".just-notes");
        Ok(Self {
            threads_dir: data_dir.join("threads"),
            data_dir,
        })
    }

    fn ensure(&self) -> Result<(), String> {
        fs::create_dir_all(&self.threads_dir).map_err(|err| {
            format!(
                "Failed to create thread storage at {}: {err}",
                self.threads_dir.display()
            )
        })?;
        fs::create_dir_all(self.data_dir.join("engine")).map_err(|err| {
            format!(
                "Failed to create engine storage at {}: {err}",
                self.data_dir.join("engine").display()
            )
        })?;
        fs::create_dir_all(self.data_dir.join("models").join("whisper")).map_err(|err| {
            format!(
                "Failed to create model storage at {}: {err}",
                self.data_dir.join("models").join("whisper").display()
            )
        })
    }

    fn thread_dir(&self, thread_id: &str) -> PathBuf {
        self.threads_dir.join(thread_id)
    }

    fn transcription_paths(&self) -> TranscriptionPaths {
        let mut available_models =
            discover_whisper_models(&self.data_dir.join("models").join("whisper"));
        let selected_model = available_models
            .iter()
            .find(|model| model.installed)
            .cloned()
            .unwrap_or_else(|| {
                available_models
                    .last()
                    .expect("whisper model candidates")
                    .clone()
            });
        for model in &mut available_models {
            model.selected = model.filename == selected_model.filename;
        }

        TranscriptionPaths {
            model_path: selected_model.path.clone(),
            model_name: selected_model.name.clone(),
            available_models,
        }
    }
}

fn discover_whisper_models(model_dir: &Path) -> Vec<WhisperModelStatus> {
    WHISPER_MODEL_CANDIDATES
        .iter()
        .map(|candidate| {
            let path = model_dir.join(candidate.filename);
            WhisperModelStatus {
                name: candidate.name.to_string(),
                filename: candidate.filename.to_string(),
                installed: path.is_file(),
                path,
                selected: false,
            }
        })
        .collect()
}

#[derive(Clone, Default)]
struct RecorderState {
    session: Arc<Mutex<Option<RecorderSession>>>,
    is_starting: Arc<AtomicBool>,
}

struct RecorderSession {
    thread_id: String,
    thread_dir: PathBuf,
    started: Instant,
    buffers: Arc<Mutex<SharedBuffers>>,
    should_stop_meter: Arc<AtomicBool>,
    should_stop_live_transcription: Arc<AtomicBool>,
    meter_thread: Option<JoinHandle<()>>,
    live_transcription_thread: Option<JoinHandle<()>>,
    audio_capture: ActiveAudioCapture,
}

enum ActiveAudioCapture {
    Devices {
        _mic_stream: Stream,
        _system_capture: SystemAudioCapture,
    },
    #[cfg(any(debug_assertions, feature = "qa-fixtures"))]
    Fixture(FixtureAudioCapture),
}

#[cfg(any(debug_assertions, feature = "qa-fixtures"))]
struct FixtureAudioCapture {
    should_stop: Arc<AtomicBool>,
    workers: Vec<JoinHandle<()>>,
}

enum RecordingInputMode {
    Devices,
    #[cfg(any(debug_assertions, feature = "qa-fixtures"))]
    Fixture {
        mic_path: PathBuf,
        system_path: PathBuf,
    },
}

struct PreparedAudioInput {
    mic_sample_rate: u32,
    system_sample_rate: u32,
    buffers: Arc<Mutex<SharedBuffers>>,
    audio_capture: ActiveAudioCapture,
}

struct SharedBuffers {
    mic: RollingChannel,
    system: RollingChannel,
}

impl SharedBuffers {
    fn new(mic_sample_rate: u32, system_sample_rate: u32) -> Self {
        Self {
            mic: RollingChannel::new(mic_sample_rate),
            system: RollingChannel::new(system_sample_rate),
        }
    }
}

struct RollingChannel {
    samples: VecDeque<f32>,
    base_index: u64,
    max_samples: usize,
    level: f32,
}

impl RollingChannel {
    fn new(sample_rate: u32) -> Self {
        let max_samples = ((sample_rate as u64 * MAX_ROLLING_BUFFER_MS) / 1000).max(1) as usize;
        Self {
            samples: VecDeque::with_capacity(max_samples.min(sample_rate as usize * 10)),
            base_index: 0,
            max_samples,
            level: 0.0,
        }
    }

    fn push(&mut self, chunk: &[f32], level: f32) {
        self.samples.extend(chunk.iter().copied());
        let overflow = self.samples.len().saturating_sub(self.max_samples);
        if overflow > 0 {
            self.samples.drain(0..overflow);
            self.base_index += overflow as u64;
        }
        self.level = smooth_level(self.level, level);
    }

    fn available_end_index(&self) -> u64 {
        self.base_index + self.samples.len() as u64
    }

    fn window(&self, start_index: u64, end_index: u64) -> Option<Vec<f32>> {
        if start_index < self.base_index || end_index > self.available_end_index() {
            return None;
        }
        let start = (start_index - self.base_index) as usize;
        let end = (end_index - self.base_index) as usize;
        if end <= start {
            return None;
        }
        Some(self.samples.range(start..end).copied().collect())
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ThreadMetadata {
    id: String,
    title: String,
    created_at_ms: u64,
    updated_at_ms: u64,
    status: ThreadStatus,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum ThreadStatus {
    Idle,
    Recording,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ThreadSummary {
    id: String,
    title: String,
    created_at_ms: u64,
    updated_at_ms: u64,
    status: ThreadStatus,
    segment_count: usize,
    path: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TranscriptSegment {
    speaker: String,
    source: String,
    start_ms: u64,
    end_ms: u64,
    text: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ThreadDetail {
    summary: ThreadSummary,
    segments: Vec<TranscriptSegment>,
    transcript_markdown_path: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    data_dir: String,
    threads_dir: String,
    fixture_mode: bool,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct MeterPayload {
    thread_id: String,
    mic_level: f32,
    system_level: f32,
    elapsed_ms: u64,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct RecordingPayload {
    thread: ThreadDetail,
    transcription: TranscriptionStatusPayload,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct LiveTranscriptSegmentPayload {
    thread_id: String,
    committed_until_ms: u64,
    segment: TranscriptSegment,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct LiveTranscriptStatusPayload {
    thread_id: String,
    active: bool,
    message: String,
    chunk_ms: u64,
    overlap_ms: u64,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TranscriptionStatusPayload {
    ready: bool,
    engine_exists: bool,
    model_exists: bool,
    engine_path: String,
    model_path: String,
    model_name: String,
    available_models: Vec<WhisperModelStatus>,
    message: String,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct WhisperModelStatus {
    name: String,
    filename: String,
    path: PathBuf,
    installed: bool,
    selected: bool,
}

#[derive(Clone)]
struct TranscriptionPaths {
    model_path: PathBuf,
    model_name: String,
    available_models: Vec<WhisperModelStatus>,
}

#[tauri::command]
fn get_app_info(paths: tauri::State<'_, AppPaths>) -> AppInfo {
    let paths = paths.inner();
    AppInfo {
        data_dir: paths.data_dir.display().to_string(),
        threads_dir: paths.threads_dir.display().to_string(),
        fixture_mode: cfg!(any(debug_assertions, feature = "qa-fixtures")),
    }
}

#[tauri::command]
fn list_threads(paths: tauri::State<'_, AppPaths>) -> Result<Vec<ThreadSummary>, String> {
    let paths = paths.inner();
    paths.ensure()?;

    let mut threads = fs::read_dir(&paths.threads_dir)
        .map_err(|err| format!("Failed to read {}: {err}", paths.threads_dir.display()))?
        .filter_map(Result::ok)
        .filter_map(|entry| load_thread_summary(&entry.path()).ok())
        .collect::<Vec<_>>();

    threads.sort_by(|left, right| {
        right
            .updated_at_ms
            .cmp(&left.updated_at_ms)
            .then_with(|| right.created_at_ms.cmp(&left.created_at_ms))
    });
    Ok(threads)
}

#[tauri::command]
fn create_thread(paths: tauri::State<'_, AppPaths>) -> Result<ThreadDetail, String> {
    create_thread_inner(paths.inner())
}

#[tauri::command]
fn get_thread(
    paths: tauri::State<'_, AppPaths>,
    thread_id: String,
) -> Result<ThreadDetail, String> {
    load_thread_by_id(paths.inner(), &thread_id)
}

#[tauri::command]
fn get_transcription_status(
    paths: tauri::State<'_, AppPaths>,
) -> Result<TranscriptionStatusPayload, String> {
    Ok(transcription_status(paths.inner()))
}

#[tauri::command]
async fn start_recording(
    app: AppHandle,
    paths: tauri::State<'_, AppPaths>,
    recorder: tauri::State<'_, RecorderState>,
    thread_id: Option<String>,
) -> Result<RecordingPayload, String> {
    let paths = paths.inner().clone();
    let recorder = recorder.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        start_recording_inner(app, paths, recorder, thread_id)
    })
    .await
    .map_err(|err| format!("Audio startup task failed: {err}"))?
}

#[cfg(any(debug_assertions, feature = "qa-fixtures"))]
#[tauri::command]
async fn start_fixture_recording(
    app: AppHandle,
    paths: tauri::State<'_, AppPaths>,
    recorder: tauri::State<'_, RecorderState>,
    thread_id: Option<String>,
) -> Result<RecordingPayload, String> {
    let paths = paths.inner().clone();
    let recorder = recorder.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let fixture_dir = paths.data_dir.join("fixtures");
        start_recording_inner_with_mode(
            app,
            paths,
            recorder,
            thread_id,
            RecordingInputMode::Fixture {
                mic_path: fixture_dir.join("qa-mic.wav"),
                system_path: fixture_dir.join("qa-system.wav"),
            },
        )
    })
    .await
    .map_err(|err| format!("Fixture startup task failed: {err}"))?
}

#[tauri::command]
async fn stop_recording(
    paths: tauri::State<'_, AppPaths>,
    recorder: tauri::State<'_, RecorderState>,
) -> Result<ThreadDetail, String> {
    let paths = paths.inner().clone();
    let recorder = recorder.inner().clone();
    tauri::async_runtime::spawn_blocking(move || stop_recording_inner(paths, recorder))
        .await
        .map_err(|err| format!("Audio stop task failed: {err}"))?
}

fn create_thread_inner(paths: &AppPaths) -> Result<ThreadDetail, String> {
    paths.ensure()?;

    let now = now_ms()?;
    let id = format!("thread-{now}");
    let thread_dir = paths.thread_dir(&id);
    fs::create_dir_all(thread_dir.join("work")).map_err(|err| {
        format!(
            "Failed to create thread folder at {}: {err}",
            thread_dir.display()
        )
    })?;

    let metadata = ThreadMetadata {
        id,
        title: "Untitled thread".to_string(),
        created_at_ms: now,
        updated_at_ms: now,
        status: ThreadStatus::Idle,
    };
    save_thread_metadata(&thread_dir, &metadata)?;
    write_text_atomic(&thread_dir.join("transcript.md"), "# Untitled thread\n\n")?;

    load_thread_detail(&thread_dir)
}

fn load_thread_by_id(paths: &AppPaths, thread_id: &str) -> Result<ThreadDetail, String> {
    let thread_dir = paths.thread_dir(thread_id);
    if !thread_dir.is_dir() {
        return Err(format!("Thread does not exist: {thread_id}"));
    }

    load_thread_detail(&thread_dir)
}

fn load_thread_detail(thread_dir: &Path) -> Result<ThreadDetail, String> {
    let summary = load_thread_summary(thread_dir)?;
    let segments = read_transcript_jsonl(&thread_dir.join("transcript.jsonl"))?;
    Ok(ThreadDetail {
        summary,
        segments,
        transcript_markdown_path: thread_dir.join("transcript.md").display().to_string(),
    })
}

fn load_thread_summary(thread_dir: &Path) -> Result<ThreadSummary, String> {
    if !thread_dir.is_dir() {
        return Err(format!("Not a thread folder: {}", thread_dir.display()));
    }

    let metadata_path = thread_dir.join("thread.json");
    let metadata = read_thread_metadata(&metadata_path)?;
    Ok(ThreadSummary {
        id: metadata.id,
        title: metadata.title,
        created_at_ms: metadata.created_at_ms,
        updated_at_ms: metadata.updated_at_ms,
        status: metadata.status,
        segment_count: count_jsonl_lines(&thread_dir.join("transcript.jsonl"))?,
        path: thread_dir.display().to_string(),
    })
}

fn read_thread_metadata(path: &Path) -> Result<ThreadMetadata, String> {
    let json = fs::read_to_string(path)
        .map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&json).map_err(|err| format!("Invalid {}: {err}", path.display()))
}

fn save_thread_metadata(thread_dir: &Path, metadata: &ThreadMetadata) -> Result<(), String> {
    let json = serde_json::to_string_pretty(metadata)
        .map_err(|err| format!("Failed to encode thread metadata: {err}"))?;
    write_text_atomic(&thread_dir.join("thread.json"), &json)
}

fn set_thread_status(thread_dir: &Path, status: ThreadStatus) -> Result<(), String> {
    let mut metadata = read_thread_metadata(&thread_dir.join("thread.json"))?;
    metadata.status = status;
    metadata.updated_at_ms = now_ms()?;
    save_thread_metadata(thread_dir, &metadata)
}

fn touch_thread(thread_dir: &Path) -> Result<(), String> {
    let mut metadata = read_thread_metadata(&thread_dir.join("thread.json"))?;
    metadata.updated_at_ms = now_ms()?;
    save_thread_metadata(thread_dir, &metadata)
}

fn reset_stale_recording_threads(paths: &AppPaths) -> Result<(), String> {
    paths.ensure()?;
    for entry in fs::read_dir(&paths.threads_dir)
        .map_err(|err| format!("Failed to read {}: {err}", paths.threads_dir.display()))?
    {
        let entry = entry
            .map_err(|err| format!("Failed to read {}: {err}", paths.threads_dir.display()))?;
        let thread_dir = entry.path();
        if !thread_dir.is_dir() {
            continue;
        }
        let metadata_path = thread_dir.join("thread.json");
        if !metadata_path.is_file() {
            continue;
        }
        let mut metadata = read_thread_metadata(&metadata_path)?;
        if metadata.status == ThreadStatus::Recording {
            metadata.status = ThreadStatus::Idle;
            save_thread_metadata(&thread_dir, &metadata)?;
        }
    }
    Ok(())
}

fn start_recording_inner(
    app: AppHandle,
    paths: AppPaths,
    recorder: RecorderState,
    requested_thread_id: Option<String>,
) -> Result<RecordingPayload, String> {
    start_recording_inner_with_mode(
        app,
        paths,
        recorder,
        requested_thread_id,
        RecordingInputMode::Devices,
    )
}

fn start_recording_inner_with_mode(
    app: AppHandle,
    paths: AppPaths,
    recorder: RecorderState,
    requested_thread_id: Option<String>,
    input_mode: RecordingInputMode,
) -> Result<RecordingPayload, String> {
    if recorder.is_starting.swap(true, Ordering::SeqCst) {
        return Err("Audio startup is already in progress".to_string());
    }

    let result = prepare_recording_session(app, paths, &recorder, requested_thread_id, input_mode);
    recorder.is_starting.store(false, Ordering::SeqCst);
    result
}

fn prepare_recording_session(
    app: AppHandle,
    paths: AppPaths,
    recorder: &RecorderState,
    requested_thread_id: Option<String>,
    input_mode: RecordingInputMode,
) -> Result<RecordingPayload, String> {
    if recorder
        .session
        .lock()
        .map_err(|_| "Recorder state lock was poisoned".to_string())?
        .is_some()
    {
        return Err("Recording is already active".to_string());
    }

    paths.ensure()?;
    let thread = select_recording_thread(&paths, requested_thread_id)?;
    let thread_id = thread.summary.id.clone();
    let thread_dir = paths.thread_dir(&thread_id);
    prepare_work_dir(&thread_dir)?;

    let started = Instant::now();
    let PreparedAudioInput {
        mic_sample_rate,
        system_sample_rate,
        buffers,
        mut audio_capture,
    } = prepare_audio_input(&app, input_mode)?;

    set_thread_status(&thread_dir, ThreadStatus::Recording)?;

    start_audio_capture(&mut audio_capture)?;

    let should_stop_meter = Arc::new(AtomicBool::new(false));
    let meter_thread = spawn_meter_thread(
        app.clone(),
        thread_id.clone(),
        Arc::clone(&buffers),
        Arc::clone(&should_stop_meter),
        started,
    );

    let transcription = transcription_status(&paths);
    let should_stop_live_transcription = Arc::new(AtomicBool::new(false));
    let live_transcription_thread = spawn_live_transcription_thread(
        app,
        paths.transcription_paths(),
        Arc::clone(&buffers),
        Arc::clone(&should_stop_live_transcription),
        thread_id.clone(),
        thread_dir.clone(),
        mic_sample_rate,
        system_sample_rate,
    );

    let mut slot = recorder
        .session
        .lock()
        .map_err(|_| "Recorder state lock was poisoned".to_string())?;
    if slot.is_some() {
        return Err("Recording is already active".to_string());
    }

    *slot = Some(RecorderSession {
        thread_id: thread_id.clone(),
        thread_dir,
        started,
        buffers,
        should_stop_meter,
        should_stop_live_transcription,
        meter_thread: Some(meter_thread),
        live_transcription_thread,
        audio_capture,
    });

    Ok(RecordingPayload {
        thread: load_thread_by_id(&paths, &thread_id)?,
        transcription,
    })
}

fn select_recording_thread(
    paths: &AppPaths,
    requested_thread_id: Option<String>,
) -> Result<ThreadDetail, String> {
    let Some(thread_id) = requested_thread_id else {
        return create_thread_inner(paths);
    };

    let thread = load_thread_by_id(paths, &thread_id)?;
    if thread.summary.status == ThreadStatus::Recording {
        return Err("The selected thread is already recording".to_string());
    }
    if thread.summary.segment_count > 0 {
        return create_thread_inner(paths);
    }

    Ok(thread)
}

fn stop_recording_inner(paths: AppPaths, recorder: RecorderState) -> Result<ThreadDetail, String> {
    if recorder.is_starting.load(Ordering::SeqCst) {
        return Err("Audio startup is still in progress".to_string());
    }

    let session = {
        let mut slot = recorder
            .session
            .lock()
            .map_err(|_| "Recorder state lock was poisoned".to_string())?;
        slot.take()
            .ok_or_else(|| "No recording is currently active".to_string())?
    };

    let RecorderSession {
        thread_id,
        thread_dir,
        started,
        buffers,
        should_stop_meter,
        should_stop_live_transcription,
        mut meter_thread,
        mut live_transcription_thread,
        audio_capture,
    } = session;

    should_stop_meter.store(true, Ordering::Relaxed);
    should_stop_live_transcription.store(true, Ordering::Relaxed);
    if let Some(thread) = meter_thread.take() {
        let _ = thread.join();
    }
    stop_audio_capture(audio_capture);
    if let Some(thread) = live_transcription_thread.take() {
        let _ = thread.join();
    }
    drop(buffers);

    let duration_ms = started.elapsed().as_millis() as u64;
    set_thread_status(&thread_dir, ThreadStatus::Idle)?;
    render_thread_markdown(&thread_dir, duration_ms)?;
    load_thread_by_id(&paths, &thread_id)
}

fn prepare_work_dir(thread_dir: &Path) -> Result<(), String> {
    let work_dir = thread_dir.join("work");
    if work_dir.exists() {
        fs::remove_dir_all(&work_dir).map_err(|err| {
            format!(
                "Failed to clear transcription work directory {}: {err}",
                work_dir.display()
            )
        })?;
    }
    fs::create_dir_all(&work_dir).map_err(|err| {
        format!(
            "Failed to create transcription work directory {}: {err}",
            work_dir.display()
        )
    })
}

fn render_thread_markdown(thread_dir: &Path, duration_ms: u64) -> Result<(), String> {
    let metadata = read_thread_metadata(&thread_dir.join("thread.json"))?;
    let segments = read_transcript_jsonl(&thread_dir.join("transcript.jsonl"))?;
    let mut markdown = String::new();
    markdown.push_str(&format!("# {}\n\n", metadata.title));
    markdown.push_str(&format!("Thread: `{}`\n\n", metadata.id));
    markdown.push_str(&format!(
        "Duration: `{}`\n\n",
        format_transcript_time(duration_ms)
    ));

    for segment in segments {
        markdown.push_str(&format!(
            "[{}] **{}:** {}\n\n",
            format_transcript_time(segment.start_ms),
            segment.speaker,
            segment.text
        ));
    }

    write_text_atomic(&thread_dir.join("transcript.md"), &markdown)
}

fn transcription_status(paths: &AppPaths) -> TranscriptionStatusPayload {
    let transcription_paths = paths.transcription_paths();
    let engine_exists = true;
    let model_exists = transcription_paths.model_path.is_file();
    let ready = model_exists;
    let message = match model_exists {
        true => format!(
            "Local transcription is ready ({})",
            transcription_paths.model_name
        ),
        false => "Local transcription model is missing".to_string(),
    };

    TranscriptionStatusPayload {
        ready,
        engine_exists,
        model_exists,
        engine_path: "embedded whisper.cpp runtime".to_string(),
        model_path: transcription_paths.model_path.display().to_string(),
        model_name: transcription_paths.model_name,
        available_models: transcription_paths.available_models,
        message,
    }
}

struct PreparedSystemLoopbackDevice {
    device: Device,
    tap: CoreAudioSystemTap,
}

struct SystemAudioCapture {
    stream: Option<Stream>,
    tap: Option<CoreAudioSystemTap>,
}

impl SystemAudioCapture {
    fn new(stream: Stream, tap: CoreAudioSystemTap) -> Self {
        Self {
            stream: Some(stream),
            tap: Some(tap),
        }
    }

    fn stream(&self) -> &Stream {
        self.stream
            .as_ref()
            .expect("system capture stream should exist until drop")
    }
}

impl Drop for SystemAudioCapture {
    fn drop(&mut self) {
        self.stream.take();
        self.tap.take();
    }
}

struct CoreAudioSystemTap {
    tap_id: AudioObjectID,
    aggregate_device_id: AudioObjectID,
}

impl CoreAudioSystemTap {
    fn create() -> Result<Self, String> {
        let empty_processes = NSArray::<NSNumber>::new();
        let description = unsafe {
            CATapDescription::initMonoGlobalTapButExcludeProcesses(
                CATapDescription::alloc(),
                &empty_processes,
            )
        };
        unsafe {
            description.setName(&NSString::from_str(SYSTEM_CAPTURE_DEVICE_NAME));
            description.setPrivate(true);
            description.setMuteBehavior(CATapMuteBehavior::Unmuted);
        }

        let mut tap_id = 0;
        let status = unsafe { AudioHardwareCreateProcessTap(Some(&description), &mut tap_id) };
        if status != kAudioHardwareNoError {
            return Err(format!(
                "Failed to create macOS system audio tap: {}",
                coreaudio_status(status)
            ));
        }

        let aggregate_uid = format!("com.just-notes.system-audio.{}", now_ms()?);
        let aggregate_description =
            create_aggregate_device_description(SYSTEM_CAPTURE_DEVICE_NAME, &aggregate_uid)?;

        let mut aggregate_device_id = 0;
        let status = unsafe {
            AudioHardwareCreateAggregateDevice(
                aggregate_description.as_ref(),
                NonNull::from(&mut aggregate_device_id),
            )
        };
        if status != kAudioHardwareNoError {
            unsafe {
                AudioHardwareDestroyProcessTap(tap_id);
            }
            return Err(format!(
                "Failed to create macOS system audio aggregate device: {}",
                coreaudio_status(status)
            ));
        }

        if let Err(err) = attach_tap_to_aggregate_device(tap_id, aggregate_device_id) {
            unsafe {
                AudioHardwareDestroyAggregateDevice(aggregate_device_id);
                AudioHardwareDestroyProcessTap(tap_id);
            }
            return Err(err);
        }

        Ok(Self {
            tap_id,
            aggregate_device_id,
        })
    }
}

impl Drop for CoreAudioSystemTap {
    fn drop(&mut self) {
        unsafe {
            AudioHardwareDestroyAggregateDevice(self.aggregate_device_id);
            AudioHardwareDestroyProcessTap(self.tap_id);
        }
    }
}

fn create_aggregate_device_description(
    device_name: &str,
    aggregate_uid: &str,
) -> Result<objc2_core_foundation::CFRetained<CFDictionary<CFType, CFType>>, String> {
    let name_key = cf_audio_key(kAudioAggregateDeviceNameKey)?;
    let uid_key = cf_audio_key(kAudioAggregateDeviceUIDKey)?;
    let private_key = cf_audio_key(kAudioAggregateDeviceIsPrivateKey)?;
    let name = CFString::from_str(device_name);
    let uid = CFString::from_str(aggregate_uid);
    let private = CFBoolean::new(true);

    Ok(CFDictionary::<CFType, CFType>::from_slices(
        &[name_key.as_ref(), uid_key.as_ref(), private_key.as_ref()],
        &[name.as_ref(), uid.as_ref(), private.as_ref()],
    ))
}

fn attach_tap_to_aggregate_device(
    tap_id: AudioObjectID,
    aggregate_device_id: AudioObjectID,
) -> Result<(), String> {
    let mut tap_uid_ref: *const CFString = ptr::null();
    let mut property_size = mem::size_of::<*const CFString>() as u32;
    let mut property_address = audio_property_address(kAudioTapPropertyUID);
    let status = unsafe {
        AudioObjectGetPropertyData(
            tap_id,
            NonNull::from(&mut property_address),
            0,
            ptr::null(),
            NonNull::from(&mut property_size),
            NonNull::new((&mut tap_uid_ref as *mut *const CFString).cast::<c_void>())
                .expect("tap UID output pointer cannot be null"),
        )
    };
    if status != kAudioHardwareNoError {
        return Err(format!(
            "Failed to read macOS system audio tap UID: {}",
            coreaudio_status(status)
        ));
    }

    let tap_uid = unsafe {
        tap_uid_ref
            .as_ref()
            .ok_or_else(|| "CoreAudio returned an empty system audio tap UID".to_string())?
    };
    let tap_list = CFArray::<CFString>::from_objects(&[tap_uid]);
    let mut tap_list_ref: *const CFArray<CFString> = tap_list.as_ref();
    let mut property_address = audio_property_address(kAudioAggregateDevicePropertyTapList);
    let property_size = mem::size_of::<*const CFArray<CFString>>() as u32;
    let status = unsafe {
        AudioObjectSetPropertyData(
            aggregate_device_id,
            NonNull::from(&mut property_address),
            0,
            ptr::null(),
            property_size,
            NonNull::new((&mut tap_list_ref as *mut *const CFArray<CFString>).cast::<c_void>())
                .expect("tap list pointer cannot be null"),
        )
    };
    if status != kAudioHardwareNoError {
        return Err(format!(
            "Failed to attach macOS system audio tap to aggregate device: {}",
            coreaudio_status(status)
        ));
    }

    Ok(())
}

fn audio_property_address(selector: u32) -> AudioObjectPropertyAddress {
    AudioObjectPropertyAddress {
        mSelector: selector,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain,
    }
}

fn cf_audio_key(key: &CStr) -> Result<objc2_core_foundation::CFRetained<CFString>, String> {
    let key = key
        .to_str()
        .map_err(|err| format!("Invalid CoreAudio dictionary key: {err}"))?;
    Ok(CFString::from_str(key))
}

fn prepare_system_loopback_device(
    host: &cpal::Host,
) -> Result<PreparedSystemLoopbackDevice, String> {
    let tap = CoreAudioSystemTap::create()?;

    for _ in 0..20 {
        if let Some(device) = find_system_loopback_device(host)? {
            return Ok(PreparedSystemLoopbackDevice { device, tap });
        }
        thread::sleep(Duration::from_millis(100));
    }

    Err(format!(
        "Created macOS system audio tap, but CPAL did not expose the {SYSTEM_CAPTURE_DEVICE_NAME} input device"
    ))
}

fn find_system_loopback_device(host: &cpal::Host) -> Result<Option<Device>, String> {
    let devices = host
        .input_devices()
        .map_err(|err| format!("Failed to enumerate audio input devices: {err}"))?;
    for device in devices {
        let Ok(description) = device.description() else {
            continue;
        };
        if description.name() == SYSTEM_CAPTURE_DEVICE_NAME {
            return Ok(Some(device));
        }
    }
    Ok(None)
}

fn coreaudio_status(status: i32) -> String {
    let bytes = status.to_be_bytes();
    if bytes
        .iter()
        .all(|byte| byte.is_ascii_graphic() || *byte == b' ')
    {
        format!("{status} ('{}')", String::from_utf8_lossy(&bytes))
    } else {
        status.to_string()
    }
}

#[derive(Clone, Copy)]
enum CaptureSource {
    Mic,
    System,
}

fn prepare_audio_input(
    app: &AppHandle,
    input_mode: RecordingInputMode,
) -> Result<PreparedAudioInput, String> {
    match input_mode {
        RecordingInputMode::Devices => prepare_device_audio_input(app),
        #[cfg(any(debug_assertions, feature = "qa-fixtures"))]
        RecordingInputMode::Fixture {
            mic_path,
            system_path,
        } => prepare_fixture_audio_input(mic_path, system_path),
    }
}

fn prepare_device_audio_input(app: &AppHandle) -> Result<PreparedAudioInput, String> {
    let host = cpal::default_host();
    ensure_microphone_permission(app)?;

    let mic_device = host
        .default_input_device()
        .ok_or_else(|| "No default microphone input device is available".to_string())?;
    let system_device = prepare_system_loopback_device(&host)?;

    let mic_config = mic_device
        .default_input_config()
        .map_err(|err| format!("Failed to read microphone config: {err}"))?;
    let system_config = system_device
        .device
        .default_input_config()
        .map_err(|err| format!("Failed to read system loopback config: {err}"))?;

    let mic_sample_rate = mic_config.sample_rate();
    let system_sample_rate = system_config.sample_rate();
    let buffers = Arc::new(Mutex::new(SharedBuffers::new(
        mic_sample_rate,
        system_sample_rate,
    )));

    let mic_stream = build_capture_stream(
        mic_device,
        mic_config,
        Arc::clone(&buffers),
        CaptureSource::Mic,
    )?;
    let system_stream = build_capture_stream(
        system_device.device,
        system_config,
        Arc::clone(&buffers),
        CaptureSource::System,
    )?;
    let system_capture = SystemAudioCapture::new(system_stream, system_device.tap);

    Ok(PreparedAudioInput {
        mic_sample_rate,
        system_sample_rate,
        buffers,
        audio_capture: ActiveAudioCapture::Devices {
            _mic_stream: mic_stream,
            _system_capture: system_capture,
        },
    })
}

fn start_audio_capture(audio_capture: &mut ActiveAudioCapture) -> Result<(), String> {
    match audio_capture {
        ActiveAudioCapture::Devices {
            _mic_stream,
            _system_capture,
        } => {
            _mic_stream
                .play()
                .map_err(|err| format!("Failed to start microphone stream: {err}"))?;
            _system_capture
                .stream()
                .play()
                .map_err(|err| format!("Failed to start system loopback stream: {err}"))
        }
        #[cfg(any(debug_assertions, feature = "qa-fixtures"))]
        ActiveAudioCapture::Fixture(_) => Ok(()),
    }
}

fn stop_audio_capture(audio_capture: ActiveAudioCapture) {
    match audio_capture {
        ActiveAudioCapture::Devices {
            _mic_stream,
            _system_capture,
        } => {
            drop(_mic_stream);
            drop(_system_capture);
        }
        #[cfg(any(debug_assertions, feature = "qa-fixtures"))]
        ActiveAudioCapture::Fixture(fixture) => {
            fixture.should_stop.store(true, Ordering::Relaxed);
            for worker in fixture.workers {
                let _ = worker.join();
            }
        }
    }
}

#[cfg(any(debug_assertions, feature = "qa-fixtures"))]
struct FixtureTrack {
    sample_rate: u32,
    samples: Vec<f32>,
}

#[cfg(any(debug_assertions, feature = "qa-fixtures"))]
fn prepare_fixture_audio_input(
    mic_path: PathBuf,
    system_path: PathBuf,
) -> Result<PreparedAudioInput, String> {
    let mic_track = read_fixture_track(&mic_path)?;
    let system_track = read_fixture_track(&system_path)?;
    let buffers = Arc::new(Mutex::new(SharedBuffers::new(
        mic_track.sample_rate,
        system_track.sample_rate,
    )));
    let should_stop = Arc::new(AtomicBool::new(false));
    let workers = vec![
        spawn_fixture_audio_worker(
            mic_track,
            Arc::clone(&buffers),
            CaptureSource::Mic,
            Arc::clone(&should_stop),
        ),
        spawn_fixture_audio_worker(
            system_track,
            Arc::clone(&buffers),
            CaptureSource::System,
            Arc::clone(&should_stop),
        ),
    ];

    Ok(PreparedAudioInput {
        mic_sample_rate: read_wav_sample_rate(&mic_path)?,
        system_sample_rate: read_wav_sample_rate(&system_path)?,
        buffers,
        audio_capture: ActiveAudioCapture::Fixture(FixtureAudioCapture {
            should_stop,
            workers,
        }),
    })
}

#[cfg(any(debug_assertions, feature = "qa-fixtures"))]
fn read_wav_sample_rate(path: &Path) -> Result<u32, String> {
    let reader = hound::WavReader::open(path)
        .map_err(|err| format!("Failed to open fixture audio {}: {err}", path.display()))?;
    Ok(reader.spec().sample_rate)
}

#[cfg(any(debug_assertions, feature = "qa-fixtures"))]
fn read_fixture_track(path: &Path) -> Result<FixtureTrack, String> {
    let mut reader = hound::WavReader::open(path).map_err(|err| {
        format!(
            "Failed to open fixture audio {}. Create qa-mic.wav and qa-system.wav in ~/.just-notes/fixtures: {err}",
            path.display()
        )
    })?;
    let spec = reader.spec();
    if spec.channels == 0 {
        return Err(format!("Fixture audio {} has no channels", path.display()));
    }

    let channels = spec.channels as usize;
    let raw_samples = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .map(|sample| {
                sample.map(|value| value.clamp(-1.0, 1.0)).map_err(|err| {
                    format!("Failed to read fixture audio {}: {err}", path.display())
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
        hound::SampleFormat::Int if spec.bits_per_sample <= 16 => reader
            .samples::<i16>()
            .map(|sample| {
                sample
                    .map(|value| value as f32 / i16::MAX as f32)
                    .map_err(|err| {
                        format!("Failed to read fixture audio {}: {err}", path.display())
                    })
            })
            .collect::<Result<Vec<_>, _>>()?,
        hound::SampleFormat::Int => {
            let denom = ((1_i64 << (spec.bits_per_sample.saturating_sub(1) as u32)) - 1) as f32;
            reader
                .samples::<i32>()
                .map(|sample| {
                    sample
                        .map(|value| (value as f32 / denom).clamp(-1.0, 1.0))
                        .map_err(|err| {
                            format!("Failed to read fixture audio {}: {err}", path.display())
                        })
                })
                .collect::<Result<Vec<_>, _>>()?
        }
    };

    let samples = raw_samples
        .chunks(channels)
        .map(average_f32)
        .collect::<Vec<_>>();
    Ok(FixtureTrack {
        sample_rate: spec.sample_rate,
        samples,
    })
}

#[cfg(any(debug_assertions, feature = "qa-fixtures"))]
fn spawn_fixture_audio_worker(
    track: FixtureTrack,
    buffers: Arc<Mutex<SharedBuffers>>,
    source: CaptureSource,
    should_stop: Arc<AtomicBool>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let chunk_samples = ((track.sample_rate as u64 * FIXTURE_CHUNK_MS) / 1000).max(1) as usize;
        for chunk in track.samples.chunks(chunk_samples) {
            if should_stop.load(Ordering::Relaxed) {
                break;
            }
            push_mono_frames(chunk.iter().copied(), &buffers, source);
            let chunk_ms = samples_to_ms(chunk.len() as u64, track.sample_rate);
            thread::sleep(Duration::from_millis(chunk_ms.max(1)));
        }
    })
}

fn build_capture_stream(
    device: Device,
    supported_config: SupportedStreamConfig,
    buffers: Arc<Mutex<SharedBuffers>>,
    source: CaptureSource,
) -> Result<Stream, String> {
    let sample_format = supported_config.sample_format();
    let config: StreamConfig = supported_config.into();

    let err_source = match source {
        CaptureSource::Mic => "microphone",
        CaptureSource::System => "system loopback",
    };
    let err_fn = move |err| eprintln!("{err_source} stream error: {err}");

    match sample_format {
        SampleFormat::F32 => device.build_input_stream(
            &config,
            move |data: &[f32], _| push_f32_samples(data, config.channels, &buffers, source),
            err_fn,
            None,
        ),
        SampleFormat::I16 => device.build_input_stream(
            &config,
            move |data: &[i16], _| push_i16_samples(data, config.channels, &buffers, source),
            err_fn,
            None,
        ),
        SampleFormat::U16 => device.build_input_stream(
            &config,
            move |data: &[u16], _| push_u16_samples(data, config.channels, &buffers, source),
            err_fn,
            None,
        ),
        other => {
            return Err(format!(
                "Unsupported {err_source} sample format: {other:?}. Expected f32, i16, or u16."
            ))
        }
    }
    .map_err(|err| format!("Failed to build {err_source} input stream: {err}"))
}

fn push_f32_samples(
    samples: &[f32],
    channels: u16,
    buffers: &Arc<Mutex<SharedBuffers>>,
    source: CaptureSource,
) {
    push_mono_frames(
        samples.chunks(channels as usize).map(average_f32),
        buffers,
        source,
    );
}

fn push_i16_samples(
    samples: &[i16],
    channels: u16,
    buffers: &Arc<Mutex<SharedBuffers>>,
    source: CaptureSource,
) {
    push_mono_frames(
        samples.chunks(channels as usize).map(|frame| {
            frame
                .iter()
                .map(|sample| *sample as f32 / i16::MAX as f32)
                .sum::<f32>()
                / frame.len() as f32
        }),
        buffers,
        source,
    );
}

fn push_u16_samples(
    samples: &[u16],
    channels: u16,
    buffers: &Arc<Mutex<SharedBuffers>>,
    source: CaptureSource,
) {
    push_mono_frames(
        samples.chunks(channels as usize).map(|frame| {
            frame
                .iter()
                .map(|sample| (*sample as f32 - 32768.0) / 32768.0)
                .sum::<f32>()
                / frame.len() as f32
        }),
        buffers,
        source,
    );
}

fn average_f32(frame: &[f32]) -> f32 {
    frame.iter().copied().sum::<f32>() / frame.len() as f32
}

fn push_mono_frames<I>(frames: I, buffers: &Arc<Mutex<SharedBuffers>>, source: CaptureSource)
where
    I: Iterator<Item = f32>,
{
    let mut chunk = Vec::new();
    let mut square_sum = 0.0;

    for sample in frames {
        let clamped = sample.clamp(-1.0, 1.0);
        square_sum += clamped * clamped;
        chunk.push(clamped);
    }

    if chunk.is_empty() {
        return;
    }

    let rms = (square_sum / chunk.len() as f32).sqrt();
    let level = (rms * 4.0).min(1.0);

    if let Ok(mut shared) = buffers.lock() {
        match source {
            CaptureSource::Mic => shared.mic.push(&chunk, level),
            CaptureSource::System => shared.system.push(&chunk, level),
        }
    }
}

fn smooth_level(current: f32, next: f32) -> f32 {
    (current * 0.72) + (next * 0.28)
}

fn spawn_meter_thread(
    app: AppHandle,
    thread_id: String,
    buffers: Arc<Mutex<SharedBuffers>>,
    should_stop: Arc<AtomicBool>,
    started: Instant,
) -> JoinHandle<()> {
    thread::spawn(move || {
        while !should_stop.load(Ordering::Relaxed) {
            if let Ok(shared) = buffers.lock() {
                let _ = app.emit(
                    "meter-update",
                    MeterPayload {
                        thread_id: thread_id.clone(),
                        mic_level: shared.mic.level,
                        system_level: shared.system.level,
                        elapsed_ms: started.elapsed().as_millis() as u64,
                    },
                );
            }

            thread::sleep(Duration::from_millis(100));
        }
    })
}

fn spawn_live_transcription_thread(
    app: AppHandle,
    paths: TranscriptionPaths,
    buffers: Arc<Mutex<SharedBuffers>>,
    should_stop: Arc<AtomicBool>,
    thread_id: String,
    thread_dir: PathBuf,
    mic_sample_rate: u32,
    system_sample_rate: u32,
) -> Option<JoinHandle<()>> {
    if !paths.model_path.is_file() {
        emit_live_status(
            &app,
            &thread_id,
            false,
            "Live transcription is disabled until the local model is installed",
        );
        return None;
    }

    Some(thread::spawn(move || {
        if let Err(err) = run_live_transcription_loop(
            app.clone(),
            paths,
            buffers,
            should_stop,
            thread_id.clone(),
            thread_dir.clone(),
            mic_sample_rate,
            system_sample_rate,
        ) {
            let _ = app.emit(
                "live-transcript-error",
                LiveTranscriptStatusPayload {
                    thread_id,
                    active: false,
                    message: err,
                    chunk_ms: LIVE_TRANSCRIPTION_WINDOW_MS,
                    overlap_ms: LIVE_TRANSCRIPTION_STABILITY_DELAY_MS,
                },
            );
        }
    }))
}

fn run_live_transcription_loop(
    app: AppHandle,
    paths: TranscriptionPaths,
    buffers: Arc<Mutex<SharedBuffers>>,
    should_stop: Arc<AtomicBool>,
    thread_id: String,
    thread_dir: PathBuf,
    mic_sample_rate: u32,
    system_sample_rate: u32,
) -> Result<(), String> {
    let whisper = WhisperRuntime::load(&paths.model_path)?;
    emit_live_status(
        &app,
        &thread_id,
        true,
        format!("Live transcription is listening with {}", paths.model_name),
    );

    let jsonl_path = thread_dir.join("transcript.jsonl");
    let mut mic = LiveChannelState::new("mic", "You", mic_sample_rate);
    let mut system = LiveChannelState::new("system", "Others", system_sample_rate);
    let mut emitted_count = 0usize;

    loop {
        let stopping = should_stop.load(Ordering::Relaxed);
        emitted_count += process_live_channel(
            &app,
            &whisper,
            &buffers,
            &jsonl_path,
            &thread_dir,
            &thread_id,
            &mut mic,
            stopping,
        )?;
        emitted_count += process_live_channel(
            &app,
            &whisper,
            &buffers,
            &jsonl_path,
            &thread_dir,
            &thread_id,
            &mut system,
            stopping,
        )?;

        if stopping {
            break;
        }

        thread::sleep(Duration::from_millis(LIVE_TRANSCRIPTION_POLL_MS));
    }

    emit_live_status(
        &app,
        &thread_id,
        false,
        format!("Live transcription stopped after {emitted_count} committed segments"),
    );
    Ok(())
}

struct LiveChannelState {
    source: &'static str,
    speaker: &'static str,
    sample_rate: u32,
    next_decode_ms: u64,
    decode_start_ms: u64,
    committed_until_ms: u64,
    last_emitted_end_ms: u64,
    prompt_tail: VecDeque<String>,
    emitted_text_tail: VecDeque<String>,
    previous_hypothesis: Option<String>,
}

impl LiveChannelState {
    fn new(source: &'static str, speaker: &'static str, sample_rate: u32) -> Self {
        Self {
            source,
            speaker,
            sample_rate,
            next_decode_ms: LIVE_TRANSCRIPTION_STEP_MS,
            decode_start_ms: 0,
            committed_until_ms: 0,
            last_emitted_end_ms: 0,
            prompt_tail: VecDeque::with_capacity(8),
            emitted_text_tail: VecDeque::with_capacity(LIVE_DUPLICATE_RECENT_SEGMENTS),
            previous_hypothesis: None,
        }
    }

    fn prompt(&self) -> String {
        self.prompt_tail
            .iter()
            .rev()
            .take(4)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn remember_prompt_text(&mut self, text: &str) {
        self.prompt_tail.push_back(text.to_string());
        while self.prompt_tail.len() > 8 {
            self.prompt_tail.pop_front();
        }
    }

    fn unique_text(&self, text: &str) -> Option<String> {
        unique_transcript_text(text, &self.emitted_text_tail)
    }

    fn remember_emitted_text(&mut self, text: &str) {
        self.emitted_text_tail.push_back(text.to_string());
        while self.emitted_text_tail.len() > LIVE_DUPLICATE_RECENT_SEGMENTS {
            self.emitted_text_tail.pop_front();
        }
    }

    fn agreed_text(&mut self, text: &str, final_flush: bool) -> Option<String> {
        let hypothesis = text.trim();
        if hypothesis.is_empty() {
            self.previous_hypothesis = None;
            return None;
        }

        let agreed = if final_flush {
            Some(hypothesis.to_string())
        } else {
            self.previous_hypothesis
                .as_deref()
                .and_then(|previous| common_transcript_prefix(previous, hypothesis))
        };
        self.previous_hypothesis = Some(hypothesis.to_string());
        agreed
    }

    fn mark_emitted_until(&mut self, end_ms: u64) {
        self.last_emitted_end_ms = self.last_emitted_end_ms.max(end_ms);
        self.decode_start_ms = self.decode_start_ms.max(end_ms);
        self.previous_hypothesis = None;
    }
}

struct WhisperRuntime {
    ctx: WhisperContext,
}

impl WhisperRuntime {
    fn load(model_path: &Path) -> Result<Self, String> {
        let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
            .map_err(|err| {
                format!(
                    "Failed to load local transcription model {}: {err}",
                    model_path.display()
                )
            })?;
        Ok(Self { ctx })
    }

    fn transcribe(
        &self,
        samples_16k: &[f32],
        prompt: &str,
        source: &str,
        speaker: &str,
    ) -> Result<Vec<TranscriptSegment>, String> {
        let mut state = self
            .ctx
            .create_state()
            .map_err(|err| format!("Failed to create transcription state: {err}"))?;
        let mut params = FullParams::new(SamplingStrategy::BeamSearch {
            beam_size: 5,
            patience: -1.0,
        });
        params.set_language(Some("en"));
        params.set_n_threads(default_thread_count() as i32);
        params.set_no_context(true);
        params.set_single_segment(false);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        let _ = prompt;

        state
            .full(params, samples_16k)
            .map_err(|err| format!("Failed to transcribe {source} audio: {err}"))?;

        let mut segments = Vec::new();
        for segment in state.as_iter() {
            let text = clean_transcript_text(&segment.to_string());
            if text.is_empty() || is_ignored_transcript_text(&text) {
                continue;
            }
            let start_ms = (segment.start_timestamp().max(0) as u64) * 10;
            let end_ms = (segment
                .end_timestamp()
                .max(segment.start_timestamp())
                .max(0) as u64)
                * 10;
            segments.push(TranscriptSegment {
                speaker: speaker.to_string(),
                source: source.to_string(),
                start_ms,
                end_ms,
                text,
            });
        }
        Ok(segments)
    }
}

#[allow(clippy::too_many_arguments)]
fn process_live_channel(
    app: &AppHandle,
    whisper: &WhisperRuntime,
    buffers: &Arc<Mutex<SharedBuffers>>,
    jsonl_path: &Path,
    thread_dir: &Path,
    thread_id: &str,
    state: &mut LiveChannelState,
    final_flush: bool,
) -> Result<usize, String> {
    let available_samples = {
        let shared = buffers
            .lock()
            .map_err(|_| "Audio buffer lock was poisoned".to_string())?;
        match state.source {
            "mic" => shared.mic.available_end_index(),
            "system" => shared.system.available_end_index(),
            _ => 0,
        }
    };
    let available_ms = samples_to_ms(available_samples, state.sample_rate);
    if available_ms <= state.committed_until_ms {
        return Ok(0);
    }

    let target_end_ms = if final_flush {
        available_ms
    } else if available_ms >= state.next_decode_ms {
        state.next_decode_ms
    } else {
        return Ok(0);
    };
    let commit_until_ms = if final_flush {
        target_end_ms
    } else {
        target_end_ms.saturating_sub(LIVE_TRANSCRIPTION_STABILITY_DELAY_MS)
    };
    if commit_until_ms <= state.committed_until_ms {
        state.next_decode_ms += LIVE_TRANSCRIPTION_STEP_MS;
        return Ok(0);
    }

    let max_window_start_ms =
        target_end_ms.saturating_sub(LIVE_TRANSCRIPTION_MAX_AGREEMENT_BUFFER_MS);
    if state.decode_start_ms < max_window_start_ms {
        state.decode_start_ms = max_window_start_ms;
        state.previous_hypothesis = None;
    }
    let window_start_ms = state.decode_start_ms;
    let start_index = ms_to_samples(window_start_ms, state.sample_rate) as u64;
    let end_index = ms_to_samples(target_end_ms, state.sample_rate) as u64;

    let samples = {
        let shared = buffers
            .lock()
            .map_err(|_| "Audio buffer lock was poisoned".to_string())?;
        let channel = match state.source {
            "mic" => &shared.mic,
            "system" => &shared.system,
            _ => return Ok(0),
        };
        channel.window(start_index, end_index)
    };

    let Some(samples) = samples else {
        state.committed_until_ms = commit_until_ms;
        state.next_decode_ms = target_end_ms + LIVE_TRANSCRIPTION_STEP_MS;
        return Ok(0);
    };

    if rms(&samples) < LIVE_SILENCE_RMS_THRESHOLD {
        state.committed_until_ms = commit_until_ms;
        state.next_decode_ms = target_end_ms + LIVE_TRANSCRIPTION_STEP_MS;
        return Ok(0);
    }

    let normalized_samples = resample_to_rate(&samples, state.sample_rate, 16_000);
    let mut segments = whisper.transcribe(
        &normalized_samples,
        &state.prompt(),
        state.source,
        state.speaker,
    )?;
    segments.sort_by_key(|segment| segment.start_ms);

    let decoded_text = segments
        .iter()
        .map(|segment| segment.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let Some(agreed_text) = state.agreed_text(&decoded_text, final_flush) else {
        state.committed_until_ms = commit_until_ms;
        state.next_decode_ms = target_end_ms + LIVE_TRANSCRIPTION_STEP_MS;
        return Ok(0);
    };

    let completed_agreed_text = completed_transcript_text(&agreed_text, final_flush);
    if completed_agreed_text.is_empty() {
        state.committed_until_ms = commit_until_ms;
        state.next_decode_ms = target_end_ms + LIVE_TRANSCRIPTION_STEP_MS;
        return Ok(0);
    }
    let stable_end_ms = estimate_text_end_ms(
        &segments,
        &completed_agreed_text,
        window_start_ms,
        commit_until_ms,
    );

    let emitted = if let Some(unique_text) = state.unique_text(&completed_agreed_text) {
        let unique_text = unique_text.trim().to_string();
        if unique_text.is_empty() {
            state.mark_emitted_until(stable_end_ms);
            state.committed_until_ms = commit_until_ms;
            state.next_decode_ms = target_end_ms + LIVE_TRANSCRIPTION_STEP_MS;
            return Ok(0);
        }

        let segment = TranscriptSegment {
            speaker: state.speaker.to_string(),
            source: state.source.to_string(),
            start_ms: state.last_emitted_end_ms.max(window_start_ms),
            end_ms: stable_end_ms,
            text: unique_text,
        };

        if segment.end_ms > segment.start_ms {
            append_live_segment(jsonl_path, &segment)?;
            touch_thread(thread_dir)?;
            state.remember_prompt_text(&segment.text);
            state.remember_emitted_text(&segment.text);
            let _ = app.emit(
                "live-transcript-segment",
                LiveTranscriptSegmentPayload {
                    thread_id: thread_id.to_string(),
                    committed_until_ms: commit_until_ms,
                    segment,
                },
            );
            state.mark_emitted_until(stable_end_ms);
            1
        } else {
            0
        }
    } else {
        state.mark_emitted_until(stable_end_ms);
        0
    };

    state.committed_until_ms = commit_until_ms;
    state.next_decode_ms = target_end_ms + LIVE_TRANSCRIPTION_STEP_MS;
    Ok(emitted)
}

fn is_ignored_transcript_text(text: &str) -> bool {
    matches!(
        text.trim().to_ascii_lowercase().as_str(),
        "[blank_audio]" | "[silence]" | "(silence)" | "[music]" | "(music)"
    )
}

fn clean_transcript_text(text: &str) -> String {
    let mut cleaned = text.trim();
    loop {
        let lower = cleaned.to_ascii_lowercase();
        let Some(prefix) = [
            "[blank_audio]",
            "[silence]",
            "(silence)",
            "[music]",
            "(music)",
            "(no audio)",
        ]
        .iter()
        .find(|prefix| lower.starts_with(**prefix)) else {
            break;
        };
        cleaned = cleaned[prefix.len()..].trim();
    }
    cleaned.to_string()
}

fn completed_transcript_text(text: &str, include_partial: bool) -> String {
    let text = text.trim();
    if include_partial || transcript_text_is_complete(text) {
        return text.to_string();
    }

    split_transcript_sentences(text)
        .into_iter()
        .filter(|sentence| transcript_text_is_complete(sentence))
        .collect::<Vec<_>>()
        .join(" ")
}

fn transcript_text_is_complete(text: &str) -> bool {
    text.trim()
        .chars()
        .next_back()
        .map(|character| matches!(character, '.' | '!' | '?'))
        .unwrap_or(false)
}

fn common_transcript_prefix(left: &str, right: &str) -> Option<String> {
    let left_words = normalized_words(left);
    let right_words = transcript_words(right);
    let prefix_len = left_words
        .iter()
        .zip(right_words.iter())
        .take_while(|(left, right)| left.as_str() == right.normalized.as_str())
        .count();

    (prefix_len > 0).then(|| join_original_words(&right_words[..prefix_len]))
}

fn estimate_text_end_ms(
    segments: &[TranscriptSegment],
    text: &str,
    window_start_ms: u64,
    fallback_end_ms: u64,
) -> u64 {
    let target_words = normalized_words(text).len();
    if target_words == 0 {
        return window_start_ms;
    }

    let mut seen_words = 0usize;
    for segment in segments {
        let segment_words = normalized_words(&segment.text).len();
        if segment_words == 0 {
            continue;
        }

        let next_seen_words = seen_words + segment_words;
        let segment_start_ms = window_start_ms + segment.start_ms;
        let segment_end_ms = window_start_ms + segment.end_ms;
        if target_words <= next_seen_words {
            let words_in_segment = target_words.saturating_sub(seen_words).max(1);
            let ratio = (words_in_segment as f64 / segment_words as f64).clamp(0.0, 1.0);
            let duration_ms = segment_end_ms.saturating_sub(segment_start_ms) as f64;
            let estimated_ms = (segment_start_ms as f64 + duration_ms * ratio)
                .round()
                .max(segment_start_ms as f64) as u64;
            return estimated_ms.min(fallback_end_ms);
        }

        seen_words = next_seen_words;
    }

    fallback_end_ms
}

fn unique_transcript_text(candidate: &str, recent_texts: &VecDeque<String>) -> Option<String> {
    let candidate = remove_internal_repeated_sentences(candidate);
    if candidate.trim().is_empty() {
        return None;
    }
    if recent_texts.is_empty() {
        return Some(candidate);
    }

    let candidate_word_pairs = transcript_words(&candidate);
    let candidate_words = candidate_word_pairs
        .iter()
        .map(|word| word.normalized.clone())
        .collect::<Vec<_>>();
    if candidate_words.len() < LIVE_DUPLICATE_NGRAM_SIZE {
        let recent = recent_texts
            .iter()
            .rev()
            .take(LIVE_DUPLICATE_RECENT_SEGMENTS)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join(" ");
        let recent_words = normalized_words(&recent);
        return (!contains_word_sequence(&recent_words, &candidate_words))
            .then(|| candidate.to_string());
    }

    let recent = recent_texts
        .iter()
        .rev()
        .take(LIVE_DUPLICATE_RECENT_SEGMENTS)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join(" ");
    let recent_words = normalized_words(&recent);
    if recent_words.len() < LIVE_DUPLICATE_NGRAM_SIZE {
        return Some(candidate);
    }

    if duplicate_coverage(&candidate_words, &recent_words) >= LIVE_DUPLICATE_COVERAGE_THRESHOLD {
        return None;
    }

    if let Some(text) = trim_duplicate_suffix(&candidate_word_pairs, &recent_words) {
        return Some(text);
    }
    if let Some(text) = trim_duplicate_prefix(&candidate_word_pairs, &recent_words) {
        return Some(text);
    }
    if let Some(text) = remove_duplicate_word_spans(&candidate_word_pairs, &recent_words) {
        return Some(remove_internal_repeated_sentences(&text));
    }

    Some(candidate)
}

fn remove_duplicate_word_spans(
    words: &[TranscriptWord],
    recent_words: &[String],
) -> Option<String> {
    let mut keep = vec![true; words.len()];
    let normalized = words
        .iter()
        .map(|word| word.normalized.clone())
        .collect::<Vec<_>>();
    let mut changed = false;

    loop {
        let Some(span) = longest_common_word_span(&normalized, recent_words, &keep) else {
            break;
        };
        for item in keep.iter_mut().take(span.end).skip(span.start) {
            *item = false;
        }
        changed = true;
    }

    if !changed {
        return None;
    }

    let kept_words = words
        .iter()
        .zip(keep.iter())
        .filter_map(|(word, keep)| keep.then_some(word))
        .collect::<Vec<_>>();
    (kept_words.len() >= LIVE_DUPLICATE_MIN_KEEP_WORDS)
        .then(|| join_original_word_refs(&kept_words))
}

struct WordSpan {
    start: usize,
    end: usize,
}

fn longest_common_word_span(
    candidate_words: &[String],
    recent_words: &[String],
    keep: &[bool],
) -> Option<WordSpan> {
    let mut best = None;
    for start in 0..candidate_words.len() {
        if !keep[start] {
            continue;
        }

        for recent_start in 0..recent_words.len() {
            let mut length = 0usize;
            while start + length < candidate_words.len()
                && recent_start + length < recent_words.len()
                && keep[start + length]
                && candidate_words[start + length] == recent_words[recent_start + length]
            {
                length += 1;
            }

            if length >= LIVE_DUPLICATE_MIN_TRIM_WORDS {
                let should_replace = best
                    .as_ref()
                    .map(|span: &WordSpan| length > span.end - span.start)
                    .unwrap_or(true);
                if should_replace {
                    best = Some(WordSpan {
                        start,
                        end: start + length,
                    });
                }
            }
        }
    }

    best
}

fn trim_duplicate_suffix(words: &[TranscriptWord], recent_words: &[String]) -> Option<String> {
    if words.len() < LIVE_DUPLICATE_MIN_KEEP_WORDS + LIVE_DUPLICATE_MIN_TRIM_WORDS {
        return None;
    }

    let mut first_valid_split = None;
    for split in LIVE_DUPLICATE_MIN_KEEP_WORDS..=(words.len() - LIVE_DUPLICATE_MIN_TRIM_WORDS) {
        let suffix = words[split..]
            .iter()
            .map(|word| word.normalized.clone())
            .collect::<Vec<_>>();
        if duplicate_coverage(&suffix, recent_words) >= LIVE_DUPLICATE_COVERAGE_THRESHOLD {
            if sentence_ends_after(&words[split - 1].original) {
                return Some(join_original_words(&words[..split]));
            }
            first_valid_split.get_or_insert(split);
        }
    }

    first_valid_split.map(|split| join_original_words(&words[..split]))
}

fn trim_duplicate_prefix(words: &[TranscriptWord], recent_words: &[String]) -> Option<String> {
    if words.len() < LIVE_DUPLICATE_MIN_KEEP_WORDS + LIVE_DUPLICATE_MIN_TRIM_WORDS {
        return None;
    }

    let mut best_split = None;
    let mut best_sentence_split = None;
    for split in LIVE_DUPLICATE_MIN_TRIM_WORDS..=(words.len() - LIVE_DUPLICATE_MIN_KEEP_WORDS) {
        let prefix = words[..split]
            .iter()
            .map(|word| word.normalized.clone())
            .collect::<Vec<_>>();
        if duplicate_coverage(&prefix, recent_words) >= LIVE_DUPLICATE_COVERAGE_THRESHOLD {
            best_split = Some(split);
            if sentence_ends_after(&words[split - 1].original) {
                best_sentence_split = Some(split);
            }
        }
    }

    best_sentence_split
        .or(best_split)
        .map(|split| join_original_words(&words[split..]))
}

fn duplicate_coverage(candidate_words: &[String], recent_words: &[String]) -> f32 {
    let candidate_ngrams = word_ngrams(candidate_words, LIVE_DUPLICATE_NGRAM_SIZE);
    if candidate_ngrams.is_empty() {
        return 0.0;
    }

    let recent_ngrams = word_ngrams(recent_words, LIVE_DUPLICATE_NGRAM_SIZE);
    let covered = candidate_ngrams
        .iter()
        .filter(|ngram| recent_ngrams.contains(ngram))
        .count();
    covered as f32 / candidate_ngrams.len() as f32
}

fn contains_word_sequence(words: &[String], sequence: &[String]) -> bool {
    if sequence.is_empty() {
        return true;
    }
    if sequence.len() > words.len() {
        return false;
    }

    words
        .windows(sequence.len())
        .any(|window| window == sequence)
}

struct TranscriptWord {
    original: String,
    normalized: String,
}

fn transcript_words(text: &str) -> Vec<TranscriptWord> {
    text.split_whitespace()
        .filter_map(|word| {
            let normalized = normalize_word(word);
            (!normalized.is_empty()).then(|| TranscriptWord {
                original: word.to_string(),
                normalized,
            })
        })
        .collect()
}

fn sentence_ends_after(word: &str) -> bool {
    word.ends_with('.') || word.ends_with('!') || word.ends_with('?')
}

fn join_original_words(words: &[TranscriptWord]) -> String {
    words
        .iter()
        .map(|word| word.original.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

fn join_original_word_refs(words: &[&TranscriptWord]) -> String {
    words
        .iter()
        .map(|word| word.original.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

fn remove_internal_repeated_sentences(text: &str) -> String {
    let sentences = split_transcript_sentences(text);
    let mut seen = Vec::<Vec<String>>::new();
    let mut kept = Vec::new();

    for sentence in sentences {
        let normalized = normalized_words(&sentence);
        if normalized
            .iter()
            .all(|word| word.chars().all(|character| character.is_ascii_digit()))
        {
            continue;
        }
        if normalized.len() >= LIVE_DUPLICATE_MIN_KEEP_WORDS && seen.contains(&normalized) {
            continue;
        }
        if !normalized.is_empty() {
            seen.push(normalized);
        }
        kept.push(sentence);
    }

    kept.join(" ")
}

fn split_transcript_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut start = 0usize;

    for (index, character) in text.char_indices() {
        if character == '.' || character == '!' || character == '?' {
            let end = index + character.len_utf8();
            let sentence = text[start..end].trim();
            if !sentence.is_empty() {
                sentences.push(sentence.to_string());
            }
            start = end;
        }
    }

    let tail = text[start..].trim();
    if !tail.is_empty() {
        sentences.push(tail.to_string());
    }

    sentences
}

fn normalized_words(text: &str) -> Vec<String> {
    text.split_whitespace()
        .filter_map(|word| {
            let normalized = normalize_word(word);
            (!normalized.is_empty()).then_some(normalized)
        })
        .collect()
}

fn normalize_word(word: &str) -> String {
    word.chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(|character| character.to_lowercase())
        .collect()
}

fn word_ngrams(words: &[String], size: usize) -> Vec<String> {
    if words.len() < size {
        return Vec::new();
    }

    words.windows(size).map(|window| window.join(" ")).collect()
}

fn append_live_segment(path: &Path, segment: &TranscriptSegment) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("Failed to open transcript {}: {err}", path.display()))?;
    let line = serde_json::to_string(segment)
        .map_err(|err| format!("Failed to encode transcript segment: {err}"))?;
    writeln!(file, "{line}")
        .map_err(|err| format!("Failed to append transcript {}: {err}", path.display()))
}

fn emit_live_status(app: &AppHandle, thread_id: &str, active: bool, message: impl Into<String>) {
    let _ = app.emit(
        "live-transcript-status",
        LiveTranscriptStatusPayload {
            thread_id: thread_id.to_string(),
            active,
            message: message.into(),
            chunk_ms: LIVE_TRANSCRIPTION_WINDOW_MS,
            overlap_ms: LIVE_TRANSCRIPTION_STABILITY_DELAY_MS,
        },
    );
}

fn read_transcript_jsonl(path: &Path) -> Result<Vec<TranscriptSegment>, String> {
    if !path.is_file() {
        return Ok(Vec::new());
    }

    let file = fs::File::open(path)
        .map_err(|err| format!("Failed to read transcript {}: {err}", path.display()))?;
    let reader = BufReader::new(file);
    let mut segments = Vec::new();

    for line in reader.lines() {
        let line =
            line.map_err(|err| format!("Failed to read transcript {}: {err}", path.display()))?;
        if line.trim().is_empty() {
            continue;
        }
        segments.push(
            serde_json::from_str::<TranscriptSegment>(&line)
                .map_err(|err| format!("Invalid transcript line in {}: {err}", path.display()))?,
        );
    }

    segments.sort_by(|left, right| {
        left.start_ms
            .cmp(&right.start_ms)
            .then_with(|| left.source.cmp(&right.source))
    });
    Ok(segments)
}

fn count_jsonl_lines(path: &Path) -> Result<usize, String> {
    if !path.is_file() {
        return Ok(0);
    }

    let file = fs::File::open(path)
        .map_err(|err| format!("Failed to read transcript {}: {err}", path.display()))?;
    Ok(BufReader::new(file)
        .lines()
        .filter(|line| line.is_ok())
        .count())
}

fn write_text_atomic(path: &Path, content: &str) -> Result<(), String> {
    let tmp_path = path.with_extension("tmp");
    let mut file = fs::File::create(&tmp_path)
        .map_err(|err| format!("Failed to create {}: {err}", tmp_path.display()))?;
    file.write_all(content.as_bytes())
        .map_err(|err| format!("Failed to write {}: {err}", tmp_path.display()))?;
    file.sync_all()
        .map_err(|err| format!("Failed to sync {}: {err}", tmp_path.display()))?;
    fs::rename(&tmp_path, path).map_err(|err| {
        format!(
            "Failed to replace {} with {}: {err}",
            path.display(),
            tmp_path.display()
        )
    })
}

fn now_ms() -> Result<u64, String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("System clock is before UNIX epoch: {err}"))?
        .as_millis() as u64)
}

fn samples_to_ms(samples: u64, sample_rate: u32) -> u64 {
    ((samples as f64 * 1000.0) / sample_rate as f64).floor() as u64
}

fn ms_to_samples(ms: u64, sample_rate: u32) -> usize {
    ((ms as f64 * sample_rate as f64) / 1000.0).round() as usize
}

fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let square_sum = samples.iter().map(|sample| sample * sample).sum::<f32>();
    (square_sum / samples.len() as f32).sqrt()
}

fn resample_to_rate(samples: &[f32], source_rate: u32, target_rate: u32) -> Vec<f32> {
    if samples.is_empty() || source_rate == target_rate {
        return samples.to_vec();
    }

    let output_len =
        ((samples.len() as f64) * (target_rate as f64) / (source_rate as f64)).ceil() as usize;
    let ratio = source_rate as f64 / target_rate as f64;
    let mut output = Vec::with_capacity(output_len);

    for out_index in 0..output_len {
        let source_pos = out_index as f64 * ratio;
        let left = source_pos.floor() as usize;
        let right = (left + 1).min(samples.len() - 1);
        let frac = (source_pos - left as f64) as f32;
        let sample = samples[left] * (1.0 - frac) + samples[right] * frac;
        output.push(sample);
    }

    output
}

fn format_transcript_time(ms: u64) -> String {
    let total_seconds = ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{minutes:02}:{seconds:02}")
}

fn default_thread_count() -> usize {
    thread::available_parallelism()
        .map(|count| count.get().clamp(2, 8))
        .unwrap_or(4)
}

#[cfg(target_os = "macos")]
fn ensure_microphone_permission(app: &AppHandle) -> Result<(), String> {
    use std::sync::mpsc;

    use block2::RcBlock;
    use objc2::runtime::Bool;
    use objc2_av_foundation::{AVAuthorizationStatus, AVCaptureDevice, AVMediaTypeAudio};

    let (sender, receiver) = mpsc::channel();

    app.run_on_main_thread(move || {
        let media_type = match unsafe { AVMediaTypeAudio } {
            Some(media_type) => media_type,
            None => {
                let _ = sender.send(Err("AVMediaTypeAudio is unavailable".to_string()));
                return;
            }
        };
        let status = unsafe { AVCaptureDevice::authorizationStatusForMediaType(media_type) };

        match status {
            AVAuthorizationStatus::Authorized => {
                let _ = sender.send(Ok(()));
            }
            AVAuthorizationStatus::Denied => {
                let _ = sender.send(Err(
                    "Microphone access is denied in macOS Privacy & Security settings".to_string(),
                ));
            }
            AVAuthorizationStatus::Restricted => {
                let _ = sender.send(Err(
                    "Microphone access is restricted by macOS policy".to_string()
                ));
            }
            AVAuthorizationStatus::NotDetermined => {
                let block = RcBlock::new(move |granted: Bool| {
                    let result = if granted.as_bool() {
                        Ok(())
                    } else {
                        Err("Microphone permission was not granted".to_string())
                    };
                    let _ = sender.send(result);
                });

                unsafe {
                    AVCaptureDevice::requestAccessForMediaType_completionHandler(
                        media_type, &block,
                    );
                }
                std::mem::forget(block);
            }
            other => {
                let _ = sender.send(Err(format!(
                    "Unknown microphone authorization status: {other:?}"
                )));
            }
        }
    })
    .map_err(|err| format!("Failed to request microphone permission on main thread: {err}"))?;

    receiver
        .recv_timeout(Duration::from_secs(120))
        .map_err(|_| "Timed out waiting for microphone permission".to_string())?
}

#[cfg(not(target_os = "macos"))]
fn ensure_microphone_permission(_app: &AppHandle) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_transcript_text_matches_overlapping_rollup() {
        let mut recent = VecDeque::new();
        recent.push_back("This is a Just Notes quality assurance test.".to_string());
        recent.push_back(
            "The blue notebook is beside the silver microphone. Every local transcript should preserve these exactly."
                .to_string(),
        );

        assert_eq!(unique_transcript_text(
            "This is a Just Notes quality assurance test. The blue notebook is beside the silver microphone. Every local transcript should preserve these exact words.",
            &recent,
        ), None);
    }

    #[test]
    fn repeated_transcript_text_allows_new_sentence() {
        let mut recent = VecDeque::new();
        recent.push_back("This is a Just Notes quality assurance test.".to_string());

        assert_eq!(
            unique_transcript_text(
                "Chapter two begins with a calendar reminder and a project checkpoint.",
                &recent,
            ),
            Some(
                "Chapter two begins with a calendar reminder and a project checkpoint.".to_string()
            )
        );
    }

    #[test]
    fn unique_transcript_text_trims_repeated_suffix() {
        let mut recent = VecDeque::new();
        recent.push_back(
            "Section 4 says the design notes mention a purple marker and a glass keyboard."
                .to_string(),
        );
        recent.push_back(
            "Section 5 says the engineering plan includes local storage. Audio can also include an"
                .to_string(),
        );

        assert_eq!(
            unique_transcript_text(
                "audio capture and live transcription. Section 4 says the design notes mention a purple marker and a glass keyboard. Section 5 says the engineering plan includes local storage, audio capture, and live transcription.",
                &recent,
            ),
            Some("audio capture and live transcription.".to_string()),
        );
    }

    #[test]
    fn unique_transcript_text_removes_repeated_middle_span() {
        let mut recent = VecDeque::new();
        recent.push_back(
            "Section 18 says the test is half-way through and the steady voice should continue."
                .to_string(),
        );

        assert_eq!(
            unique_transcript_text(
                "the local application and long-running stability. Section 18 says the test is half-way through and the steady voice should continue. Section 19 says the local app must not require cloud services for the transcript.",
                &recent,
            ),
            Some(
                "the local application and long-running stability. Section 19 says the local app must not require cloud services for the transcript."
                    .to_string()
            ),
        );
    }

    #[test]
    fn unique_transcript_text_trims_longest_repeated_prefix() {
        let mut recent = VecDeque::new();
        recent.push_back(
            "Section 1 says the green calendar moved beside the copper lamp.".to_string(),
        );

        assert_eq!(
            unique_transcript_text(
                "Section 1 says the green calendar moved beside the copper lamp. Section 2 says the yellow folder stayed under the quiet monitor.",
                &recent,
            ),
            Some(
                "Section 2 says the yellow folder stayed under the quiet monitor.".to_string()
            ),
        );
    }

    #[test]
    fn unique_transcript_text_removes_internal_repeated_sentence() {
        let recent = VecDeque::new();

        assert_eq!(
            unique_transcript_text(
                "Section 21 says the navy notebook contains project tasks. Section 21 says the navy notebook contains project tasks.",
                &recent,
            ),
            Some("Section 21 says the navy notebook contains project tasks.".to_string()),
        );
    }

    #[test]
    fn unique_transcript_text_drops_short_duplicate_fragments() {
        let mut recent = VecDeque::new();
        recent.push_back(
            "Section 7 says the transcript should advance steadily without repeating earlier phrases."
                .to_string(),
        );

        assert_eq!(unique_transcript_text("phrases.", &recent), None);
    }

    #[test]
    fn unique_transcript_text_removes_numeric_sentence_artifacts() {
        let recent = VecDeque::new();

        assert_eq!(
            unique_transcript_text(
                "3. Section 4 says the design notes mention a purple marker and a glass keyboard. 4.",
                &recent,
            ),
            Some(
                "Section 4 says the design notes mention a purple marker and a glass keyboard."
                    .to_string()
            ),
        );
    }

    #[test]
    fn completed_transcript_text_drops_live_partial_sentence() {
        assert_eq!(
            completed_transcript_text(
                "Section 1 says the green calendar moved beside the copper lamp. Section 2 says the yellow",
                false,
            ),
            "Section 1 says the green calendar moved beside the copper lamp.".to_string(),
        );
        assert_eq!(
            completed_transcript_text("Section 2 says the yellow", false),
            "".to_string(),
        );
        assert_eq!(
            completed_transcript_text("Section 2 says the yellow", true),
            "Section 2 says the yellow".to_string(),
        );
    }

    #[test]
    fn common_transcript_prefix_returns_agreed_words_from_latest_text() {
        assert_eq!(
            common_transcript_prefix(
                "Section 1 says the green calendar moved beside the copper lamp. Section 2 says the yellow",
                "Section 1 says the green calendar moved beside the copper lamp. Section 2 says the yellow folder stayed under the quiet monitor.",
            ),
            Some(
                "Section 1 says the green calendar moved beside the copper lamp. Section 2 says the yellow"
                    .to_string()
            ),
        );
    }

    #[test]
    fn live_channel_state_requires_two_matching_hypotheses() {
        let mut state = LiveChannelState::new("system", "Others", 48_000);
        assert_eq!(
            state.agreed_text(
                "Section 1 says the green calendar moved beside the copper lamp.",
                false,
            ),
            None,
        );
        assert_eq!(
            state.agreed_text(
                "Section 1 says the green calendar moved beside the copper lamp. Section 2 says the yellow folder.",
                false,
            ),
            Some("Section 1 says the green calendar moved beside the copper lamp.".to_string()),
        );
    }

    #[test]
    fn clean_transcript_text_removes_leading_non_speech_marker() {
        assert_eq!(
            clean_transcript_text("(no audio) Long recording quality test begins now."),
            "Long recording quality test begins now.".to_string(),
        );
    }

    #[test]
    fn estimate_text_end_ms_tracks_confirmed_prefix() {
        let segments = vec![
            TranscriptSegment {
                speaker: "Others".to_string(),
                source: "system".to_string(),
                start_ms: 0,
                end_ms: 4_000,
                text: "Section 1 says the green calendar moved beside the copper lamp.".to_string(),
            },
            TranscriptSegment {
                speaker: "Others".to_string(),
                source: "system".to_string(),
                start_ms: 4_000,
                end_ms: 8_000,
                text: "Section 2 says the yellow folder stayed under the quiet monitor."
                    .to_string(),
            },
        ];

        assert_eq!(
            estimate_text_end_ms(
                &segments,
                "Section 1 says the green calendar moved beside the copper lamp.",
                10_000,
                22_000,
            ),
            14_000,
        );
    }
}

pub fn run() {
    let paths = AppPaths::discover().expect("failed to locate Just Notes data directory");

    let builder = tauri::Builder::default()
        .manage(paths)
        .manage(RecorderState::default())
        .setup(|app| {
            reset_stale_recording_threads(app.state::<AppPaths>().inner())?;
            Ok(())
        });

    #[cfg(any(debug_assertions, feature = "qa-fixtures"))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        get_app_info,
        list_threads,
        create_thread,
        get_thread,
        get_transcription_status,
        start_recording,
        start_fixture_recording,
        stop_recording
    ]);

    #[cfg(not(any(debug_assertions, feature = "qa-fixtures")))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        get_app_info,
        list_threads,
        create_thread,
        get_thread,
        get_transcription_status,
        start_recording,
        stop_recording
    ]);

    builder
        .run(tauri::generate_context!())
        .expect("error while running Just Notes");
}
