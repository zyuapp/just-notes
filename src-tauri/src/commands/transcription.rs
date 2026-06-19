use tauri::State;

use crate::{
    app::AppPaths,
    settings::SettingsState,
    transcription::{
        transcription_status_with_downloads, ModelDownloadState, TranscriptionProvider,
        TranscriptionStatusPayload,
    },
};

#[tauri::command]
pub(crate) fn get_transcription_status(
    paths: State<'_, AppPaths>,
    settings: State<'_, SettingsState>,
    downloads: State<'_, ModelDownloadState>,
) -> Result<TranscriptionStatusPayload, String> {
    Ok(transcription_status_with_downloads(
        &paths,
        settings.snapshot().transcription_provider.into(),
        &downloads,
    ))
}

#[tauri::command]
pub(crate) fn start_transcription_model_download(
    paths: State<'_, AppPaths>,
    settings: State<'_, SettingsState>,
    downloads: State<'_, ModelDownloadState>,
    provider: TranscriptionProvider,
) -> Result<TranscriptionStatusPayload, String> {
    downloads.start_download(provider, paths.inner().clone(), settings.inner().clone())?;
    Ok(transcription_status_with_downloads(
        &paths,
        settings.snapshot().transcription_provider.into(),
        &downloads,
    ))
}

#[tauri::command]
pub(crate) fn cancel_transcription_model_download(
    paths: State<'_, AppPaths>,
    settings: State<'_, SettingsState>,
    downloads: State<'_, ModelDownloadState>,
    provider: TranscriptionProvider,
) -> Result<TranscriptionStatusPayload, String> {
    downloads.cancel_download(provider);
    Ok(transcription_status_with_downloads(
        &paths,
        settings.snapshot().transcription_provider.into(),
        &downloads,
    ))
}
