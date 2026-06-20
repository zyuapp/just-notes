use std::{
    fs::{self, File},
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use super::{
    delete_parakeet_model, finalization_transcription_catalog,
    finalization_transcription_selection, PARAKEET_DECODER, PARAKEET_ENCODER, PARAKEET_JOINER,
    PARAKEET_MODEL_ID, PARAKEET_MODEL_NAME, PARAKEET_TOKENS,
};
use crate::app::AppPaths;

#[test]
fn catalog_selects_the_installed_parakeet_model() {
    let fixture = ModelDirFixture::new();
    fixture.install_parakeet();

    let catalog = finalization_transcription_catalog(&fixture.app_paths());

    assert_eq!(catalog.selection.model_name, PARAKEET_MODEL_NAME);
    assert!(catalog.selection.is_installed());
    assert!(selected_model(
        &catalog.available_models,
        PARAKEET_MODEL_NAME
    ));
}

#[test]
fn selection_points_at_the_model_dir_when_missing() {
    let fixture = ModelDirFixture::new();

    let selection = finalization_transcription_selection(&fixture.app_paths());

    assert_eq!(selection.model_name, PARAKEET_MODEL_NAME);
    assert!(!selection.is_installed());
    assert!(selection.model_path.ends_with(PARAKEET_MODEL_ID));
}

#[test]
fn delete_removes_the_installed_model_and_is_idempotent() {
    let fixture = ModelDirFixture::new();
    fixture.install_parakeet();
    let paths = fixture.app_paths();
    assert!(finalization_transcription_selection(&paths).is_installed());

    delete_parakeet_model(&paths).expect("delete installed model");
    assert!(!finalization_transcription_selection(&paths).is_installed());

    delete_parakeet_model(&paths).expect("delete already-missing model");
}

fn selected_model(models: &[super::TranscriptionModelStatus], name: &str) -> bool {
    models
        .iter()
        .any(|model| model.name == name && model.selected)
}

struct ModelDirFixture {
    root: PathBuf,
}

impl ModelDirFixture {
    fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "just-notes-model-policy-test-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("models").join("parakeet").join(PARAKEET_MODEL_ID))
            .expect("create parakeet fixture dir");
        Self { root }
    }

    fn app_paths(&self) -> AppPaths {
        AppPaths {
            data_dir: self.root.clone(),
            threads_dir: self.root.join("threads"),
            archived_dir: self.root.join("archived"),
        }
    }

    fn install_parakeet(&self) {
        let model_dir = self
            .root
            .join("models")
            .join("parakeet")
            .join(PARAKEET_MODEL_ID);
        for filename in [
            PARAKEET_ENCODER,
            PARAKEET_DECODER,
            PARAKEET_JOINER,
            PARAKEET_TOKENS,
        ] {
            File::create(model_dir.join(filename)).expect("create parakeet fixture file");
        }
    }
}

impl Drop for ModelDirFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
