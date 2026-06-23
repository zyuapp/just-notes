use tauri::AppHandle;

use crate::{
    app::AppPaths,
    settings::AppSettings,
    threads::repository::load_thread_by_id,
    transcription::{
        finalization_transcription_selection, spawn_finalization, wav_duration_ms,
        FinalizationAudioArtifacts, FinalizationConfig, FinalizationStart, FinalizeState,
    },
};

// A resumed thread's saved WAVs hold only the latest session, so re-transcribing
// and replacing would drop earlier sessions. Tolerate a little capture-startup
// latency before treating the audio as covering less than the whole thread.
const RESUME_AUDIO_TOLERANCE_MS: u64 = 5_000;

/// Re-runs the authoritative finalization pass on a thread's saved audio:
/// re-transcribes the WAVs, suppresses cross-channel bleed, and replaces the
/// live transcript. Available only when the raw audio was kept and the thread
/// is idle and single-session.
pub(crate) fn reprocess_thread(
    app: AppHandle,
    paths: AppPaths,
    settings: AppSettings,
    finalize: FinalizeState,
    thread_id: String,
) -> Result<(), String> {
    let thread = load_thread_by_id(&paths, &thread_id)?;
    if thread.summary.status.is_busy() {
        return Err("This thread is busy; stop recording or wait for it to finish.".to_string());
    }

    let thread_dir = paths.thread_dir(&thread_id);
    // Reprocessing keeps the audio it reads — it exists only because the user
    // chose to retain it.
    let audio_artifacts = FinalizationAudioArtifacts::for_thread_dir(&thread_dir, true);
    if !audio_artifacts.paths().has_any() {
        return Err("This thread has no saved audio to re-transcribe".to_string());
    }
    if thread.summary.duration_ms
        > audio_duration_ms(&audio_artifacts).saturating_add(RESUME_AUDIO_TOLERANCE_MS)
    {
        return Err("Re-transcribing isn't available for resumed recordings.".to_string());
    }

    let outcome = spawn_finalization(FinalizationConfig {
        app,
        state: finalize,
        thread_id,
        thread_dir,
        audio_artifacts,
        model_selection: finalization_transcription_selection(&paths),
        markdown_copy: settings.markdown_copy,
    })?;
    match outcome {
        FinalizationStart::Started => Ok(()),
        FinalizationStart::AlreadyRunning => {
            Err("This thread is already being processed".to_string())
        }
    }
}

fn audio_duration_ms(audio_artifacts: &FinalizationAudioArtifacts) -> u64 {
    let paths = audio_artifacts.paths();
    wav_duration_ms(paths.mic_path())
        .unwrap_or(0)
        .max(wav_duration_ms(paths.system_path()).unwrap_or(0))
}
