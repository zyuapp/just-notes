use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use tauri::{AppHandle, Emitter, Manager};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

mod app;
mod capture;
mod ipc;
mod threads;
mod transcription;

use app::AppPaths;
use capture::{
    prepare_audio_input, start_audio_capture, stop_audio_capture, ActiveAudioCapture,
    PreparedAudioInput, RecordingInputMode, SharedBuffers,
};
use ipc::{
    AppInfo, LiveTranscriptSegmentPayload, LiveTranscriptStatusPayload, MeterPayload,
    RecordingPayload,
};
use threads::repository::{
    create_thread as create_thread_record, list_threads as list_thread_records, load_thread_by_id,
    prepare_work_dir, render_thread_markdown, reset_stale_recording_threads, set_thread_status,
    touch_thread,
};
use threads::transcript_store::append_live_segment;
use threads::{ThreadDetail, ThreadStatus, ThreadSummary, TranscriptSegment};
use transcription::{
    clean_transcript_text, common_transcript_prefix, completed_transcript_text,
    estimate_text_end_ms, first_audible_ms, is_ignored_transcript_text, ms_to_samples,
    resample_to_rate, rms, samples_to_ms, unique_transcript_text, TranscriptionPaths,
    TranscriptionStatusPayload, LIVE_DUPLICATE_RECENT_SEGMENTS,
};

type LiveSampleWindow = Option<Vec<f32>>;

const LIVE_TRANSCRIPTION_STEP_MS: u64 = 2_000;
const LIVE_TRANSCRIPTION_WINDOW_MS: u64 = 12_000;
const LIVE_TRANSCRIPTION_MAX_AGREEMENT_BUFFER_MS: u64 = 30_000;
const LIVE_TRANSCRIPTION_STABILITY_DELAY_MS: u64 = 2_000;
const LIVE_TRANSCRIPTION_POLL_MS: u64 = 250;
const LIVE_SILENCE_RMS_THRESHOLD: f32 = 0.005;

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

struct RecordingSessionConfig {
    app: AppHandle,
    thread_id: String,
    thread_dir: PathBuf,
    started: Instant,
    input: PreparedAudioInput,
    transcription_paths: TranscriptionPaths,
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
    list_thread_records(paths.inner())
}

#[tauri::command]
fn create_thread(paths: tauri::State<'_, AppPaths>) -> Result<ThreadDetail, String> {
    create_thread_record(paths.inner())
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
    ensure_recorder_idle(recorder)?;
    paths.ensure()?;

    let thread = select_recording_thread(&paths, requested_thread_id)?;
    let thread_id = thread.summary.id.clone();
    let thread_dir = paths.thread_dir(&thread_id);
    prepare_work_dir(&thread_dir)?;

    let started = Instant::now();
    let mut input = prepare_audio_input(&app, input_mode)?;
    set_thread_status(&thread_dir, ThreadStatus::Recording)?;
    start_audio_capture(&mut input.audio_capture)?;

    let transcription = transcription_status(&paths);
    let session = build_recording_session(RecordingSessionConfig {
        app,
        thread_id: thread_id.clone(),
        thread_dir,
        started,
        input,
        transcription_paths: paths.transcription_paths(),
    });
    store_recording_session(recorder, session)?;

    Ok(RecordingPayload {
        thread: load_thread_by_id(&paths, &thread_id)?,
        transcription,
    })
}

fn ensure_recorder_idle(recorder: &RecorderState) -> Result<(), String> {
    let session = recorder
        .session
        .lock()
        .map_err(|_| "Recorder state lock was poisoned".to_string())?;
    if session.is_some() {
        return Err("Recording is already active".to_string());
    }
    Ok(())
}

