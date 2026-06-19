use std::{fs, path::PathBuf};

use super::transcription_status_with_downloads;
use crate::app::AppPaths;
use crate::transcription::download::{DownloadSnapshot, ModelDownloadState};
use crate::transcription::models::TranscriptionModelStatus;
use crate::transcription::TranscriptionModelDownloadState;

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("just-notes-status-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create fixture root");
        Self { root }
    }

    fn paths(&self) -> AppPaths {
        AppPaths {
            data_dir: self.root.clone(),
            threads_dir: self.root.join("threads"),
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn parakeet_model(models: &[TranscriptionModelStatus]) -> &TranscriptionModelStatus {
    models.first().expect("parakeet model in status")
}

#[test]
fn downloading_snapshot_drives_progress_cancel_and_message() {
    let fixture = Fixture::new("downloading");
    let downloads = ModelDownloadState::default();
    downloads.set_snapshot(DownloadSnapshot::new(
        TranscriptionModelDownloadState::Downloading,
        50,
        100,
        None,
    ));

    let status = transcription_status_with_downloads(&fixture.paths(), &downloads);

    let parakeet = parakeet_model(&status.available_models);
    assert_eq!(
        parakeet.download_state,
        TranscriptionModelDownloadState::Downloading
    );
    assert_eq!(parakeet.progress_bytes, 50);
    assert_eq!(parakeet.total_bytes, 100);
    assert!(parakeet.can_cancel);
    assert!(!parakeet.can_download);
    assert_eq!(status.message, "Downloading Parakeet model (50%)");
}

#[test]
fn zero_total_bytes_reports_zero_percent() {
    let fixture = Fixture::new("zero-total");
    let downloads = ModelDownloadState::default();
    downloads.set_snapshot(DownloadSnapshot::new(
        TranscriptionModelDownloadState::Downloading,
        0,
        0,
        None,
    ));

    let status = transcription_status_with_downloads(&fixture.paths(), &downloads);

    assert_eq!(status.message, "Downloading Parakeet model (0%)");
}

#[test]
fn idle_uninstalled_model_can_download() {
    let fixture = Fixture::new("idle");
    let downloads = ModelDownloadState::default();

    let status = transcription_status_with_downloads(&fixture.paths(), &downloads);

    let parakeet = parakeet_model(&status.available_models);
    assert_eq!(
        parakeet.download_state,
        TranscriptionModelDownloadState::Idle
    );
    assert!(parakeet.can_download);
    assert!(!parakeet.can_cancel);
    assert!(status.message.contains("missing"));
}
