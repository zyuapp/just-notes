use std::{
    collections::VecDeque,
    env, fs,
    fs::OpenOptions,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::Command,
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
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};

const LIVE_TRANSCRIPTION_CHUNK_MS: u64 = 3_000;
const LIVE_TRANSCRIPTION_OVERLAP_MS: u64 = 750;
const LIVE_TRANSCRIPTION_POLL_MS: u64 = 250;
const LIVE_SILENCE_RMS_THRESHOLD: f32 = 0.005;
const MAX_ROLLING_BUFFER_MS: u64 = 120_000;

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
        TranscriptionPaths {
            engine_path: self.data_dir.join("engine").join("whisper-cli"),
            model_path: self
                .data_dir
                .join("models")
                .join("whisper")
                .join("ggml-base.en.bin"),
        }
    }
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
    _mic_stream: Stream,
    _system_stream: Stream,
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
    message: String,
}

struct TranscriptionPaths {
    engine_path: PathBuf,
    model_path: PathBuf,
}

#[tauri::command]
fn get_app_info(paths: tauri::State<'_, AppPaths>) -> AppInfo {
    let paths = paths.inner();
    AppInfo {
        data_dir: paths.data_dir.display().to_string(),
        threads_dir: paths.threads_dir.display().to_string(),
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
fn get_thread(paths: tauri::State<'_, AppPaths>, thread_id: String) -> Result<ThreadDetail, String> {
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
        transcript_markdown_path: thread_dir
            .join("transcript.md")
            .display()
            .to_string(),
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
    let json =
        fs::read_to_string(path).map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
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
        let entry =
            entry.map_err(|err| format!("Failed to read {}: {err}", paths.threads_dir.display()))?;
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
    if recorder.is_starting.swap(true, Ordering::SeqCst) {
        return Err("Audio startup is already in progress".to_string());
    }

    let result = prepare_recording_session(app, paths, &recorder, requested_thread_id);
    recorder.is_starting.store(false, Ordering::SeqCst);
    result
}

fn prepare_recording_session(
    app: AppHandle,
    paths: AppPaths,
    recorder: &RecorderState,
    requested_thread_id: Option<String>,
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
    let host = cpal::default_host();
    ensure_microphone_permission(&app)?;

    let mic_device = host
        .default_input_device()
        .ok_or_else(|| "No default microphone input device is available".to_string())?;
    let system_device = host
        .default_output_device()
        .ok_or_else(|| "No default system output device is available".to_string())?;

    let mic_config = mic_device
        .default_input_config()
        .map_err(|err| format!("Failed to read microphone config: {err}"))?;
    let system_config = system_device
        .default_output_config()
        .map_err(|err| format!("Failed to read system output config: {err}"))?;

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
        system_device,
        system_config,
        Arc::clone(&buffers),
        CaptureSource::System,
    )?;

    set_thread_status(&thread_dir, ThreadStatus::Recording)?;

    mic_stream
        .play()
        .map_err(|err| format!("Failed to start microphone stream: {err}"))?;
    system_stream
        .play()
        .map_err(|err| format!("Failed to start system loopback stream: {err}"))?;

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
        _mic_stream: mic_stream,
        _system_stream: system_stream,
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
        _mic_stream,
        _system_stream,
    } = session;

    should_stop_meter.store(true, Ordering::Relaxed);
    should_stop_live_transcription.store(true, Ordering::Relaxed);
    if let Some(thread) = meter_thread.take() {
        let _ = thread.join();
    }
    drop(_mic_stream);
    drop(_system_stream);
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
    markdown.push_str(&format!("Duration: `{}`\n\n", format_transcript_time(duration_ms)));

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
    let engine_exists = transcription_paths.engine_path.is_file();
    let model_exists = transcription_paths.model_path.is_file();
    let ready = engine_exists && model_exists;
    let message = match (engine_exists, model_exists) {
        (true, true) => "Local transcription is ready".to_string(),
        (false, true) => "Local transcription engine is missing".to_string(),
        (true, false) => "Local transcription model is missing".to_string(),
        (false, false) => "Local transcription engine and model are missing".to_string(),
    };

    TranscriptionStatusPayload {
        ready,
        engine_exists,
        model_exists,
        engine_path: transcription_paths.engine_path.display().to_string(),
        model_path: transcription_paths.model_path.display().to_string(),
        message,
    }
}

