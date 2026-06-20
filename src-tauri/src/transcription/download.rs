use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;

mod install;
mod snapshot;

use crate::app::AppPaths;

use super::{
    delete_parakeet_model, parakeet_artifact, ModelArtifact, TranscriptionModelDownloadState,
};
pub(crate) use snapshot::DownloadSnapshot;

type DownloadSnapshots = Arc<Mutex<Option<DownloadSnapshot>>>;
type DownloadCancellation = Arc<Mutex<Option<Arc<AtomicBool>>>>;

/// Invoked on the download worker thread after the model installs successfully.
/// The download flow stays unaware of settings; the caller decides what a
/// completed install means.
pub(crate) type OnInstalled = Box<dyn FnOnce() + Send + 'static>;

#[derive(Clone, Default)]
pub(crate) struct ModelDownloadState {
    inner: DownloadSnapshots,
    cancellation: DownloadCancellation,
}

struct DownloadJob {
    artifact: ModelArtifact,
    paths: AppPaths,
    on_installed: OnInstalled,
    cancellation: Arc<AtomicBool>,
}

impl ModelDownloadState {
    pub(crate) fn snapshot_for(&self, total_bytes: u64) -> DownloadSnapshot {
        self.inner
            .lock()
            .ok()
            .and_then(|snapshot| snapshot.clone())
            .unwrap_or_else(|| DownloadSnapshot::idle(total_bytes))
    }

    pub(crate) fn start_download(
        &self,
        paths: AppPaths,
        on_installed: OnInstalled,
    ) -> Result<(), String> {
        let artifact = parakeet_artifact();
        if artifact
            .expected_files(&paths)
            .iter()
            .all(|path| path.is_file())
        {
            self.set_snapshot(DownloadSnapshot::idle(artifact.archive_bytes));
            return Ok(());
        }

        let cancellation = Arc::new(AtomicBool::new(false));
        self.mark_download_started(artifact, &cancellation)?;
        self.spawn_download_thread(DownloadJob {
            artifact,
            paths,
            on_installed,
            cancellation,
        });
        Ok(())
    }

    pub(crate) fn cancel_download(&self) -> bool {
        let cancelled = self
            .cancellation
            .lock()
            .ok()
            .and_then(|cancellation| cancellation.clone())
            .map(|flag| {
                flag.store(true, Ordering::SeqCst);
                true
            })
            .unwrap_or(false);
        if cancelled {
            let total_bytes = self.snapshot_for(0).total_bytes;
            self.set_snapshot(DownloadSnapshot::new(
                TranscriptionModelDownloadState::Cancelled,
                0,
                total_bytes,
                None,
            ));
        }
        cancelled
    }

    pub(crate) fn download_running(&self) -> bool {
        self.cancellation
            .lock()
            .map(|cancellation| cancellation.is_some())
            .unwrap_or(false)
    }

    fn mark_download_started(
        &self,
        artifact: ModelArtifact,
        cancellation: &Arc<AtomicBool>,
    ) -> Result<(), String> {
        self.ensure_not_running()?;
        let mut snapshot = self
            .inner
            .lock()
            .map_err(|_| "Model download state is unavailable".to_string())?;
        if snapshot.as_ref().is_some_and(DownloadSnapshot::active) {
            return already_running();
        }
        *snapshot = Some(DownloadSnapshot::new(
            TranscriptionModelDownloadState::Downloading,
            0,
            artifact.archive_bytes,
            None,
        ));
        if let Ok(mut cancellation_slot) = self.cancellation.lock() {
            *cancellation_slot = Some(Arc::clone(cancellation));
        }
        Ok(())
    }

    fn ensure_not_running(&self) -> Result<(), String> {
        if self.download_running() {
            return already_running();
        }
        Ok(())
    }

    fn spawn_download_thread(&self, job: DownloadJob) {
        let state = self.clone();
        thread::spawn(move || {
            let result =
                install::install_model(job.artifact, &job.paths, &state, &job.cancellation);
            state.clear_cancellation();
            state.finish_download(job, result);
        });
    }

    fn clear_cancellation(&self) {
        if let Ok(mut cancellation) = self.cancellation.lock() {
            *cancellation = None;
        }
    }

    fn finish_download(&self, job: DownloadJob, result: Result<(), install::DownloadError>) {
        match result {
            Ok(()) => {
                (job.on_installed)();
                self.set_snapshot(DownloadSnapshot::idle(job.artifact.archive_bytes));
            }
            Err(install::DownloadError::Cancelled) => self.set_snapshot(DownloadSnapshot::new(
                TranscriptionModelDownloadState::Cancelled,
                0,
                job.artifact.archive_bytes,
                None,
            )),
            Err(install::DownloadError::Failed(message)) => {
                self.set_snapshot(DownloadSnapshot::new(
                    TranscriptionModelDownloadState::Failed,
                    0,
                    job.artifact.archive_bytes,
                    Some(message),
                ))
            }
        }
    }

    pub(super) fn set_snapshot(&self, snapshot: DownloadSnapshot) {
        if let Ok(mut slot) = self.inner.lock() {
            *slot = Some(snapshot);
        }
    }

    pub(crate) fn reset_to_idle(&self) {
        self.set_snapshot(DownloadSnapshot::idle(parakeet_artifact().archive_bytes));
    }

    pub(crate) fn delete_model(&self, paths: &AppPaths) -> Result<(), String> {
        // Refuse mid-download: removing model files would race the installer thread.
        if self.download_running() {
            return Err("Finish or cancel the download before deleting the model".to_string());
        }
        delete_parakeet_model(paths)?;
        self.reset_to_idle();
        Ok(())
    }
}

fn already_running() -> Result<(), String> {
    Err("Parakeet model download is already running".to_string())
}

#[cfg(test)]
mod tests;
