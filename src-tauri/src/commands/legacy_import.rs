use tauri::{AppHandle, State};

use crate::{
    app::AppPaths,
    ipc::{LegacyImportPreview, LegacyImportResult},
    legacy_import::{self, LegacyImportState},
    recording::RecorderState,
    threads::import::{LegacyImportPreviewData, LegacyImportResultData},
    transcription::FinalizeState,
};

#[tauri::command]
pub(crate) async fn prepare_legacy_import(
    app: AppHandle,
    paths: State<'_, AppPaths>,
    session: State<'_, LegacyImportState>,
) -> Result<Option<LegacyImportPreview>, String> {
    legacy_import::prepare(app, paths.inner().clone(), session.inner().clone())
        .await
        .map(|preview| preview.map(LegacyImportPreview::from))
}

#[tauri::command]
pub(crate) async fn confirm_legacy_import(
    app: AppHandle,
    paths: State<'_, AppPaths>,
    recorder: State<'_, RecorderState>,
    finalize: State<'_, FinalizeState>,
    session: State<'_, LegacyImportState>,
) -> Result<LegacyImportResult, String> {
    legacy_import::confirm(
        app,
        paths.inner().clone(),
        recorder.inner().clone(),
        finalize.inner().clone(),
        session.inner().clone(),
    )
    .await
    .map(LegacyImportResult::from)
}

#[tauri::command]
pub(crate) fn cancel_legacy_import(session: State<'_, LegacyImportState>) -> Result<(), String> {
    session.clear()
}

impl From<LegacyImportPreviewData> for LegacyImportPreview {
    fn from(value: LegacyImportPreviewData) -> Self {
        Self {
            active_recordings: value.active_recordings,
            archived_recordings: value.archived_recordings,
            duplicates: value.duplicates,
            conflicts: value.conflicts,
            bytes_to_copy: value.bytes_to_copy,
            source_path: value.source_path,
        }
    }
}

impl From<LegacyImportResultData> for LegacyImportResult {
    fn from(value: LegacyImportResultData) -> Self {
        Self {
            imported: value.imported,
            active_recordings: value.active_recordings,
            archived_recordings: value.archived_recordings,
            duplicates: value.duplicates,
            conflicts: value.conflicts,
        }
    }
}
