use std::sync::atomic::Ordering;

use tauri::{AppHandle, Emitter};

use super::state::{RecorderSession, RecorderState};
use crate::{
    capture::stop_audio_capture,
    threads::{
        repository::{
            load_thread_by_id, render_thread_markdown, set_thread_duration, set_thread_status,
        },
        ThreadDetail, ThreadStatus,
    },
    transcription::{spawn_finalization, transcription_paths, FinalizationConfig, FinalizeState},
    tray,
};

pub(crate) fn stop_recording(
    app: AppHandle,
    recorder: RecorderState,
    finalize: FinalizeState,
) -> Result<ThreadDetail, String> {
    recorder.ensure_not_starting()?;
    let session = recorder.take_session()?;

    let RecorderSession {
        thread_id,
        thread_dir,
        started,
        paths,
        settings,
        buffers,
        should_stop_meter,
        should_stop_live_transcription,
        mut meter_thread,
        mut live_transcription_thread,
        audio_capture,
        audio_sink,
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
    if let Some(sink) = audio_sink {
        if let Err(err) = sink.stop() {
            eprintln!("audio sink error: {err}");
        }
    }
    drop(buffers);

    let duration_ms = started.elapsed().as_millis() as u64;
    set_thread_duration(&thread_dir, duration_ms)?;
    set_thread_status(&thread_dir, ThreadStatus::Idle)?;
    if settings.markdown_copy {
        render_thread_markdown(&thread_dir)?;
    }
    tray::set_tray_recording(&app, false);

    spawn_finalization(FinalizationConfig {
        app: app.clone(),
        state: finalize,
        thread_id: thread_id.clone(),
        thread_dir,
        paths: transcription_paths(&paths),
        markdown_copy: settings.markdown_copy,
    });

    let detail = load_thread_by_id(&paths, &thread_id)?;
    let _ = app.emit("recording-stopped", &detail);
    Ok(detail)
}
