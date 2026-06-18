use std::sync::atomic::Ordering;

use tauri::{AppHandle, Emitter};

use super::state::{RecorderSession, RecorderState};
use crate::{
    capture::stop_audio_capture,
    indicator,
    threads::{
        repository::{
            load_thread_by_id, render_thread_markdown, set_thread_duration, set_thread_status,
        },
        ThreadDetail, ThreadStatus,
    },
    transcription::{
        emit_finalization_failure, finalization_transcription_paths, spawn_finalization,
        FinalizationConfig, FinalizationStart, FinalizeState,
    },
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
        mut meter_thread,
        audio_capture,
        audio_sink,
        audio_artifacts,
    } = session;

    should_stop_meter.store(true, Ordering::Relaxed);
    if let Some(thread) = meter_thread.take() {
        let _ = thread.join();
    }
    stop_audio_capture(audio_capture);
    let audio_sink_result = audio_sink.stop();
    drop(buffers);

    // Capture is finished at this point, so the tray and indicator must leave
    // the recording state even if persisting the thread below fails.
    tray::set_tray_recording(&app, false);
    indicator::set_indicator_recording(&app, false);

    let audio_artifacts_for_failure = audio_artifacts.clone();
    let duration_ms = started.elapsed().as_millis() as u64;
    if let Err(err) = persist_stopped_thread(&thread_dir, duration_ms, settings.markdown_copy) {
        let mut message = format!("Failed to finish recording metadata: {err}");
        append_transient_audio_cleanup_error(&mut message, &audio_artifacts_for_failure);
        emit_finalization_failure(&app, &thread_id, &message);
        return Err(err);
    }

    match audio_sink_result {
        Ok(()) => {
            match spawn_finalization(FinalizationConfig {
                app: app.clone(),
                state: finalize,
                thread_id: thread_id.clone(),
                thread_dir,
                audio_artifacts: audio_artifacts.clone(),
                paths: finalization_transcription_paths(&paths),
                markdown_copy: settings.markdown_copy,
            }) {
                Ok(FinalizationStart::Started) => {}
                Ok(FinalizationStart::AlreadyRunning) => {
                    let mut message = "Transcript finalization is already running".to_string();
                    append_transient_audio_cleanup_error(&mut message, &audio_artifacts);
                    emit_finalization_failure(&app, &thread_id, &message);
                }
                Err(err) => {
                    let mut message = err;
                    append_transient_audio_cleanup_error(&mut message, &audio_artifacts);
                    emit_finalization_failure(&app, &thread_id, &message);
                }
            }
        }
        Err(err) => {
            let mut message = format!("Failed to finish recording audio: {err}");
            append_transient_audio_cleanup_error(&mut message, &audio_artifacts);
            emit_finalization_failure(&app, &thread_id, &message);
        }
    }

    let detail = load_thread_by_id(&paths, &thread_id)?;
    let _ = app.emit("recording-stopped", &detail);
    Ok(detail)
}

fn persist_stopped_thread(
    thread_dir: &std::path::Path,
    duration_ms: u64,
    markdown_copy: bool,
) -> Result<(), String> {
    set_thread_duration(thread_dir, duration_ms)?;
    set_thread_status(thread_dir, ThreadStatus::Idle)?;
    if markdown_copy {
        render_thread_markdown(thread_dir)?;
    }
    Ok(())
}

fn append_transient_audio_cleanup_error(
    message: &mut String,
    audio_artifacts: &crate::transcription::FinalizationAudioArtifacts,
) {
    if let Err(cleanup_err) = audio_artifacts.cleanup_if_transient() {
        message.push_str(&format!("; also failed to remove raw audio: {cleanup_err}"));
    }
}