#[derive(Clone, Copy)]
enum CaptureSource {
    Mic,
    System,
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
    if !paths.engine_path.is_file() || !paths.model_path.is_file() {
        emit_live_status(
            &app,
            &thread_id,
            false,
            "Live transcription is disabled until the local engine and model are installed",
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
                    chunk_ms: LIVE_TRANSCRIPTION_CHUNK_MS,
                    overlap_ms: LIVE_TRANSCRIPTION_OVERLAP_MS,
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
    emit_live_status(
        &app,
        &thread_id,
        true,
        "Live transcription is listening with 3s chunks and 750ms overlap",
    );

    let work_dir = thread_dir.join("work");
    let jsonl_path = thread_dir.join("transcript.jsonl");
    let mut mic = LiveChannelState::new("mic", "You", mic_sample_rate);
    let mut system = LiveChannelState::new("system", "Others", system_sample_rate);
    let mut emitted_count = 0usize;

    loop {
        let stopping = should_stop.load(Ordering::Relaxed);
        emitted_count += process_live_channel(
            &app,
            &paths,
            &buffers,
            &work_dir,
            &jsonl_path,
            &thread_dir,
            &thread_id,
            &mut mic,
            stopping,
        )?;
        emitted_count += process_live_channel(
            &app,
            &paths,
            &buffers,
            &work_dir,
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
    next_chunk_end_ms: u64,
    committed_until_ms: u64,
    last_emitted_end_ms: u64,
    chunk_index: usize,
}

impl LiveChannelState {
    fn new(source: &'static str, speaker: &'static str, sample_rate: u32) -> Self {
        Self {
            source,
            speaker,
            sample_rate,
            next_chunk_end_ms: LIVE_TRANSCRIPTION_CHUNK_MS,
            committed_until_ms: 0,
            last_emitted_end_ms: 0,
            chunk_index: 0,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn process_live_channel(
    app: &AppHandle,
    paths: &TranscriptionPaths,
    buffers: &Arc<Mutex<SharedBuffers>>,
    work_dir: &Path,
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
    } else if available_ms >= state.next_chunk_end_ms {
        state.next_chunk_end_ms
    } else {
        return Ok(0);
    };
    let commit_until_ms = if final_flush {
        target_end_ms
    } else {
        target_end_ms.saturating_sub(LIVE_TRANSCRIPTION_OVERLAP_MS)
    };
    if commit_until_ms <= state.committed_until_ms {
        state.next_chunk_end_ms += LIVE_TRANSCRIPTION_CHUNK_MS;
        return Ok(0);
    }

    let window_start_ms = state
        .committed_until_ms
        .saturating_sub(LIVE_TRANSCRIPTION_OVERLAP_MS);
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
        state.next_chunk_end_ms = target_end_ms + LIVE_TRANSCRIPTION_CHUNK_MS;
        return Ok(0);
    };

    if rms(&samples) < LIVE_SILENCE_RMS_THRESHOLD {
        state.committed_until_ms = commit_until_ms;
        state.next_chunk_end_ms = target_end_ms + LIVE_TRANSCRIPTION_CHUNK_MS;
        return Ok(0);
    }

    let stem = format!("{}-{:04}", state.source, state.chunk_index);
    let chunk_path = work_dir.join(format!("{stem}-16k.wav"));
    let normalized_samples = resample_to_rate(&samples, state.sample_rate, 16_000);
    write_wav(&chunk_path, 16_000, &normalized_samples)?;

    let output_prefix = work_dir.join(stem);
    let mut segments = transcribe_chunk(
        &paths.engine_path,
        &paths.model_path,
        &chunk_path,
        &output_prefix,
        state.source,
        state.speaker,
    )?;
    segments.sort_by_key(|segment| segment.start_ms);

    let mut emitted = 0usize;
    for mut segment in segments {
        segment.start_ms += window_start_ms;
        segment.end_ms += window_start_ms;
        segment.start_ms = segment.start_ms.min(target_end_ms);
        segment.end_ms = segment.end_ms.min(target_end_ms).max(segment.start_ms);

        if segment.start_ms < state.last_emitted_end_ms || segment.start_ms >= commit_until_ms {
            continue;
        }

        append_live_segment(jsonl_path, &segment)?;
        touch_thread(thread_dir)?;
        let emitted_end_ms = segment.end_ms;
        let _ = app.emit(
            "live-transcript-segment",
            LiveTranscriptSegmentPayload {
                thread_id: thread_id.to_string(),
                committed_until_ms: commit_until_ms,
                segment,
            },
        );
        state.last_emitted_end_ms = state.last_emitted_end_ms.max(emitted_end_ms);
        emitted += 1;
    }

    state.committed_until_ms = commit_until_ms;
    state.next_chunk_end_ms = target_end_ms + LIVE_TRANSCRIPTION_CHUNK_MS;
    state.chunk_index += 1;
    Ok(emitted)
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
            chunk_ms: LIVE_TRANSCRIPTION_CHUNK_MS,
            overlap_ms: LIVE_TRANSCRIPTION_OVERLAP_MS,
        },
    );
}

fn transcribe_chunk(
    whisper_cli: &Path,
    model_path: &Path,
    input_path: &Path,
    output_prefix: &Path,
    source: &str,
    speaker: &str,
) -> Result<Vec<TranscriptSegment>, String> {
    let output = Command::new(whisper_cli)
        .arg("-m")
        .arg(model_path)
        .arg("-f")
        .arg(input_path)
        .arg("-l")
        .arg("en")
        .arg("-t")
        .arg(default_thread_count().to_string())
        .arg("-oj")
        .arg("-otxt")
        .arg("-of")
        .arg(output_prefix)
        .output()
        .map_err(|err| {
            format!(
                "Failed to run whisper.cpp for {source} audio with {}: {err}",
                whisper_cli.display()
            )
        })?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "whisper.cpp failed for {source} audio with status {}:\n{}",
            output.status,
            stderr.trim()
        ));
    }

