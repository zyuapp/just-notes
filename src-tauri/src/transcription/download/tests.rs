use std::{
    fs::{self, File},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use super::install::DownloadError;
use super::{DownloadJob, ModelDownloadState};
use crate::{
    app::AppPaths,
    transcription::{parakeet_artifact, ModelArtifact, TranscriptionModelDownloadState},
};

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("just-notes-download-{tag}-{}", std::process::id()));
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

fn install_model_files(artifact: ModelArtifact, paths: &AppPaths) {
    for path in artifact.expected_files(paths) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create model dir");
        }
        File::create(&path).expect("create model file");
    }
}

fn job(artifact: ModelArtifact, paths: AppPaths, on_installed: super::OnInstalled) -> DownloadJob {
    DownloadJob {
        artifact,
        paths,
        on_installed,
        cancellation: Arc::new(AtomicBool::new(false)),
    }
}

#[test]
fn start_download_short_circuits_when_already_installed() {
    let fixture = Fixture::new("installed");
    let paths = fixture.paths();
    let artifact = parakeet_artifact();
    install_model_files(artifact, &paths);

    let state = ModelDownloadState::default();
    state
        .start_download(paths, Box::new(|| {}))
        .expect("start_download");

    assert!(!state.download_running());
    assert_eq!(
        state.snapshot_for(0).state,
        TranscriptionModelDownloadState::Idle
    );
}

#[test]
fn cancel_download_returns_false_when_not_running() {
    let state = ModelDownloadState::default();

    assert!(!state.cancel_download());
}

#[test]
fn finish_download_maps_failure_to_snapshot() {
    let fixture = Fixture::new("failed");
    let state = ModelDownloadState::default();

    state.finish_download(
        job(parakeet_artifact(), fixture.paths(), Box::new(|| {})),
        Err(DownloadError::Failed("boom".to_string())),
    );

    let snapshot = state.snapshot_for(0);
    assert_eq!(snapshot.state, TranscriptionModelDownloadState::Failed);
    assert_eq!(snapshot.error_message.as_deref(), Some("boom"));
}

#[test]
fn finish_download_runs_callback_and_marks_idle_on_success() {
    let fixture = Fixture::new("success");
    let state = ModelDownloadState::default();
    let installed = Arc::new(AtomicBool::new(false));
    let installed_flag = Arc::clone(&installed);

    state.finish_download(
        job(
            parakeet_artifact(),
            fixture.paths(),
            Box::new(move || installed_flag.store(true, Ordering::SeqCst)),
        ),
        Ok(()),
    );

    assert!(installed.load(Ordering::SeqCst));
    assert_eq!(
        state.snapshot_for(0).state,
        TranscriptionModelDownloadState::Idle
    );
}
