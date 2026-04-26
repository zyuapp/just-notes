use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};

use tauri::AppHandle;

use super::{
    meter::spawn_meter_thread,
    state::{selected_thread_is_reusable, RecorderSession, RecorderState},
};
use crate::{
    app::AppPaths,
    capture::{
        prepare_audio_input, start_audio_capture, stop_audio_capture, PreparedAudioInput,
        RecordingInputMode,
    },
    ipc::RecordingPayload,
    threads::{
        repository::{
            create_thread as create_thread_record, load_thread_by_id, prepare_work_dir,
            render_thread_markdown, set_thread_status,
        },
        ThreadDetail, ThreadStatus,
    },
    transcription::{
        spawn_live_transcription_thread, transcription_paths, transcription_status,
        LiveTranscriptionThreadConfig, TranscriptionPaths,
    },
};

struct RecordingSessionConfig {
    app: AppHandle,
    thread_id: String,
    thread_dir: PathBuf,
    started: Instant,
    input: PreparedAudioInput,
    transcription_paths: TranscriptionPaths,
}

pub(crate) fn start_recording(
    app: AppHandle,
    paths: AppPaths,
    recorder: RecorderState,
    requested_thread_id: Option<String>,
) -> Result<RecordingPayload, String> {
    start_recording_with_mode(
        app,
        paths,
        recorder,
        requested_thread_id,
        RecordingInputMode::Devices,
    )
}

#[cfg(any(debug_assertions, feature = "qa-fixtures"))]
pub(crate) fn start_fixture_recording(
    app: AppHandle,
    paths: AppPaths,
    recorder: RecorderState,
    requested_thread_id: Option<String>,
) -> Result<RecordingPayload, String> {
    let fixture_dir = paths.data_dir.join("fixtures");
    start_recording_with_mode(
        app,
        paths,
        recorder,
        requested_thread_id,
        RecordingInputMode::Fixture {
            mic_path: fixture_dir.join("qa-mic.wav"),
            system_path: fixture_dir.join("qa-system.wav"),
        },
    )
}

pub(crate) fn stop_recording(
    paths: AppPaths,
    recorder: RecorderState,
) -> Result<ThreadDetail, String> {
    recorder.ensure_not_starting()?;
    let session = recorder.take_session()?;

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

fn start_recording_with_mode(
    app: AppHandle,
    paths: AppPaths,
    recorder: RecorderState,
    requested_thread_id: Option<String>,
    input_mode: RecordingInputMode,
) -> Result<RecordingPayload, String> {
    let _starting = recorder.begin_starting()?;
    prepare_recording_session(app, paths, &recorder, requested_thread_id, input_mode)
}

fn prepare_recording_session(
    app: AppHandle,
    paths: AppPaths,
    recorder: &RecorderState,
    requested_thread_id: Option<String>,
    input_mode: RecordingInputMode,
) -> Result<RecordingPayload, String> {
    recorder.ensure_idle()?;
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
        transcription_paths: transcription_paths(&paths),
    });
    recorder.store_session(session)?;

    Ok(RecordingPayload {
        thread: load_thread_by_id(&paths, &thread_id)?,
        transcription,
    })
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

fn select_recording_thread(
    paths: &AppPaths,
    requested_thread_id: Option<String>,
) -> Result<ThreadDetail, String> {
    let Some(thread_id) = requested_thread_id else {
        return create_thread_record(paths);
    };

    let thread = load_thread_by_id(paths, &thread_id)?;
    if selected_thread_is_reusable(&thread) {
        return Ok(thread);
    }
    if thread.summary.status.is_recording() {
        return Err("The selected thread is already recording".to_string());
    }

    create_thread_record(paths)
}
