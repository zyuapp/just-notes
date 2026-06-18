use std::path::{Path, PathBuf};

use crate::app::AppPaths;

const WHISPER_MODEL_CANDIDATES: [WhisperModelCandidate; 3] = [
    WhisperModelCandidate {
        name: "medium.en",
        filename: "ggml-medium.en.bin",
    },
    WhisperModelCandidate {
        name: "small.en",
        filename: "ggml-small.en.bin",
    },
    WhisperModelCandidate {
        name: "base.en",
        filename: "ggml-base.en.bin",
    },
];

struct WhisperModelCandidate {
    name: &'static str,
    filename: &'static str,
}

#[derive(serde::Serialize, ts_rs::TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) struct TranscriptionStatusPayload {
    pub(crate) ready: bool,
    pub(crate) engine_exists: bool,
    pub(crate) model_exists: bool,
    pub(crate) engine_path: String,
    pub(crate) model_path: String,
    pub(crate) model_name: String,
    pub(crate) available_models: Vec<WhisperModelStatus>,
    pub(crate) message: String,
}

#[derive(serde::Serialize, ts_rs::TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) struct WhisperModelStatus {
    pub(crate) name: String,
    pub(crate) filename: String,
    #[ts(type = "string")]
    pub(crate) path: PathBuf,
    pub(crate) installed: bool,
    pub(crate) selected: bool,
}

#[derive(Clone)]
pub(crate) struct TranscriptionPaths {
    pub(crate) model_path: PathBuf,
    pub(crate) model_name: String,
    pub(crate) available_models: Vec<WhisperModelStatus>,
}

pub(crate) fn discover_whisper_models(model_dir: &Path) -> Vec<WhisperModelStatus> {
    WHISPER_MODEL_CANDIDATES
        .iter()
        .map(|candidate| {
            let path = model_dir.join(candidate.filename);
            WhisperModelStatus {
                name: candidate.name.to_string(),
                filename: candidate.filename.to_string(),
                installed: path.is_file(),
                path,
                selected: false,
            }
        })
        .collect()
}

pub(crate) fn finalization_transcription_paths(paths: &AppPaths) -> TranscriptionPaths {
    let mut available_models =
        discover_whisper_models(&paths.data_dir.join("models").join("whisper"));
    let selected_model = select_model(&available_models);
    for model in &mut available_models {
        model.selected = model.filename == selected_model.filename;
    }

    TranscriptionPaths {
        model_path: selected_model.path.clone(),
        model_name: selected_model.name.clone(),
        available_models,
    }
}

fn select_model(available_models: &[WhisperModelStatus]) -> WhisperModelStatus {
    available_models
        .iter()
        .find(|model| model.installed)
        .cloned()
        .unwrap_or_else(|| {
            available_models
                .last()
                .expect("whisper model candidates")
                .clone()
        })
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{self, File},
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::finalization_transcription_paths;
    use crate::app::AppPaths;

    #[test]
    fn finalization_prefers_the_largest_installed_model() {
        let fixture = ModelDirFixture::new();
        fixture.install("ggml-base.en.bin");
        fixture.install("ggml-small.en.bin");

        let paths = finalization_transcription_paths(&fixture.app_paths());

        assert_eq!(paths.model_name, "small.en");
        assert!(selected_model(&paths.available_models, "small.en"));
    }

    #[test]
    fn finalization_policy_marks_the_largest_installed_model_as_selected() {
        let fixture = ModelDirFixture::new();
        fixture.install("ggml-base.en.bin");
        fixture.install("ggml-small.en.bin");

        let paths = finalization_transcription_paths(&fixture.app_paths());

        assert_eq!(paths.model_name, "small.en");
        assert!(selected_model(&paths.available_models, "small.en"));
    }

    #[test]
    fn missing_models_fall_back_to_base_model_path() {
        let fixture = ModelDirFixture::new();

        let paths = finalization_transcription_paths(&fixture.app_paths());

        assert_eq!(paths.model_name, "base.en");
        assert!(paths.model_path.ends_with("ggml-base.en.bin"));
    }

    fn selected_model(models: &[super::WhisperModelStatus], name: &str) -> bool {
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
            fs::create_dir_all(root.join("models").join("whisper"))
                .expect("create model fixture dir");
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
    }

    impl Drop for ModelDirFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