fn build_recording_session(config: RecordingSessionConfig) -> RecorderSession {
    let RecordingSessionConfig {
        app,
        thread_id,
        thread_dir,
        started,
        input,
        transcription_paths,
    } = config;
    let PreparedAudioInput {
        mic_sample_rate,
        system_sample_rate,
        buffers,
        audio_capture,
    } = input;
    let should_stop_meter = Arc::new(AtomicBool::new(false));
    let meter_thread = spawn_meter_thread(
        app.clone(),
        thread_id.clone(),
        Arc::clone(&buffers),
        Arc::clone(&should_stop_meter),
        started,
    );

    let should_stop_live_transcription = Arc::new(AtomicBool::new(false));
    let live_transcription_thread =
        spawn_live_transcription_thread(LiveTranscriptionThreadConfig {
            app,
            paths: transcription_paths,
            buffers: Arc::clone(&buffers),
            should_stop: Arc::clone(&should_stop_live_transcription),
            thread_id: thread_id.clone(),
            thread_dir: thread_dir.clone(),
            mic_sample_rate,
            system_sample_rate,
        });

    RecorderSession {
        thread_id,
        thread_dir,
        started,
        buffers,
        should_stop_meter,
        should_stop_live_transcription,
        meter_thread: Some(meter_thread),
        live_transcription_thread,
        audio_capture,
    }
}

fn store_recording_session(
    recorder: &RecorderState,
    session: RecorderSession,
) -> Result<(), String> {
    let mut slot = recorder
        .session
        .lock()
        .map_err(|_| "Recorder state lock was poisoned".to_string())?;
    if slot.is_some() {
        return Err("Recording is already active".to_string());
    }

    *slot = Some(session);
    Ok(())
}

