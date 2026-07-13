mod install;
mod plan;
mod scan;

#[cfg(test)]
mod tests;

use std::path::PathBuf;

use crate::app::AppPaths;

use super::{ThreadMetadata, ThreadStatus};

pub(crate) use install::cleanup_stale_import_staging;
pub(crate) use plan::plan_legacy_import;

#[derive(Clone)]
pub(crate) struct LegacyImportPreviewData {
    pub(crate) active_recordings: usize,
    pub(crate) archived_recordings: usize,
    pub(crate) duplicates: usize,
    pub(crate) conflicts: usize,
    pub(crate) bytes_to_copy: u64,
    pub(crate) source_path: String,
}

pub(crate) struct LegacyImportResultData {
    pub(crate) imported: usize,
    pub(crate) active_recordings: usize,
    pub(crate) archived_recordings: usize,
    pub(crate) duplicates: usize,
    pub(crate) conflicts: usize,
}

pub(crate) struct LegacyImportPlan {
    preview: LegacyImportPreviewData,
    recordings: Vec<PlannedRecording>,
    active_destination: PathBuf,
    archive_destination: PathBuf,
    staging_root: PathBuf,
}

struct PlannedRecording {
    source: PathBuf,
    destination: PathBuf,
    replacement_id: Option<String>,
}

#[derive(Clone, Copy)]
enum RecordingStore {
    Active,
    Archived,
}

impl LegacyImportPlan {
    pub(crate) fn preview(&self) -> LegacyImportPreviewData {
        self.preview.clone()
    }

    pub(crate) fn matches_destination(&self, paths: &AppPaths) -> bool {
        self.active_destination == paths.threads_dir
            && self.archive_destination == paths.archived_dir
    }
}

fn format_io(context: &'static str) -> impl FnOnce(std::io::Error) -> String {
    move |error| format!("Failed to {context}: {error}")
}
