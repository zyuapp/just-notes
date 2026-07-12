use std::{path::PathBuf, sync::atomic::AtomicBool, sync::Arc, time::Instant};

use tauri::{AppHandle, Manager};

use super::{
    audio_sink::spawn_audio_sink,
    meter::spawn_meter_thread,
    selection::{select_recording_thread, SelectedThread},
    state::{RecorderSession, RecorderState},
};
use crate::{
    app::{AppPaths, StorageGate},
    capture::{prepare_audio_input, start_audio_capture, PreparedAudioInput, RecordingInputMode},
    ipc::RecordingPayload,
    settings::{self, AppSettings, SettingsState},
    threads::{
        repository::{load_thread_by_id, prepare_work_dir, set_thread_status},
        ThreadStatus,
    },
    transcription::{
        finalization_transcription_selection, spawn_live_transcription, transcription_status,
        FinalizationAudioArtifacts, LiveTranscriptionConfig,
    },
    tray,
};

struct RecordingSessionConfig {
    app: AppHandle,
    thread_id: String,
    thread_dir: PathBuf,
    started: Instant,
    input: PreparedAudioInput,
    paths: AppPaths,
    settings: AppSettings,
    resume_offset_ms: Option<u64>,
}

struct RecordingStart {
    app: AppHandle,
    paths: AppPaths,
    recorder: RecorderState,
    settings: AppSettings,
    requested_thread_id: Option<String>,
    new_thread_title: Option<String>,
}

pub(super) struct RecordingRequest {
    pub(super) app: AppHandle,
    pub(super) base_paths: AppPaths,
    pub(super) recorder: RecorderState,
    pub(super) settings_state: SettingsState,
    pub(super) requested_thread_id: Option<String>,
    pub(super) new_thread_title: Option<String>,
    pub(super) input_mode: RecordingInputMode,
}

pub(super) fn start_recording_with_mode(
    request: RecordingRequest,
) -> Result<RecordingPayload, String> {
    let starting_recorder = request.recorder.clone();
    let gate = request.app.state::<StorageGate>();
    let _storage_guard = gate.lock()?;
    let _starting = starting_recorder.begin_starting()?;
    let settings = request.settings_state.snapshot();
    let start = RecordingStart {
        app: request.app.clone(),
        paths: settings::effective_paths(&request.base_paths, &settings),
        recorder: request.recorder,
        settings,
        requested_thread_id: request.requested_thread_id,
        new_thread_title: request.new_thread_title,
    };
    prepare_recording_session(start, request.input_mode)
}

fn prepare_recording_session(
    start: RecordingStart,
    input_mode: RecordingInputMode,
) -> Result<RecordingPayload, String> {
    let RecordingStart {
        app,
        paths,
        recorder,
        settings,
        requested_thread_id,
        new_thread_title,
    } = start;
    recorder.ensure_idle()?;
    paths.ensure()?;
    let transcription = transcription_status(&paths);
    if !transcription.ready {
        return Err("Parakeet model is required before recording".to_string());
    }

    // Prepare permissions and devices before allocating a new thread. A failed
    // calendar-notification retry should not leave an empty meeting note behind.
    let input = prepare_audio_input(&app, input_mode)?;

    let SelectedThread {
        thread,
        resume_offset_ms,
        newly_created,
    } = select_recording_thread(&paths, requested_thread_id, new_thread_title.as_deref())?;
    let thread_id = thread.summary.id.clone();
    let thread_dir = paths.thread_dir(&thread_id);
    prepare_work_dir(&thread_dir)?;

    let started = Instant::now();
    set_thread_status(&thread_dir, ThreadStatus::Recording)?;

    let config = RecordingSessionConfig {
        app: app.clone(),
        thread_id: thread_id.clone(),
        thread_dir: thread_dir.clone(),
        started,
        input,
        paths: paths.clone(),
        settings,
        resume_offset_ms,
    };
    if let Err(err) = activate_session(&recorder, config) {
        if newly_created {
            crate::threads::create::discard_failed_thread(&thread_dir);
        } else {
            let _ = set_thread_status(&thread_dir, ThreadStatus::Idle);
        }
        return Err(err);
    }
    tray::set_tray_recording(&app, true);

    Ok(RecordingPayload {
        thread: load_thread_by_id(&paths, &thread_id)?,
        transcription,
    })
}

fn activate_session(
    recorder: &RecorderState,
    mut config: RecordingSessionConfig,
) -> Result<(), String> {
    let audio_artifacts = FinalizationAudioArtifacts::for_thread_dir(
        &config.thread_dir,
        config.settings.save_raw_audio,
    );
    let audio_sink = spawn_audio_sink(
        audio_artifacts.paths(),
        Arc::clone(&config.input.buffers),
        config.input.mic_sample_rate,
        config.input.system_sample_rate,
    )?;
    if let Err(err) = start_audio_capture(&mut config.input.audio_capture) {
        let _ = audio_sink.stop();
        let _ = audio_artifacts.cleanup_if_transient();
        return Err(err);
    }
    let session_id = recorder.allocate_session_id();
    let session = build_recording_session(config, audio_sink, audio_artifacts, session_id);
    recorder.store_session(session)
}

fn build_recording_session(
    config: RecordingSessionConfig,
    audio_sink: super::audio_sink::AudioSink,
    audio_artifacts: FinalizationAudioArtifacts,
    session_id: u64,
) -> RecorderSession {
    let input = config.input;
    let should_stop_meter = Arc::new(AtomicBool::new(false));
    let meter_thread = spawn_meter_thread(
        config.app.clone(),
        config.thread_id.clone(),
        Arc::clone(&input.buffers),
        Arc::clone(&should_stop_meter),
        config.started,
    );
    let live_transcription = spawn_live_transcription(LiveTranscriptionConfig {
        app: config.app.clone(),
        thread_id: config.thread_id.clone(),
        thread_dir: config.thread_dir.clone(),
        buffers: Arc::clone(&input.buffers),
        model_selection: finalization_transcription_selection(&config.paths),
        mic_sample_rate: input.mic_sample_rate,
        system_sample_rate: input.system_sample_rate,
        offset_ms: config.resume_offset_ms.unwrap_or(0),
    });

    RecorderSession {
        session_id,
        thread_id: config.thread_id,
        thread_dir: config.thread_dir,
        started: config.started,
        paths: config.paths,
        settings: config.settings,
        buffers: input.buffers,
        should_stop_meter,
        meter_thread: Some(meter_thread),
        audio_capture: input.audio_capture,
        audio_sink,
        live_transcription,
        audio_artifacts,
        resume_offset_ms: config.resume_offset_ms,
    }
}
