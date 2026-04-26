use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::JoinHandle,
    time::Instant,
};

use crate::{
    capture::{ActiveAudioCapture, SharedBuffers},
    threads::ThreadDetail,
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
    pub(super) buffers: Arc<Mutex<SharedBuffers>>,
    pub(super) should_stop_meter: Arc<AtomicBool>,
    pub(super) should_stop_live_transcription: Arc<AtomicBool>,
    pub(super) meter_thread: Option<JoinHandle<()>>,
    pub(super) live_transcription_thread: Option<JoinHandle<()>>,
    pub(super) audio_capture: ActiveAudioCapture,
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
}

pub(super) struct StartingGuard<'a> {
    recorder: &'a RecorderState,
}

impl Drop for StartingGuard<'_> {
    fn drop(&mut self) {
        self.recorder.is_starting.store(false, Ordering::SeqCst);
    }
}

pub(super) fn selected_thread_is_reusable(thread: &ThreadDetail) -> bool {
    !thread.summary.status.is_recording() && thread.summary.segment_count == 0
}
