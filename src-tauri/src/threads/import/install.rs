use std::{
    fs,
    path::{Path, PathBuf},
};

use super::{
    format_io, LegacyImportPlan, LegacyImportResultData, PlannedRecording, ThreadMetadata,
    ThreadStatus,
};

impl LegacyImportPlan {
    pub(crate) fn execute(self) -> Result<LegacyImportResultData, String> {
        cleanup_staging_root(&self.staging_root)?;
        fs::create_dir_all(&self.staging_root).map_err(format_io("prepare import staging"))?;
        let mut installed = Vec::new();
        for recording in &self.recordings {
            if let Err(error) = install_recording(recording, &self.staging_root) {
                let rollback_errors = rollback_import(&installed);
                let _ = cleanup_staging_root(&self.staging_root);
                let detail = if rollback_errors.is_empty() {
                    String::new()
                } else {
                    format!(" Rollback was incomplete: {}", rollback_errors.join("; "))
                };
                return Err(format!("{error}.{detail}"));
            }
            installed.push(recording.destination.clone());
        }
        cleanup_staging_root(&self.staging_root)?;

        Ok(LegacyImportResultData {
            imported: self.recordings.len(),
            active_recordings: self.preview.active_recordings,
            archived_recordings: self.preview.archived_recordings,
            duplicates: self.preview.duplicates,
            conflicts: self.preview.conflicts,
        })
    }
}

pub(crate) fn cleanup_stale_import_staging(paths: &crate::app::AppPaths) -> Result<(), String> {
    cleanup_staging_root(&paths.legacy_import_staging_dir())
}

fn cleanup_staging_root(staging_root: &Path) -> Result<(), String> {
    if staging_root.exists() {
        fs::remove_dir_all(staging_root).map_err(format_io("clear incomplete legacy import"))?;
    }
    Ok(())
}

fn install_recording(recording: &PlannedRecording, staging_root: &Path) -> Result<(), String> {
    if recording.destination.exists() {
        return Err(format!(
            "Recordings changed after the import preview; {} now exists. Review the import again",
            recording.destination.display()
        ));
    }
    let parent = recording
        .destination
        .parent()
        .ok_or_else(|| "The import destination is invalid".to_string())?;
    fs::create_dir_all(parent).map_err(format_io("prepare import destination"))?;
    let name = recording
        .destination
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "The imported recording has an invalid name".to_string())?;
    let staging = staging_root.join(name);
    if staging.exists() {
        fs::remove_dir_all(&staging).map_err(format_io("clear incomplete import"))?;
    }

    let result = (|| {
        copy_tree(&recording.source, &staging)?;
        normalize_recording_metadata(&staging, recording.replacement_id.as_deref())?;
        fs::rename(&staging, &recording.destination).map_err(format_io("install recording"))
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(source).map_err(format_io("inspect legacy data"))?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "Legacy data contains an unsupported symbolic link at {}",
            source.display()
        ));
    }
    if metadata.is_file() {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(format_io("prepare imported file"))?;
        }
        fs::copy(source, destination).map_err(format_io("copy legacy data"))?;
        return Ok(());
    }
    if !metadata.is_dir() {
        return Err(format!("Unsupported legacy data at {}", source.display()));
    }
    fs::create_dir_all(destination).map_err(format_io("prepare imported folder"))?;
    for entry in fs::read_dir(source).map_err(format_io("read legacy data"))? {
        let entry = entry.map_err(format_io("read legacy entry"))?;
        copy_tree(&entry.path(), &destination.join(entry.file_name()))?;
    }
    Ok(())
}

fn normalize_recording_metadata(
    recording: &Path,
    replacement_id: Option<&str>,
) -> Result<(), String> {
    let metadata_path = recording.join("thread.json");
    let mut metadata: ThreadMetadata = serde_json::from_slice(
        &fs::read(&metadata_path).map_err(format_io("read imported metadata"))?,
    )
    .map_err(|error| format!("Invalid {}: {error}", metadata_path.display()))?;
    if let Some(replacement_id) = replacement_id {
        metadata.id = replacement_id.to_string();
    }
    metadata.status = ThreadStatus::Idle;
    let json = serde_json::to_vec_pretty(&metadata)
        .map_err(|error| format!("Failed to encode imported metadata: {error}"))?;
    fs::write(&metadata_path, json).map_err(format_io("normalize imported recording metadata"))
}

fn rollback_import(installed: &[PathBuf]) -> Vec<String> {
    let mut errors = Vec::new();
    for path in installed.iter().rev() {
        if let Err(error) = fs::remove_dir_all(path) {
            errors.push(format!("could not remove {}: {error}", path.display()));
        }
    }
    errors
}
