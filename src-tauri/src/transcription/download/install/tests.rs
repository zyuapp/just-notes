use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

use super::install_extracted_model;
use crate::{
    app::AppPaths,
    transcription::{artifact_for_provider, ModelArtifact, TranscriptionProvider},
};

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("just-notes-install-{tag}-{}", std::process::id()));
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

fn parakeet() -> ModelArtifact {
    artifact_for_provider(TranscriptionProvider::Parakeet).expect("parakeet artifact")
}

fn write_model_files(dir: &Path, artifact: ModelArtifact, paths: &AppPaths) {
    fs::create_dir_all(dir).expect("create source dir");
    for path in artifact.expected_files(paths) {
        let name = path.file_name().expect("model file name");
        File::create(dir.join(name)).expect("create model file");
    }
}

#[test]
fn installs_from_model_id_subdir() {
    let fixture = Fixture::new("subdir");
    let paths = fixture.paths();
    let artifact = parakeet();
    let extracting = paths.data_dir.join("extracting");
    write_model_files(&extracting.join(artifact.model_id), artifact, &paths);

    install_extracted_model(artifact, &paths, &extracting).expect("install");

    for path in artifact.expected_files(&paths) {
        assert!(path.is_file(), "missing installed file {}", path.display());
    }
}

#[test]
fn installs_from_flat_layout() {
    let fixture = Fixture::new("flat");
    let paths = fixture.paths();
    let artifact = parakeet();
    let extracting = paths.data_dir.join("extracting");
    write_model_files(&extracting, artifact, &paths);

    install_extracted_model(artifact, &paths, &extracting).expect("install");

    assert!(artifact.expected_files(&paths).iter().all(|p| p.is_file()));
}

#[test]
fn rejects_missing_file() {
    let fixture = Fixture::new("missing");
    let paths = fixture.paths();
    let artifact = parakeet();
    let extracting = paths.data_dir.join("extracting");
    let source = extracting.join(artifact.model_id);
    fs::create_dir_all(&source).expect("create source");
    let expected = artifact.expected_files(&paths);
    for path in &expected[..expected.len() - 1] {
        File::create(source.join(path.file_name().expect("model file name"))).expect("create");
    }

    let result = install_extracted_model(artifact, &paths, &extracting);

    assert!(result.is_err());
    assert!(!artifact.final_model_dir(&paths).exists());
}
