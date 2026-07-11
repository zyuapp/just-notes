use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use tauri::{AppHandle, Emitter};

use super::finalize_audio::{transcribe_wav_channel, FinalizationAudioArtifacts};
use super::{
    load_transcriber, suppress_cross_channel_bleed, suppress_system_dominated_mic_segments,
    ChannelRole, SegmenterConfig, TranscriptionModelSelection,
};
use crate::{
    ipc::FinalizationStatusPayload,
    threads::{commit::commit_transcript, repository::set_thread_status, ThreadStatus},
};

type FinalizeJobs = Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>;

#[derive(Clone, Default)]
pub(crate) struct FinalizeState(FinalizeJobs);

impl FinalizeState {
    fn begin(&self, thread_id: &str) -> Option<Arc<AtomicBool>> {
        let mut jobs = self.0.lock().ok()?;
        if jobs.contains_key(thread_id) {
            return None;
        }
        let cancel = Arc::new(AtomicBool::new(false));
        jobs.insert(thread_id.to_string(), Arc::clone(&cancel));
        Some(cancel)
    }

    fn finish(&self, thread_id: &str) {
        if let Ok(mut jobs) = self.0.lock() {
            jobs.remove(thread_id);
        }
    }

    pub(crate) fn cancel(&self, thread_id: &str) -> bool {
        let Ok(jobs) = self.0.lock() else {
            return false;
        };
        match jobs.get(thread_id) {
            Some(flag) => {
                flag.store(true, Ordering::Relaxed);
                true
            }
            None => false,
        }
    }

    pub(crate) fn is_active(&self) -> bool {
        self.0.lock().map(|jobs| !jobs.is_empty()).unwrap_or(false)
    }

    pub(crate) fn wait_for_idle(&self) {
        while self.is_active() {
            thread::sleep(Duration::from_millis(100));
        }
    }
}

pub(crate) struct FinalizationConfig {
    pub(crate) app: AppHandle,
    pub(crate) state: FinalizeState,
    pub(crate) thread_id: String,
    pub(crate) thread_dir: PathBuf,
    pub(crate) audio_artifacts: FinalizationAudioArtifacts,
    pub(crate) model_selection: TranscriptionModelSelection,
    pub(crate) markdown_copy: bool,
}

pub(crate) enum FinalizationStart {
    Started,
    AlreadyRunning,
}

enum FinalizationOutcome {
    Completed,
    Cancelled,
    Empty,
}

type FinalizationBeginResult = Result<Option<Arc<AtomicBool>>, String>;

pub(crate) fn spawn_finalization(config: FinalizationConfig) -> Result<FinalizationStart, String> {
    let Some(cancel) = prepare_finalization(&config)? else {
        return Ok(FinalizationStart::AlreadyRunning);
    };
    emit_finalization_status(
        &config.app,
        &config.thread_id,
        "running",
        "Improving the transcript from the saved recording",
    );
    thread::spawn(move || finish_finalization(config, cancel));
    Ok(FinalizationStart::Started)
}

fn prepare_finalization(config: &FinalizationConfig) -> FinalizationBeginResult {
    if !config.model_selection.is_installed() {
        return Err("Local transcription model is not installed".to_string());
    }
    if !config.audio_artifacts.paths().has_any() {
        return Err("No recording audio was captured for final transcription".to_string());
    }
    let Some(cancel) = config.state.begin(&config.thread_id) else {
        return Ok(None);
    };
    if set_thread_status(&config.thread_dir, ThreadStatus::Transcribing).is_err() {
        config.state.finish(&config.thread_id);
        return Err("Failed to mark thread as transcribing".to_string());
    }
    Ok(Some(cancel))
}

fn finish_finalization(config: FinalizationConfig, cancel: Arc<AtomicBool>) {
    let outcome = run_finalization_guarded(&config, &cancel);
    let outcome = cleanup_completed_finalization(&config, outcome);
    let _ = set_thread_status(&config.thread_dir, ThreadStatus::Idle);
    config.state.finish(&config.thread_id);
    emit_finalization_outcome(&config, outcome);
}

fn run_finalization_guarded(
    config: &FinalizationConfig,
    cancel: &AtomicBool,
) -> Result<FinalizationOutcome, String> {
    // catch_unwind keeps a decoder panic from leaking the
    // Transcribing status and the registry entry for this thread.
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_finalization(
            config,
            cancel,
            config.audio_artifacts.paths().mic_path(),
            config.audio_artifacts.paths().system_path(),
        )
    }))
    .unwrap_or_else(|_| Err("Transcript finalization crashed".to_string()))
}

fn cleanup_completed_finalization(
    config: &FinalizationConfig,
    outcome: Result<FinalizationOutcome, String>,
) -> Result<FinalizationOutcome, String> {
    match outcome {
        Ok(FinalizationOutcome::Completed) => config
            .audio_artifacts
            .cleanup_if_transient()
            .map(|()| FinalizationOutcome::Completed),
        other => other,
    }
}

fn emit_finalization_outcome(
    config: &FinalizationConfig,
    outcome: Result<FinalizationOutcome, String>,
) {
    match outcome {
        Ok(FinalizationOutcome::Completed) => emit_finalization_status(
            &config.app,
            &config.thread_id,
            "done",
            "The polished transcript is ready",
        ),
        Ok(FinalizationOutcome::Cancelled | FinalizationOutcome::Empty) => {
            emit_finalization_status(
                &config.app,
                &config.thread_id,
                "cancelled",
                "No final transcript was produced",
            )
        }
        Err(err) => emit_finalization_status(&config.app, &config.thread_id, "failed", &err),
    }
}

fn run_finalization(
    config: &FinalizationConfig,
    cancel: &AtomicBool,
    mic_path: &Path,
    system_path: &Path,
) -> Result<FinalizationOutcome, String> {
    let transcriber = load_transcriber(&config.model_selection)?;
    let mut segments = transcribe_wav_channel(
        &*transcriber,
        mic_path,
        ChannelRole {
            source: "mic",
            speaker: "You",
        },
        cancel,
        SegmenterConfig::finalize(),
    )?;
    segments.extend(transcribe_wav_channel(
        &*transcriber,
        system_path,
        ChannelRole {
            source: "system",
            speaker: "Others",
        },
        cancel,
        SegmenterConfig::finalize(),
    )?);
    if cancel.load(Ordering::Relaxed) {
        return Ok(FinalizationOutcome::Cancelled);
    }

    segments.sort_by(|left, right| {
        left.start_ms
            .cmp(&right.start_ms)
            .then_with(|| left.end_ms.cmp(&right.end_ms))
            .then_with(|| left.source.cmp(&right.source))
    });
    let segments = suppress_cross_channel_bleed(segments);
    let segments = suppress_system_dominated_mic_segments(segments, mic_path, system_path)?;
    if segments.is_empty() {
        return Ok(FinalizationOutcome::Empty);
    }

    commit_transcript(&config.thread_dir, &segments, config.markdown_copy)?;
    Ok(FinalizationOutcome::Completed)
}

fn emit_finalization_status(app: &AppHandle, thread_id: &str, state: &str, message: &str) {
    let _ = app.emit(
        "finalization-status",
        FinalizationStatusPayload {
            thread_id: thread_id.to_string(),
            state: state.to_string(),
            message: message.to_string(),
        },
    );
}
