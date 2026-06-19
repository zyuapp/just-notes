use tauri::State;

use crate::{
    app::AppPaths,
    transcription::{
        transcription_status_with_downloads, ModelDownloadState, TranscriptionStatusPayload,
    },
};

#[tauri::command]
pub(crate) fn get_transcription_status(
    paths: State<'_, AppPaths>,
    downloads: State<'_, ModelDownloadState>,
) -> Result<TranscriptionStatusPayload, String> {
    Ok(transcription_status_with_downloads(&paths, &downloads))
}

#[tauri::command]
pub(crate) fn start_transcription_model_download(
    paths: State<'_, AppPaths>,
    downloads: State<'_, ModelDownloadState>,
) -> Result<TranscriptionStatusPayload, String> {
    downloads.start_download(paths.inner().clone(), Box::new(|| {}))?;
    Ok(transcription_status_with_downloads(&paths, &downloads))
}

#[tauri::command]
pub(crate) fn cancel_transcription_model_download(
    paths: State<'_, AppPaths>,
    downloads: State<'_, ModelDownloadState>,
) -> Result<TranscriptionStatusPayload, String> {
    downloads.cancel_download();
    Ok(transcription_status_with_downloads(&paths, &downloads))
}
