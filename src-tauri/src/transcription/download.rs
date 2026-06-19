use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
};

mod install;

use crate::app::AppPaths;

use super::{
    artifact_for_provider, ModelArtifact, TranscriptionModelDownloadState, TranscriptionProvider,
};

type DownloadSnapshots = Arc<Mutex<HashMap<TranscriptionProvider, DownloadSnapshot>>>;
type DownloadCancellations = Arc<Mutex<HashMap<TranscriptionProvider, Arc<AtomicBool>>>>;

/// Invoked on the download worker thread after a model installs successfully.
/// The download flow stays unaware of settings; the caller decides what a
/// completed install means (e.g. switching the active provider preference).
pub(crate) type OnInstalled = Box<dyn FnOnce() + Send + 'static>;

#[derive(Clone, Default)]
pub(crate) struct ModelDownloadState {
    inner: DownloadSnapshots,
    cancellations: DownloadCancellations,
}

struct DownloadJob {
    provider: TranscriptionProvider,
    artifact: ModelArtifact,
    paths: AppPaths,
    on_installed: OnInstalled,
    cancellation: Arc<AtomicBool>,
}

#[derive(Clone)]
pub(crate) struct DownloadSnapshot {
    pub(crate) state: TranscriptionModelDownloadState,
    pub(crate) progress_bytes: u64,
    pub(crate) total_bytes: u64,
    pub(crate) error_message: Option<String>,
}

impl DownloadSnapshot {
    pub(super) fn new(
        state: TranscriptionModelDownloadState,
        progress_bytes: u64,
        total_bytes: u64,
        error_message: Option<String>,
    ) -> Self {
        Self {
            state,
            progress_bytes,
            total_bytes,
            error_message,
        }
    }

    fn idle(total_bytes: u64) -> Self {
        Self::new(TranscriptionModelDownloadState::Idle, 0, total_bytes, None)
    }

    fn active(&self) -> bool {
        matches!(
            self.state,
            TranscriptionModelDownloadState::Downloading
                | TranscriptionModelDownloadState::Installing
        )
    }
}

impl ModelDownloadState {
    pub(crate) fn snapshot_for(
        &self,
        provider: TranscriptionProvider,
        total_bytes: u64,
    ) -> DownloadSnapshot {
        self.inner
            .lock()
            .ok()
            .and_then(|downloads| downloads.get(&provider).cloned())
            .unwrap_or_else(|| DownloadSnapshot::idle(total_bytes))
    }

    pub(crate) fn start_download(
        &self,
        provider: TranscriptionProvider,
        paths: AppPaths,
        on_installed: OnInstalled,
    ) -> Result<(), String> {
        let artifact = artifact_for_provider(provider)
            .ok_or_else(|| format!("{} is not downloadable", provider.display_name()))?;
        if artifact
            .expected_files(&paths)
            .iter()
            .all(|path| path.is_file())
        {
            self.set_snapshot(provider, DownloadSnapshot::idle(artifact.archive_bytes));
            return Ok(());
        }

        let cancellation = Arc::new(AtomicBool::new(false));
        self.mark_download_started(provider, artifact, &cancellation)?;
        self.spawn_download_thread(DownloadJob {
            provider,
            artifact,
            paths,
            on_installed,
            cancellation,
        });
        Ok(())
    }

    pub(crate) fn cancel_download(&self, provider: TranscriptionProvider) -> bool {
        let cancelled = self
            .cancellations
            .lock()
            .ok()
            .and_then(|cancellations| cancellations.get(&provider).cloned())
            .map(|flag| {
                flag.store(true, Ordering::SeqCst);
                true
            })
            .unwrap_or(false);
        if cancelled {
            let total_bytes = self.snapshot_for(provider, 0).total_bytes;
            self.set_snapshot(
                provider,
                DownloadSnapshot::new(
                    TranscriptionModelDownloadState::Cancelled,
                    0,
                    total_bytes,
                    None,
                ),
            );
        }
        cancelled
    }

    pub(crate) fn download_running(&self, provider: TranscriptionProvider) -> bool {
        self.cancellations
            .lock()
            .map(|cancellations| cancellations.contains_key(&provider))
            .unwrap_or(false)
    }

    fn mark_download_started(
        &self,
        provider: TranscriptionProvider,
        artifact: ModelArtifact,
        cancellation: &Arc<AtomicBool>,
    ) -> Result<(), String> {
        self.ensure_not_running(provider)?;
        let mut downloads = self
            .inner
            .lock()
            .map_err(|_| "Model download state is unavailable".to_string())?;
        if downloads
            .get(&provider)
            .is_some_and(DownloadSnapshot::active)
        {
            return already_running(provider);
        }
        downloads.insert(
            provider,
            DownloadSnapshot::new(
                TranscriptionModelDownloadState::Downloading,
                0,
                artifact.archive_bytes,
                None,
            ),
        );
        if let Ok(mut cancellations) = self.cancellations.lock() {
            cancellations.insert(provider, Arc::clone(cancellation));
        }
        Ok(())
    }

    fn ensure_not_running(&self, provider: TranscriptionProvider) -> Result<(), String> {
        if self.download_running(provider) {
            return already_running(provider);
        }
        Ok(())
    }

    fn spawn_download_thread(&self, job: DownloadJob) {
        let state = self.clone();
        thread::spawn(move || {
            let result =
                install::install_model(job.artifact, &job.paths, &state, &job.cancellation);
            state.clear_cancellation(job.provider);
            state.finish_download(job, result);
        });
    }

    fn clear_cancellation(&self, provider: TranscriptionProvider) {
        if let Ok(mut cancellations) = self.cancellations.lock() {
            cancellations.remove(&provider);
        }
    }

    fn finish_download(&self, job: DownloadJob, result: Result<(), install::DownloadError>) {
        match result {
            Ok(()) => {
                (job.on_installed)();
                self.set_snapshot(
                    job.provider,
                    DownloadSnapshot::idle(job.artifact.archive_bytes),
                );
            }
            Err(install::DownloadError::Cancelled) => self.set_snapshot(
                job.provider,
                DownloadSnapshot::new(
                    TranscriptionModelDownloadState::Cancelled,
                    0,
                    job.artifact.archive_bytes,
                    None,
                ),
            ),
            Err(install::DownloadError::Failed(message)) => self.set_snapshot(
                job.provider,
                DownloadSnapshot::new(
                    TranscriptionModelDownloadState::Failed,
                    0,
                    job.artifact.archive_bytes,
                    Some(message),
                ),
            ),
        }
    }

    pub(super) fn set_snapshot(&self, provider: TranscriptionProvider, snapshot: DownloadSnapshot) {
        if let Ok(mut downloads) = self.inner.lock() {
            downloads.insert(provider, snapshot);
        }
    }
}

fn already_running(provider: TranscriptionProvider) -> Result<(), String> {
    Err(format!(
        "{} model download is already running",
        provider.display_name()
    ))
}

#[cfg(test)]
mod tests;
