use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

use bzip2::{write::BzEncoder, Compression};

use super::{extract_archive, install_extracted_model, verify_archive};
use crate::{
    app::AppPaths,
    transcription::{parakeet_artifact, ModelArtifact},
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
            archived_dir: self.root.join("archived"),
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn parakeet() -> ModelArtifact {
    parakeet_artifact()
}

fn write_model_files(dir: &Path, artifact: ModelArtifact, paths: &AppPaths) {
    fs::create_dir_all(dir).expect("create source dir");
    for path in artifact.expected_files(paths) {
        let name = path.file_name().expect("model file name");
        File::create(dir.join(name)).expect("create model file");
    }
}

fn archive_fixture(path: &Path) -> &'static [u8] {
    const CONTENT: &[u8] = b"verified archive payload";
    let file = File::create(path).expect("create archive");
    let encoder = BzEncoder::new(file, Compression::best());
    let mut archive = tar::Builder::new(encoder);
    let mut header = tar::Header::new_gnu();
    header.set_size(CONTENT.len() as u64);
    header.set_mode(0o644);
    header.set_mtime(0);
    header.set_cksum();
    archive
        .append_data(&mut header, "fixture/payload.txt", CONTENT)
        .expect("append fixture payload");
    let encoder = archive.into_inner().expect("finish tar archive");
    encoder.finish().expect("finish bzip2 archive");
    CONTENT
}

#[test]
fn verifies_and_extracts_tar_bz2_archive() {
    const ARCHIVE_SHA256: &str = "a8d4e9ccb5fa1a8e49477d6ff745c84d5c2dd0e89e0d22f63e2481915361a2fa";
    let fixture = Fixture::new("archive-round-trip");
    let archive_path = fixture.root.join("fixture.tar.bz2");
    let extracting_dir = fixture.root.join("extracted");
    let expected_content = archive_fixture(&archive_path);
    let invalid_artifact = ModelArtifact {
        archive_sha256: "0000000000000000000000000000000000000000000000000000000000000000",
        ..parakeet()
    };
    let artifact = ModelArtifact {
        archive_sha256: ARCHIVE_SHA256,
        ..parakeet()
    };

    assert!(verify_archive(invalid_artifact, &archive_path).is_err());
    verify_archive(artifact, &archive_path).expect("verify archive checksum");
    extract_archive(&archive_path, &extracting_dir).expect("extract archive");

    let payload = fs::read(extracting_dir.join("fixture/payload.txt")).expect("read payload");
    assert_eq!(payload, expected_content);
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
