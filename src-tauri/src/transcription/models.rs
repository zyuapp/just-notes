use std::path::{Path, PathBuf};

use crate::app::AppPaths;

use super::artifacts::parakeet_artifact;

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
pub(crate) const PARAKEET_MODEL_ID: &str = "sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8";
pub(crate) const PARAKEET_MODEL_NAME: &str = "Parakeet TDT 0.6B v2";
pub(crate) const PARAKEET_ENCODER: &str = "encoder.int8.onnx";
pub(crate) const PARAKEET_DECODER: &str = "decoder.int8.onnx";
pub(crate) const PARAKEET_JOINER: &str = "joiner.int8.onnx";
pub(crate) const PARAKEET_TOKENS: &str = "tokens.txt";
pub(crate) const PARAKEET_MODEL_SUBDIR: &str = "parakeet";
pub(crate) const PARAKEET_MODEL_FILENAMES: [&str; 4] = [
    PARAKEET_ENCODER,
    PARAKEET_DECODER,
    PARAKEET_JOINER,
    PARAKEET_TOKENS,
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
    pub(crate) provider: TranscriptionProvider,
    pub(crate) available_models: Vec<TranscriptionModelStatus>,
    pub(crate) message: String,
}

#[derive(serde::Serialize, ts_rs::TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) struct TranscriptionModelStatus {
    pub(crate) name: String,
    pub(crate) filename: String,
    pub(crate) provider: TranscriptionProvider,
    #[ts(type = "string")]
    pub(crate) path: PathBuf,
    pub(crate) installed: bool,
    pub(crate) selected: bool,
    pub(crate) downloadable: bool,
    pub(crate) download_state: TranscriptionModelDownloadState,
    pub(crate) progress_bytes: u64,
    pub(crate) total_bytes: u64,
    pub(crate) display_size: String,
    pub(crate) can_download: bool,
    pub(crate) can_cancel: bool,
    pub(crate) error_message: Option<String>,
}

#[derive(
    serde::Serialize, serde::Deserialize, ts_rs::TS, Clone, Copy, Debug, Hash, PartialEq, Eq,
)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) enum TranscriptionProvider {
    Parakeet,
    Whisper,
}

#[derive(serde::Serialize, ts_rs::TS, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) enum TranscriptionModelDownloadState {
    Idle,
    Downloading,
    Installing,
    Failed,
    Cancelled,
}

impl TranscriptionProvider {
    pub(crate) fn runtime_name(self) -> &'static str {
        match self {
            Self::Parakeet => "embedded sherpa-onnx Parakeet runtime",
            Self::Whisper => "embedded whisper.cpp runtime",
        }
    }

    pub(crate) fn display_name(self) -> &'static str {
        match self {
            Self::Parakeet => "Parakeet",
            Self::Whisper => "Whisper",
        }
    }
}

#[derive(Clone)]
pub(crate) struct TranscriptionModelSelection {
    pub(crate) provider: TranscriptionProvider,
    pub(crate) model_path: PathBuf,
    pub(crate) model_name: String,
}

impl TranscriptionModelSelection {
    pub(crate) fn is_installed(&self) -> bool {
        match self.provider {
            TranscriptionProvider::Parakeet => parakeet_model_files(&self.model_path)
                .iter()
                .all(|path| path.is_file()),
            TranscriptionProvider::Whisper => self.model_path.is_file(),
        }
    }
}

pub(crate) struct TranscriptionModelCatalog {
    pub(crate) selection: TranscriptionModelSelection,
    pub(crate) available_models: Vec<TranscriptionModelStatus>,
}

pub(crate) fn discover_whisper_models(model_dir: &Path) -> Vec<TranscriptionModelStatus> {
    WHISPER_MODEL_CANDIDATES
        .iter()
        .map(|candidate| {
            let path = model_dir.join(candidate.filename);
            TranscriptionModelStatus {
                name: candidate.name.to_string(),
                filename: candidate.filename.to_string(),
                provider: TranscriptionProvider::Whisper,
                installed: path.is_file(),
                path,
                selected: false,
                downloadable: false,
                download_state: TranscriptionModelDownloadState::Idle,
                progress_bytes: 0,
                total_bytes: 0,
                display_size: String::new(),
                can_download: false,
                can_cancel: false,
                error_message: None,
            }
        })
        .collect()
}

pub(crate) fn finalization_transcription_catalog(
    paths: &AppPaths,
    provider: TranscriptionProvider,
) -> TranscriptionModelCatalog {
    let mut available_models = discover_models(paths);
    let selected_model = select_model(&available_models, provider);
    for model in &mut available_models {
        model.selected =
            model.provider == selected_model.provider && model.filename == selected_model.filename;
    }

    TranscriptionModelCatalog {
        selection: TranscriptionModelSelection {
            provider: selected_model.provider,
            model_path: selected_model.path.clone(),
            model_name: selected_model.name.clone(),
        },
        available_models,
    }
}

pub(crate) fn finalization_transcription_selection(
    paths: &AppPaths,
    provider: TranscriptionProvider,
) -> TranscriptionModelSelection {
    finalization_transcription_catalog(paths, provider).selection
}

fn discover_models(paths: &AppPaths) -> Vec<TranscriptionModelStatus> {
    let mut models = vec![parakeet_model_status(&parakeet_model_dir(paths))];
    models.extend(discover_whisper_models(
        &paths.data_dir.join("models").join("whisper"),
    ));
    models
}

fn parakeet_model_status(model_dir: &Path) -> TranscriptionModelStatus {
    let artifact = parakeet_artifact();
    let installed = parakeet_model_files(model_dir)
        .iter()
        .all(|path| path.is_file());
    TranscriptionModelStatus {
        name: PARAKEET_MODEL_NAME.to_string(),
        filename: PARAKEET_MODEL_ID.to_string(),
        provider: TranscriptionProvider::Parakeet,
        path: model_dir.to_path_buf(),
        installed,
        selected: false,
        downloadable: true,
        download_state: TranscriptionModelDownloadState::Idle,
        progress_bytes: 0,
        total_bytes: artifact.archive_bytes,
        display_size: artifact.display_size.to_string(),
        can_download: !installed,
        can_cancel: false,
        error_message: None,
    }
}

pub(crate) fn parakeet_model_dir(paths: &AppPaths) -> PathBuf {
    paths
        .data_dir
        .join("models")
        .join(PARAKEET_MODEL_SUBDIR)
        .join(PARAKEET_MODEL_ID)
}

pub(crate) fn parakeet_model_files(model_dir: &Path) -> [PathBuf; 4] {
    PARAKEET_MODEL_FILENAMES.map(|name| model_dir.join(name))
}

fn select_model(
    available_models: &[TranscriptionModelStatus],
    provider: TranscriptionProvider,
) -> TranscriptionModelStatus {
    let provider_models = available_models
        .iter()
        .filter(|model| model.provider == provider)
        .collect::<Vec<_>>();
    provider_models
        .iter()
        .find(|model| model.installed)
        .copied()
        .or_else(|| provider_models.last().copied())
        .expect("provider model candidates")
        .clone()
}

#[cfg(test)]
mod tests;
