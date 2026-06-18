use std::{
    fs::{self, File},
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use super::{
    finalization_transcription_catalog, finalization_transcription_selection,
    TranscriptionProvider, PARAKEET_DECODER, PARAKEET_ENCODER, PARAKEET_JOINER, PARAKEET_MODEL_ID,
    PARAKEET_MODEL_NAME, PARAKEET_TOKENS,
};
use crate::app::AppPaths;

#[test]
fn finalization_prefers_the_largest_installed_model() {
    let fixture = ModelDirFixture::new();
    fixture.install("ggml-base.en.bin");
    fixture.install("ggml-small.en.bin");

    let catalog =
        finalization_transcription_catalog(&fixture.app_paths(), TranscriptionProvider::Whisper);

    assert_eq!(catalog.selection.model_name, "small.en");
    assert!(selected_model(&catalog.available_models, "small.en"));
}

#[test]
fn finalization_policy_marks_the_largest_installed_model_as_selected() {
    let fixture = ModelDirFixture::new();
    fixture.install("ggml-base.en.bin");
    fixture.install("ggml-small.en.bin");

    let catalog =
        finalization_transcription_catalog(&fixture.app_paths(), TranscriptionProvider::Whisper);

    assert_eq!(catalog.selection.model_name, "small.en");
    assert!(selected_model(&catalog.available_models, "small.en"));
}

#[test]
fn missing_models_fall_back_to_base_model_path() {
    let fixture = ModelDirFixture::new();

    let paths =
        finalization_transcription_selection(&fixture.app_paths(), TranscriptionProvider::Whisper);

    assert_eq!(paths.model_name, "base.en");
    assert!(paths.model_path.ends_with("ggml-base.en.bin"));
}

#[test]
fn parakeet_is_selected_when_requested() {
    let fixture = ModelDirFixture::new();
    fixture.install_parakeet();

    let catalog =
        finalization_transcription_catalog(&fixture.app_paths(), TranscriptionProvider::Parakeet);

    assert_eq!(catalog.selection.provider, TranscriptionProvider::Parakeet);
    assert_eq!(catalog.selection.model_name, PARAKEET_MODEL_NAME);
    assert!(catalog.selection.is_installed());
    assert!(selected_model(
        &catalog.available_models,
        PARAKEET_MODEL_NAME
    ));
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
        fs::create_dir_all(root.join("models").join("whisper")).expect("create model fixture dir");
        fs::create_dir_all(root.join("models").join("parakeet").join(PARAKEET_MODEL_ID))
            .expect("create parakeet fixture dir");
        Self { root }
    }

    fn app_paths(&self) -> AppPaths {
        AppPaths {
            data_dir: self.root.clone(),
            threads_dir: self.root.join("threads"),
        }
    }

    fn install(&self, filename: &str) {
        File::create(self.root.join("models").join("whisper").join(filename))
            .expect("create model fixture file");
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
