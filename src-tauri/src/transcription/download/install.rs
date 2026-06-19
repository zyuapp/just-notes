use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

use bzip2::read::BzDecoder;
use sha2::{Digest, Sha256};

use crate::app::AppPaths;

use super::{DownloadSnapshot, ModelDownloadState};
use crate::transcription::{ModelArtifact, TranscriptionModelDownloadState};

#[derive(Debug)]
pub(super) enum DownloadError {
    Cancelled,
    Failed(String),
}

impl From<String> for DownloadError {
    fn from(value: String) -> Self {
        Self::Failed(value)
    }
}

pub(super) fn install_model(
    artifact: ModelArtifact,
    paths: &AppPaths,
    state: &ModelDownloadState,
    cancellation: &AtomicBool,
) -> Result<(), DownloadError> {
    let archive_path = artifact.partial_archive_path(paths);
    let extracting_dir = artifact.extracting_dir(paths);
    fs::create_dir_all(ModelArtifact::download_dir(paths))
        .map_err(format_io("create download folder"))?;
    cleanup_path(&archive_path)?;
    cleanup_path(&extracting_dir)?;

    let result = (|| {
        download_archive(artifact, &archive_path, state, cancellation)?;
        check_cancelled(cancellation)?;
        verify_archive(artifact, &archive_path)?;
        check_cancelled(cancellation)?;

        state.set_snapshot(
            artifact.provider,
            DownloadSnapshot::new(
                TranscriptionModelDownloadState::Installing,
                artifact.archive_bytes,
                artifact.archive_bytes,
                None,
            ),
        );
        extract_archive(&archive_path, &extracting_dir)?;
        check_cancelled(cancellation)?;
        install_extracted_model(artifact, paths, &extracting_dir)
    })();

    let cleanup_archive = cleanup_path(&archive_path);
    let cleanup_extracting = cleanup_path(&extracting_dir);
    result?;
    cleanup_archive?;
    cleanup_extracting?;
    Ok(())
}

fn download_archive(
    artifact: ModelArtifact,
    archive_path: &Path,
    state: &ModelDownloadState,
    cancellation: &AtomicBool,
) -> Result<(), DownloadError> {
    let mut response = reqwest::blocking::get(artifact.archive_url)
        .map_err(|err| format!("Failed to download {}: {err}", artifact.display_name))?
        .error_for_status()
        .map_err(|err| format!("Failed to download {}: {err}", artifact.display_name))?;
    let mut archive = File::create(archive_path).map_err(format_io("create model archive"))?;
    let mut buffer = [0_u8; 1024 * 256];
    let mut downloaded = 0_u64;
    loop {
        check_cancelled(cancellation)?;
        let read = response
            .read(&mut buffer)
            .map_err(|err| format!("Failed to read model download: {err}"))?;
        if read == 0 {
            break;
        }
        archive
            .write_all(&buffer[..read])
            .map_err(format_io("write model archive"))?;
        downloaded = downloaded.saturating_add(read as u64);
        state.set_snapshot(
            artifact.provider,
            DownloadSnapshot::new(
                TranscriptionModelDownloadState::Downloading,
                downloaded,
                artifact.archive_bytes,
                None,
            ),
        );
    }
    Ok(())
}

fn verify_archive(artifact: ModelArtifact, archive_path: &Path) -> Result<(), DownloadError> {
    let mut archive = File::open(archive_path).map_err(format_io("open model archive"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 256];
    loop {
        let read = archive
            .read(&mut buffer)
            .map_err(|err| format!("Failed to read model archive: {err}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let checksum = format!("{:x}", hasher.finalize());
    if checksum != artifact.archive_sha256 {
        return Err(format!(
            "Downloaded model failed integrity check: expected {}, got {}",
            artifact.archive_sha256, checksum
        )
        .into());
    }
    Ok(())
}

fn extract_archive(archive_path: &Path, extracting_dir: &Path) -> Result<(), DownloadError> {
    cleanup_path(extracting_dir)?;
    fs::create_dir_all(extracting_dir).map_err(format_io("create extraction folder"))?;
    let archive = File::open(archive_path).map_err(format_io("open model archive"))?;
    let decoder = BzDecoder::new(archive);
    let mut tar = tar::Archive::new(decoder);
    tar.unpack(extracting_dir)
        .map_err(|err| format!("Failed to extract model archive: {err}"))?;
    Ok(())
}

fn install_extracted_model(
    artifact: ModelArtifact,
    paths: &AppPaths,
    extracting_dir: &Path,
) -> Result<(), DownloadError> {
    let extracted_model_dir = extracting_dir.join(artifact.model_id);
    let source_dir = if extracted_model_dir.is_dir() {
        extracted_model_dir.as_path()
    } else {
        extracting_dir
    };
    for final_path in artifact.expected_files(paths) {
        let Some(filename) = final_path.file_name() else {
            return Err(format!("Invalid {} model filename", artifact.display_name).into());
        };
        let source_path = source_dir.join(filename);
        if !source_path.is_file() {
            return Err(format!("Downloaded model is missing {}", source_path.display()).into());
        }
    }

    let final_dir = artifact.final_model_dir(paths);
    let parent = final_dir
        .parent()
        .ok_or_else(|| format!("Invalid {} model folder", artifact.display_name))?;
    fs::create_dir_all(parent).map_err(format_io("create model folder"))?;
    let staging_dir =
        ModelArtifact::download_dir(paths).join(format!("{}.installing", artifact.model_id));
    cleanup_path(&staging_dir)?;
    fs::create_dir_all(&staging_dir).map_err(format_io("create model install folder"))?;

    for final_path in artifact.expected_files(paths) {
        let filename = final_path
            .file_name()
            .ok_or_else(|| format!("Invalid {} model filename", artifact.display_name))?;
        fs::copy(source_dir.join(filename), staging_dir.join(filename))
            .map_err(format_io("copy model file"))?;
    }

    cleanup_path(&final_dir)?;
    fs::rename(&staging_dir, &final_dir).map_err(format_io("install model folder"))?;
    Ok(())
}

fn check_cancelled(cancellation: &AtomicBool) -> Result<(), DownloadError> {
    if cancellation.load(Ordering::SeqCst) {
        return Err(DownloadError::Cancelled);
    }
    Ok(())
}

fn cleanup_path(path: &Path) -> Result<(), DownloadError> {
    if path.is_dir() {
        fs::remove_dir_all(path).map_err(format_io("clean temporary model folder"))?;
    } else if path.exists() {
        fs::remove_file(path).map_err(format_io("clean temporary model file"))?;
    }
    Ok(())
}

fn format_io(action: &'static str) -> impl FnOnce(std::io::Error) -> DownloadError {
    move |err| DownloadError::Failed(format!("Failed to {action}: {err}"))
}

#[cfg(test)]
mod tests;
