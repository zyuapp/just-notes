use std::{path::Path, sync::atomic::Ordering};

use tauri::{AppHandle, Emitter, Manager};

use super::state::{RecorderSession, RecorderState};
use crate::{
    app::StorageGate,
    capture::stop_audio_capture,
    threads::{
        repository::{
            load_thread_by_id, render_thread_markdown, set_thread_duration, set_thread_status,
        },
        ThreadDetail, ThreadStatus,
    },
    tray,
};

pub(crate) fn stop_recording(
    app: AppHandle,
    recorder: RecorderState,
) -> Result<ThreadDetail, String> {
    let gate = app.state::<StorageGate>().inner().clone();
    gate.run(|| stop_recording_inner(app, recorder))
}

fn stop_recording_inner(app: AppHandle, recorder: RecorderState) -> Result<ThreadDetail, String> {
    recorder.ensure_not_starting()?;
    let session = recorder.take_session()?;

    let RecorderSession {
        session_id: _,
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
        live_transcription,
        audio_artifacts,
        resume_offset_ms,
    } = session;

    should_stop_meter.store(true, Ordering::Relaxed);
    if let Some(thread) = meter_thread.take() {
        let _ = thread.join();
    }
    stop_audio_capture(audio_capture);
    // Live transcription has been persisting each utterance as it lands; stopping
    // it flushes the final tail, leaving an authoritative transcript on disk with
    // no separate re-transcription pass.
    live_transcription.stop();
    let audio_sink_result = audio_sink.stop();
    drop(buffers);

    // Capture is finished, so the tray must leave the recording state even if
    // persisting below fails.
    tray::set_tray_recording(&app, false);

    // The live worker publishes per-channel segments with no cross-channel
    // filtering, so suppress speaker bleed once while the WAVs still exist.
    // Best-effort: an unpolished transcript never blocks the stop.
    let _ = crate::transcription::polish_thread_transcript(
        &thread_dir,
        audio_artifacts.paths(),
        resume_offset_ms.unwrap_or(0),
    );

    // Duration from the recorded audio rather than wall-clock, which over-counts
    // by the capture startup latency; fall back to elapsed time only when the
    // WAVs cannot be measured.
    let session_ms = audio_artifacts
        .measured_duration_ms()
        .unwrap_or_else(|| started.elapsed().as_millis() as u64);
    let duration_ms = resume_offset_ms.unwrap_or(0).saturating_add(session_ms);
    let persist_result = persist_stopped_thread(&thread_dir, duration_ms, settings.markdown_copy);

    // No finalization pass owns raw-audio cleanup now, so honor the retention
    // policy here even if persisting failed: drop the WAVs unless the user opted
    // to keep them. Best-effort, so a leftover file never blocks the stop.
    let _ = audio_artifacts.cleanup_if_transient();
    // A failed audio-file finalize leaves a possibly-truncated WAV, but the
    // transcript was persisted live and is unaffected.
    let _ = audio_sink_result;
    persist_result?;

    let detail = load_thread_by_id(&paths, &thread_id)?;
    let _ = app.emit("recording-stopped", &detail);
    Ok(detail)
}

fn persist_stopped_thread(
    thread_dir: &Path,
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
