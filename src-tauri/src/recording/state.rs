use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::JoinHandle,
    time::Instant,
};

use super::audio_sink::AudioSink;
use crate::{
    app::AppPaths,
    capture::{ActiveAudioCapture, SharedBuffers},
    settings::AppSettings,
    threads::ThreadDetail,
    transcription::FinalizationAudioArtifacts,
};

#[derive(Clone, Default)]
pub(crate) struct RecorderState {
    session: Arc<Mutex<Option<RecorderSession>>>,
    is_starting: Arc<AtomicBool>,
}

pub(super) struct RecorderSession {
    pub(super) thread_id: String,
    pub(super) thread_dir: PathBuf,
    pub(super) started: Instant,
    // Paths and settings are captured at start so the session keeps writing to
    // the folder it was created in even if the user changes them mid-recording.
    pub(super) paths: AppPaths,
    pub(super) settings: AppSettings,
    pub(super) buffers: Arc<Mutex<SharedBuffers>>,
    pub(super) should_stop_meter: Arc<AtomicBool>,
    pub(super) meter_thread: Option<JoinHandle<()>>,
    pub(super) audio_capture: ActiveAudioCapture,
    pub(super) audio_sink: AudioSink,
    pub(super) audio_artifacts: FinalizationAudioArtifacts,
    // Prior recording length when resuming an existing thread; the new session's
    // transcript and duration are offset by it. None for a fresh recording.
    pub(super) resume_offset_ms: Option<u64>,
}

impl RecorderState {
    pub(super) fn begin_starting(&self) -> Result<StartingGuard<'_>, String> {
        if self.is_starting.swap(true, Ordering::SeqCst) {
            return Err("Audio startup is already in progress".to_string());
        }
        Ok(StartingGuard { recorder: self })
    }

    pub(super) fn ensure_not_starting(&self) -> Result<(), String> {
        if self.is_starting.load(Ordering::SeqCst) {
            return Err("Audio startup is still in progress".to_string());
        }
        Ok(())
    }

    pub(super) fn ensure_idle(&self) -> Result<(), String> {
        let session = self
            .session
            .lock()
            .map_err(|_| "Recorder state lock was poisoned".to_string())?;
        if session.is_some() {
            return Err("Recording is already active".to_string());
        }
        Ok(())
    }

    pub(super) fn store_session(&self, session: RecorderSession) -> Result<(), String> {
        let mut slot = self
            .session
            .lock()
            .map_err(|_| "Recorder state lock was poisoned".to_string())?;
        if slot.is_some() {
            return Err("Recording is already active".to_string());
        }

        *slot = Some(session);
        Ok(())
    }

    pub(super) fn take_session(&self) -> Result<RecorderSession, String> {
        let mut slot = self
            .session
            .lock()
            .map_err(|_| "Recorder state lock was poisoned".to_string())?;
        slot.take()
            .ok_or_else(|| "No recording is currently active".to_string())
    }

    pub(crate) fn is_active(&self) -> bool {
        self.session
            .lock()
            .map(|session| session.is_some())
            .unwrap_or(false)
    }
}

pub(super) struct StartingGuard<'a> {
    recorder: &'a RecorderState,
}

impl Drop for StartingGuard<'_> {
    fn drop(&mut self) {
        self.recorder.is_starting.store(false, Ordering::SeqCst);
    }
}

pub(super) enum ThreadSelection {
    /// Empty existing thread: record into it as if new.
    Reuse(Box<ThreadDetail>),
    /// Existing thread with content: append a new session to it.
    Resume(Box<ThreadDetail>),
}

pub(super) fn classify_selected_thread(thread: ThreadDetail) -> Result<ThreadSelection, String> {
    if thread.summary.status.is_busy() {
        return Err("The selected thread is busy recording or transcribing".to_string());
    }
    if thread.summary.segment_count == 0 {
        return Ok(ThreadSelection::Reuse(Box::new(thread)));
    }
    Ok(ThreadSelection::Resume(Box::new(thread)))
}
