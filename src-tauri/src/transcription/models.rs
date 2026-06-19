use std::path::{Path, PathBuf};

use crate::app::AppPaths;

use super::artifacts::parakeet_artifact;

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
pub(crate) const PARAKEET_DISPLAY_NAME: &str = "Parakeet";
pub(crate) const PARAKEET_RUNTIME_NAME: &str = "embedded sherpa-onnx Parakeet runtime";

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
    pub(crate) available_models: Vec<TranscriptionModelStatus>,
    pub(crate) message: String,
}

#[derive(serde::Serialize, ts_rs::TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) struct TranscriptionModelStatus {
    pub(crate) name: String,
    pub(crate) filename: String,
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

#[derive(Clone)]
pub(crate) struct TranscriptionModelSelection {
    pub(crate) model_path: PathBuf,
    pub(crate) model_name: String,
}

impl TranscriptionModelSelection {
    pub(crate) fn is_installed(&self) -> bool {
        parakeet_model_files(&self.model_path)
            .iter()
            .all(|path| path.is_file())
    }
}

pub(crate) struct TranscriptionModelCatalog {
    pub(crate) selection: TranscriptionModelSelection,
    pub(crate) available_models: Vec<TranscriptionModelStatus>,
}

pub(crate) fn finalization_transcription_catalog(paths: &AppPaths) -> TranscriptionModelCatalog {
    let mut model = parakeet_model_status(&parakeet_model_dir(paths));
    model.selected = true;
    TranscriptionModelCatalog {
        selection: TranscriptionModelSelection {
            model_path: model.path.clone(),
            model_name: model.name.clone(),
        },
        available_models: vec![model],
    }
}

pub(crate) fn finalization_transcription_selection(
    paths: &AppPaths,
) -> TranscriptionModelSelection {
    finalization_transcription_catalog(paths).selection
}

fn parakeet_model_status(model_dir: &Path) -> TranscriptionModelStatus {
    let artifact = parakeet_artifact();
    let installed = parakeet_model_files(model_dir)
        .iter()
        .all(|path| path.is_file());
    TranscriptionModelStatus {
        name: PARAKEET_MODEL_NAME.to_string(),
        filename: PARAKEET_MODEL_ID.to_string(),
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

#[cfg(test)]
mod tests;