    let json_path = output_prefix.with_extension("json");
    if json_path.is_file() {
        let json = fs::read_to_string(&json_path)
            .map_err(|err| format!("Failed to read {}: {err}", json_path.display()))?;
        return parse_whisper_json(&json, source, speaker);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Ok(segments) = parse_whisper_json(&stdout, source, speaker) {
        return Ok(segments);
    }

    let txt_path = output_prefix.with_extension("txt");
    if txt_path.is_file() {
        let text = fs::read_to_string(&txt_path)
            .map_err(|err| format!("Failed to read {}: {err}", txt_path.display()))?;
        return Ok(parse_whisper_text(&text, source, speaker));
    }

    Err(format!(
        "whisper.cpp completed for {source} audio but did not produce {} or {}. stderr:\n{}",
        json_path.display(),
        txt_path.display(),
        stderr.trim()
    ))
}

fn parse_whisper_json(
    json: &str,
    source: &str,
    speaker: &str,
) -> Result<Vec<TranscriptSegment>, String> {
    let value: Value =
        serde_json::from_str(json).map_err(|err| format!("Invalid whisper JSON: {err}"))?;
    let entries = value
        .get("transcription")
        .or_else(|| value.get("segments"))
        .and_then(Value::as_array)
        .ok_or_else(|| "Whisper JSON did not contain transcription segments".to_string())?;

    let mut segments = Vec::new();
    for entry in entries {
        let text = entry
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        if text.is_empty() {
            continue;
        }

        let (start_ms, end_ms) = extract_segment_times(entry);
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

fn extract_segment_times(entry: &Value) -> (u64, u64) {
    if let Some(offsets) = entry.get("offsets") {
        let start_ms = offsets.get("from").and_then(Value::as_u64).unwrap_or(0);
        let end_ms = offsets
            .get("to")
            .and_then(Value::as_u64)
            .unwrap_or(start_ms);
        return (start_ms, end_ms);
    }

    if let (Some(start), Some(end)) = (entry.get("start"), entry.get("end")) {
        return (json_number_to_ms(start), json_number_to_ms(end));
    }

    if let Some(timestamps) = entry.get("timestamps") {
        let start_ms = timestamps
            .get("from")
            .and_then(Value::as_str)
            .and_then(parse_timecode)
            .unwrap_or(0);
        let end_ms = timestamps
            .get("to")
            .and_then(Value::as_str)
            .and_then(parse_timecode)
            .unwrap_or(start_ms);
        return (start_ms, end_ms);
    }

    (0, 0)
}

fn json_number_to_ms(value: &Value) -> u64 {
    if let Some(number) = value.as_f64() {
        return (number * 1000.0).round().max(0.0) as u64;
    }

    value.as_u64().unwrap_or_default()
}

fn parse_whisper_text(text: &str, source: &str, speaker: &str) -> Vec<TranscriptSegment> {
    let mut segments = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let Some(end_index) = trimmed.find(']') else {
            continue;
        };
        let Some(time_range) = trimmed.strip_prefix('[').map(|line| &line[..end_index - 1]) else {
            continue;
        };
        let Some((from, to)) = time_range.split_once("-->") else {
            continue;
        };
        let text = trimmed[end_index + 1..].trim();
        if text.is_empty() {
            continue;
        }

        segments.push(TranscriptSegment {
            speaker: speaker.to_string(),
            source: source.to_string(),
            start_ms: parse_timecode(from.trim()).unwrap_or(0),
            end_ms: parse_timecode(to.trim()).unwrap_or(0),
            text: text.to_string(),
        });
    }

    segments
}

fn parse_timecode(value: &str) -> Option<u64> {
    let normalized = value.replace(',', ".");
    let parts: Vec<&str> = normalized.split(':').collect();
    if parts.len() != 3 {
        return None;
    }

    let hours = parts[0].parse::<u64>().ok()?;
    let minutes = parts[1].parse::<u64>().ok()?;
    let seconds = parts[2].parse::<f64>().ok()?;
    Some((((hours * 60 + minutes) * 60) as f64 * 1000.0 + seconds * 1000.0).round() as u64)
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
    Ok(BufReader::new(file).lines().filter(|line| line.is_ok()).count())
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

fn write_wav(path: &Path, sample_rate: u32, samples: &[f32]) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)
        .map_err(|err| format!("Failed to create {}: {err}", path.display()))?;

    for sample in samples {
        let scaled = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        writer
            .write_sample(scaled)
            .map_err(|err| format!("Failed to write {}: {err}", path.display()))?;
    }

    writer
        .finalize()
        .map_err(|err| format!("Failed to finalize {}: {err}", path.display()))
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

pub fn run() {
    let paths = AppPaths::discover().expect("failed to locate Just Notes data directory");

    tauri::Builder::default()
        .manage(paths)
        .manage(RecorderState::default())
        .setup(|app| {
            reset_stale_recording_threads(app.state::<AppPaths>().inner())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            list_threads,
            create_thread,
            get_thread,
            get_transcription_status,
            start_recording,
            stop_recording
        ])
        .run(tauri::generate_context!())
        .expect("error while running Just Notes");
}
