use tauri::{AppHandle, State};

use super::effective_paths;
use crate::{
    app::AppPaths,
    capture::microphone_permission_status,
    ipc::{AppInfo, PermissionsPayload},
    platform,
    settings::{SettingsState, TranscriptionProviderPreference},
    transcription::{transcription_status, TranscriptionProvider, TranscriptionStatusPayload},
};

#[tauri::command]
pub(crate) fn get_app_info(
    paths: State<'_, AppPaths>,
    settings: State<'_, SettingsState>,
) -> AppInfo {
    let paths = effective_paths(&paths, &settings);
    AppInfo {
        data_dir: paths.data_dir.display().to_string(),
        threads_dir: paths.threads_dir.display().to_string(),
        fixture_mode: cfg!(any(debug_assertions, feature = "qa-fixtures")),
    }
}

#[tauri::command]
pub(crate) fn get_transcription_status(
    paths: State<'_, AppPaths>,
    settings: State<'_, SettingsState>,
) -> Result<TranscriptionStatusPayload, String> {
    Ok(transcription_status(
        &paths,
        selected_transcription_provider(settings.snapshot().transcription_provider),
    ))
}

#[tauri::command]
pub(crate) async fn get_permissions_status(app: AppHandle) -> Result<PermissionsPayload, String> {
    tauri::async_runtime::spawn_blocking(move || {
        Ok(PermissionsPayload {
            microphone: microphone_permission_status(&app)?,
            system_audio: "unknown".to_string(),
        })
    })
    .await
    .map_err(|err| format!("Permission check task failed: {err}"))?
}

#[tauri::command]
pub(crate) fn reveal_in_finder(path: String) -> Result<(), String> {
    platform::reveal_in_finder(&path)
}

#[tauri::command]
pub(crate) fn copy_text_to_clipboard(text: String) -> Result<(), String> {
    platform::copy_to_clipboard(&text)
}

#[tauri::command]
pub(crate) fn open_privacy_settings(pane: String) -> Result<(), String> {
    platform::open_privacy_settings(&pane)
}

fn selected_transcription_provider(
    provider: TranscriptionProviderPreference,
) -> TranscriptionProvider {
    match provider {
        TranscriptionProviderPreference::Parakeet => TranscriptionProvider::Parakeet,
        TranscriptionProviderPreference::Whisper => TranscriptionProvider::Whisper,
    }
}
