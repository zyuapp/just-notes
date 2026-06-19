use std::path::PathBuf;

use crate::app::AppPaths;

use super::models::{
    PARAKEET_MODEL_FILENAMES, PARAKEET_MODEL_ID, PARAKEET_MODEL_NAME, PARAKEET_MODEL_SUBDIR,
};

pub(crate) const PARAKEET_ARCHIVE_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8.tar.bz2";
pub(crate) const PARAKEET_ARCHIVE_SHA256: &str =
    "157c157bc51155e03e37d2466522a3a737dd9c72bb25f36eb18912964161e1ad";
pub(crate) const PARAKEET_ARCHIVE_BYTES: u64 = 482_468_385;

#[derive(Clone, Copy)]
pub(crate) struct ModelArtifact {
    pub(crate) model_id: &'static str,
    pub(crate) display_name: &'static str,
    pub(crate) model_subdir: &'static str,
    pub(crate) model_files: &'static [&'static str],
    pub(crate) archive_url: &'static str,
    pub(crate) archive_sha256: &'static str,
    pub(crate) archive_bytes: u64,
    pub(crate) display_size: &'static str,
}

impl ModelArtifact {
    pub(crate) fn download_dir(paths: &AppPaths) -> PathBuf {
        paths.data_dir.join("models").join(".downloads")
    }

    pub(crate) fn partial_archive_path(self, paths: &AppPaths) -> PathBuf {
        Self::download_dir(paths).join(format!("{}.tar.bz2.part", self.model_id))
    }

    pub(crate) fn extracting_dir(self, paths: &AppPaths) -> PathBuf {
        Self::download_dir(paths).join(format!("{}.extracting", self.model_id))
    }

    pub(crate) fn final_model_dir(self, paths: &AppPaths) -> PathBuf {
        paths
            .data_dir
            .join("models")
            .join(self.model_subdir)
            .join(self.model_id)
    }

    pub(crate) fn expected_files(self, paths: &AppPaths) -> Vec<PathBuf> {
        let model_dir = self.final_model_dir(paths);
        self.model_files
            .iter()
            .map(|name| model_dir.join(name))
            .collect()
    }
}

pub(crate) fn parakeet_artifact() -> ModelArtifact {
    ModelArtifact {
        model_id: PARAKEET_MODEL_ID,
        display_name: PARAKEET_MODEL_NAME,
        model_subdir: PARAKEET_MODEL_SUBDIR,
        model_files: &PARAKEET_MODEL_FILENAMES,
        archive_url: PARAKEET_ARCHIVE_URL,
        archive_sha256: PARAKEET_ARCHIVE_SHA256,
        archive_bytes: PARAKEET_ARCHIVE_BYTES,
        display_size: "460 MB",
    }
}
