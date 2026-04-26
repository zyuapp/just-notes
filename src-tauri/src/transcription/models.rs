use std::path::{Path, PathBuf};

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
