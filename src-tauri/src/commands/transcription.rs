use tauri::State;

use crate::{
    app::AppPaths,
    settings::{SettingsState, TranscriptionProviderPreference},
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
        selected_transcription_provider(settings.snapshot().transcription_provider),
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
        selected_transcription_provider(settings.snapshot().transcription_provider),
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
        selected_transcription_provider(settings.snapshot().transcription_provider),
        &downloads,
    ))
}

fn selected_transcription_provider(
    provider: TranscriptionProviderPreference,
) -> TranscriptionProvider {
    match provider {
        TranscriptionProviderPreference::Parakeet => TranscriptionProvider::Parakeet,
        TranscriptionProviderPreference::Whisper => TranscriptionProvider::Whisper,
    }
}
