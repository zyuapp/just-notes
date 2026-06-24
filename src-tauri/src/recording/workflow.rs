use std::{path::PathBuf, sync::atomic::AtomicBool, sync::Arc, time::Instant};

use tauri::AppHandle;

use super::{
    audio_sink::spawn_audio_sink,
    meter::spawn_meter_thread,
    selection::{select_recording_thread, SelectedThread},
    state::{RecorderSession, RecorderState},
};
use crate::{
    app::AppPaths,
    capture::{prepare_audio_input, start_audio_capture, PreparedAudioInput, RecordingInputMode},
    ipc::RecordingPayload,
    settings::AppSettings,
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
}

pub(crate) fn start_recording(
    app: AppHandle,
    paths: AppPaths,
    recorder: RecorderState,
    settings: AppSettings,
    requested_thread_id: Option<String>,
) -> Result<RecordingPayload, String> {
    start_recording_with_mode(
        RecordingStart {
            app,
            paths,
            recorder,
            settings,
            requested_thread_id,
        },
        RecordingInputMode::Devices,
    )
}

#[cfg(any(debug_assertions, feature = "qa-fixtures"))]
pub(crate) fn start_fixture_recording(
    app: AppHandle,
    paths: AppPaths,
    recorder: RecorderState,
    settings: AppSettings,
    requested_thread_id: Option<String>,
) -> Result<RecordingPayload, String> {
    let fixture_dir = paths.data_dir.join("fixtures");
    start_recording_with_mode(
        RecordingStart {
            app,
            paths,
            recorder,
            settings,
            requested_thread_id,
        },
        RecordingInputMode::Fixture {
            mic_path: fixture_dir.join("qa-mic.wav"),
            system_path: fixture_dir.join("qa-system.wav"),
        },
    )
}

fn start_recording_with_mode(
    start: RecordingStart,
    input_mode: RecordingInputMode,
) -> Result<RecordingPayload, String> {
    let recorder = start.recorder.clone();
    let _starting = recorder.begin_starting()?;
    prepare_recording_session(start, input_mode)
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
    } = start;
    recorder.ensure_idle()?;
    paths.ensure()?;
    let transcription = transcription_status(&paths);
    if !transcription.ready {
        return Err("Parakeet model is required before recording".to_string());
    }

    let SelectedThread {
        thread,
        resume_offset_ms,
    } = select_recording_thread(&paths, requested_thread_id)?;
    let thread_id = thread.summary.id.clone();
    let thread_dir = paths.thread_dir(&thread_id);
    prepare_work_dir(&thread_dir)?;

    let started = Instant::now();
    let input = prepare_audio_input(&app, input_mode)?;
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
        let _ = set_thread_status(&thread_dir, ThreadStatus::Idle);
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
    let session = build_recording_session(config, audio_sink, audio_artifacts);
    recorder.store_session(session)
}

fn build_recording_session(
    config: RecordingSessionConfig,
    audio_sink: super::audio_sink::AudioSink,
    audio_artifacts: FinalizationAudioArtifacts,
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
