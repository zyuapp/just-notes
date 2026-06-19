use std::path::PathBuf;

use crate::app::AppPaths;

use super::{
    models::{parakeet_model_dir, PARAKEET_MODEL_ID, PARAKEET_MODEL_NAME},
    parakeet_model_files, TranscriptionProvider,
};

pub(crate) const PARAKEET_ARCHIVE_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8.tar.bz2";
pub(crate) const PARAKEET_ARCHIVE_SHA256: &str =
    "157c157bc51155e03e37d2466522a3a737dd9c72bb25f36eb18912964161e1ad";
pub(crate) const PARAKEET_ARCHIVE_BYTES: u64 = 482_468_385;

#[derive(Clone, Copy)]
pub(crate) struct ModelArtifact {
    pub(crate) provider: TranscriptionProvider,
    pub(crate) model_id: &'static str,
    pub(crate) display_name: &'static str,
    pub(crate) archive_url: &'static str,
    pub(crate) archive_sha256: &'static str,
    pub(crate) archive_bytes: u64,
    pub(crate) display_size: &'static str,
}

impl ModelArtifact {
    pub(crate) fn download_dir(self, paths: &AppPaths) -> PathBuf {
        paths.data_dir.join("models").join(".downloads")
    }

    pub(crate) fn partial_archive_path(self, paths: &AppPaths) -> PathBuf {
        self.download_dir(paths)
            .join(format!("{}.tar.bz2.part", self.model_id))
    }

    pub(crate) fn extracting_dir(self, paths: &AppPaths) -> PathBuf {
        self.download_dir(paths)
            .join(format!("{}.extracting", self.model_id))
    }

    pub(crate) fn final_model_dir(self, paths: &AppPaths) -> PathBuf {
        parakeet_model_dir(paths)
    }

    pub(crate) fn expected_files(self, paths: &AppPaths) -> [PathBuf; 4] {
        parakeet_model_files(&self.final_model_dir(paths))
    }
}

pub(crate) fn parakeet_artifact() -> ModelArtifact {
    ModelArtifact {
        provider: TranscriptionProvider::Parakeet,
        model_id: PARAKEET_MODEL_ID,
        display_name: PARAKEET_MODEL_NAME,
        archive_url: PARAKEET_ARCHIVE_URL,
        archive_sha256: PARAKEET_ARCHIVE_SHA256,
        archive_bytes: PARAKEET_ARCHIVE_BYTES,
        display_size: "460 MB",
    }
}

pub(crate) fn artifact_for_provider(provider: TranscriptionProvider) -> Option<ModelArtifact> {
    match provider {
        TranscriptionProvider::Parakeet => Some(parakeet_artifact()),
        TranscriptionProvider::Whisper => None,
    }
}