fn select_recording_thread(
    paths: &AppPaths,
    requested_thread_id: Option<String>,
) -> Result<ThreadDetail, String> {
    let Some(thread_id) = requested_thread_id else {
        return create_thread_record(paths);
    };

    let thread = load_thread_by_id(paths, &thread_id)?;
    if thread.summary.status == ThreadStatus::Recording {
        return Err("The selected thread is already recording".to_string());
    }
    if thread.summary.segment_count > 0 {
        return create_thread_record(paths);
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

struct LiveTranscriptionThreadConfig {
    app: AppHandle,
    paths: TranscriptionPaths,
    buffers: Arc<Mutex<SharedBuffers>>,
    should_stop: Arc<AtomicBool>,
    thread_id: String,
    thread_dir: PathBuf,
    mic_sample_rate: u32,
    system_sample_rate: u32,
}

fn spawn_live_transcription_thread(
    config: LiveTranscriptionThreadConfig,
) -> Option<JoinHandle<()>> {
    let LiveTranscriptionThreadConfig {
        app,
        paths,
        buffers,
        should_stop,
        thread_id,
        thread_dir,
        mic_sample_rate,
        system_sample_rate,
    } = config;

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
        let config = LiveTranscriptionThreadConfig {
            app: app.clone(),
            paths,
            buffers,
            should_stop,
            thread_id: thread_id.clone(),
            thread_dir,
            mic_sample_rate,
            system_sample_rate,
        };
        if let Err(err) = run_live_transcription_loop(config) {
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

fn run_live_transcription_loop(config: LiveTranscriptionThreadConfig) -> Result<(), String> {
    let LiveTranscriptionThreadConfig {
        app,
        paths,
        buffers,
        should_stop,
        thread_id,
        thread_dir,
        mic_sample_rate,
        system_sample_rate,
    } = config;
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
        let context = LiveChannelContext {
            app: &app,
            whisper: &whisper,
            buffers: &buffers,
            jsonl_path: &jsonl_path,
            thread_dir: &thread_dir,
            thread_id: &thread_id,
        };
        let stopping = should_stop.load(Ordering::Relaxed);
        emitted_count += process_live_channel(&context, &mut mic, stopping)?;
        emitted_count += process_live_channel(&context, &mut system, stopping)?;

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

struct LiveChannelContext<'a> {
    app: &'a AppHandle,
    whisper: &'a WhisperRuntime,
    buffers: &'a Arc<Mutex<SharedBuffers>>,
    jsonl_path: &'a Path,
    thread_dir: &'a Path,
    thread_id: &'a str,
}

struct LiveDecodeWindow {
    target_end_ms: u64,
    commit_until_ms: u64,
    window_start_ms: u64,
    samples: Vec<f32>,
    audible_start_ms: Option<u64>,
}

fn live_target_end_ms(
    available_ms: u64,
    state: &LiveChannelState,
    final_flush: bool,
) -> Option<u64> {
    let target_end_ms = if final_flush {
        available_ms
    } else if available_ms >= state.next_decode_ms {
        state.next_decode_ms
    } else {
        return None;
    };
    Some(target_end_ms)
}

fn live_commit_until_ms(target_end_ms: u64, final_flush: bool) -> u64 {
    if final_flush {
        target_end_ms
    } else {
        target_end_ms.saturating_sub(LIVE_TRANSCRIPTION_STABILITY_DELAY_MS)
    }
}

fn prepare_live_decode_window(
    buffers: &Arc<Mutex<SharedBuffers>>,
    state: &mut LiveChannelState,
    final_flush: bool,
) -> Result<Option<LiveDecodeWindow>, String> {
    let available_samples = live_available_samples(buffers, state.source)?;
    let available_ms = samples_to_ms(available_samples, state.sample_rate);
    if available_ms <= state.committed_until_ms {
        return Ok(None);
    }

    let Some(target_end_ms) = live_target_end_ms(available_ms, state, final_flush) else {
        return Ok(None);
    };
    let commit_until_ms = live_commit_until_ms(target_end_ms, final_flush);
    if commit_until_ms <= state.committed_until_ms {
        state.next_decode_ms += LIVE_TRANSCRIPTION_STEP_MS;
        return Ok(None);
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

    let samples = live_samples(buffers, state.source, start_index, end_index)?;

    let Some(samples) = samples else {
        advance_live_decode(state, commit_until_ms, target_end_ms);
        return Ok(None);
    };

    if rms(&samples) < LIVE_SILENCE_RMS_THRESHOLD {
        advance_live_decode(state, commit_until_ms, target_end_ms);
        return Ok(None);
    }

    let audible_start_ms =
        first_audible_ms(&samples, state.sample_rate, LIVE_SILENCE_RMS_THRESHOLD)
            .map(|offset_ms| window_start_ms + offset_ms);
    Ok(Some(LiveDecodeWindow {
        target_end_ms,
        commit_until_ms,
        window_start_ms,
        samples,
        audible_start_ms,
    }))
}

fn live_available_samples(
    buffers: &Arc<Mutex<SharedBuffers>>,
    source: &str,
) -> Result<u64, String> {
    let shared = buffers
        .lock()
        .map_err(|_| "Audio buffer lock was poisoned".to_string())?;
    let available_samples = match source {
        "mic" => shared.mic.available_end_index(),
        "system" => shared.system.available_end_index(),
        _ => 0,
    };
    Ok(available_samples)
}

fn live_samples(
    buffers: &Arc<Mutex<SharedBuffers>>,
    source: &str,
    start_index: u64,
    end_index: u64,
) -> Result<LiveSampleWindow, String> {
    let shared = buffers
        .lock()
        .map_err(|_| "Audio buffer lock was poisoned".to_string())?;
    let samples = match source {
        "mic" => shared.mic.window(start_index, end_index),
        "system" => shared.system.window(start_index, end_index),
        _ => None,
    };
    Ok(samples)
}

fn advance_live_decode(state: &mut LiveChannelState, commit_until_ms: u64, target_end_ms: u64) {
    state.committed_until_ms = commit_until_ms;
    state.next_decode_ms = target_end_ms + LIVE_TRANSCRIPTION_STEP_MS;
}

fn process_live_channel(
    context: &LiveChannelContext<'_>,
    state: &mut LiveChannelState,
    final_flush: bool,
) -> Result<usize, String> {
    let Some(window) = prepare_live_decode_window(context.buffers, state, final_flush)? else {
        return Ok(0);
    };

    let normalized_samples = resample_to_rate(&window.samples, state.sample_rate, 16_000);
    let mut segments = context.whisper.transcribe(
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
        advance_live_decode(state, window.commit_until_ms, window.target_end_ms);
        return Ok(0);
    };

    let completed_agreed_text = completed_transcript_text(&agreed_text, final_flush);
    if completed_agreed_text.is_empty() {
        advance_live_decode(state, window.commit_until_ms, window.target_end_ms);
        return Ok(0);
    }
    let stable_end_ms = estimate_text_end_ms(
        &segments,
        &completed_agreed_text,
        window.window_start_ms,
        window.commit_until_ms,
    );

    let emitted = emit_unique_live_segment(
        context,
        state,
        &window,
        &completed_agreed_text,
        stable_end_ms,
    )?;
    advance_live_decode(state, window.commit_until_ms, window.target_end_ms);
    Ok(emitted)
}

fn emit_unique_live_segment(
    context: &LiveChannelContext<'_>,
    state: &mut LiveChannelState,
    window: &LiveDecodeWindow,
    text: &str,
    stable_end_ms: u64,
) -> Result<usize, String> {
    let Some(unique_text) = state.unique_text(text) else {
        state.mark_emitted_until(stable_end_ms);
        return Ok(0);
    };

    let unique_text = unique_text.trim().to_string();
    if unique_text.is_empty() {
        state.mark_emitted_until(stable_end_ms);
        return Ok(0);
    }

    let segment = TranscriptSegment {
        speaker: state.speaker.to_string(),
        source: state.source.to_string(),
        start_ms: state
            .last_emitted_end_ms
            .max(window.window_start_ms)
            .max(window.audible_start_ms.unwrap_or(window.window_start_ms)),
        end_ms: stable_end_ms,
        text: unique_text,
    };

    if segment.end_ms <= segment.start_ms {
        return Ok(0);
    }

    append_live_segment(context.jsonl_path, &segment)?;
    touch_thread(context.thread_dir)?;
    state.remember_prompt_text(&segment.text);
    state.remember_emitted_text(&segment.text);
    let _ = context.app.emit(
        "live-transcript-segment",
        LiveTranscriptSegmentPayload {
            thread_id: context.thread_id.to_string(),
            committed_until_ms: window.commit_until_ms,
            segment,
        },
    );
    state.mark_emitted_until(stable_end_ms);
    Ok(1)
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

pub(crate) fn now_ms() -> Result<u64, String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("System clock is before UNIX epoch: {err}"))?
        .as_millis() as u64)
}

fn default_thread_count() -> usize {
    thread::available_parallelism()
        .map(|count| count.get().clamp(2, 8))
        .unwrap_or(4)
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

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use super::*;
    use crate::threads::transcript_store::read_transcript_jsonl;

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

    #[test]
    fn first_audible_ms_skips_leading_silence() {
        let mut samples = vec![0.0; 16_000 * 3];
        samples.extend(vec![0.04; 16_000]);

        assert_eq!(
            first_audible_ms(&samples, 16_000, LIVE_SILENCE_RMS_THRESHOLD),
            Some(3_000)
        );
    }

    #[test]
    fn append_live_segment_keeps_jsonl_chronological() {
        let path = env::temp_dir().join(format!(
            "just-notes-transcript-order-{}.jsonl",
            now_ms().unwrap()
        ));
        let later = TranscriptSegment {
            speaker: "Others".to_string(),
            source: "system".to_string(),
            start_ms: 4_000,
            end_ms: 8_000,
            text: "System section two.".to_string(),
        };
        let earlier = TranscriptSegment {
            speaker: "You".to_string(),
            source: "mic".to_string(),
            start_ms: 3_000,
            end_ms: 12_000,
            text: "Microphone checkpoint alpha.".to_string(),
        };

        append_live_segment(&path, &later).unwrap();
        append_live_segment(&path, &earlier).unwrap();

        let segments = read_transcript_jsonl(&path).unwrap();
        let _ = fs::remove_file(&path);
        assert_eq!(
            segments
                .iter()
                .map(|segment| segment.source.as_str())
                .collect::<Vec<_>>(),
            vec!["mic", "system"],
        );
    }
}
