use tauri::State;

use crate::{
    app::AppPaths,
    settings::{set_transcription_provider, SettingsState},
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
    let persist_paths = paths.inner().clone();
    let persist_settings = settings.inner().clone();
    downloads.start_download(
        provider,
        paths.inner().clone(),
        Box::new(move || {
            set_transcription_provider(&persist_paths, &persist_settings, provider.into());
        }),
    )?;
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
