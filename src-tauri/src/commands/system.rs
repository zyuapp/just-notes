use tauri::{AppHandle, Manager, State};

use crate::{
    app::AppPaths,
    capture::{microphone_permission_status, system_audio_permission_status},
    ipc::{AppInfo, PermissionsPayload},
    platform,
};

#[tauri::command]
pub(crate) fn get_app_info(app: AppHandle, paths: State<'_, AppPaths>) -> AppInfo {
    AppInfo {
        data_dir: paths.data_dir.display().to_string(),
        threads_dir: paths.threads_dir.display().to_string(),
        fixture_mode: cfg!(any(debug_assertions, feature = "qa-fixtures")),
        version: app.package_info().version.to_string(),
    }
}

#[tauri::command]
pub(crate) async fn get_permissions_status(app: AppHandle) -> Result<PermissionsPayload, String> {
    tauri::async_runtime::spawn_blocking(move || {
        Ok(PermissionsPayload {
            microphone: microphone_permission_status(&app)?,
            system_audio: system_audio_permission_status(),
        })
    })
    .await
    .map_err(|err| format!("Permission check task failed: {err}"))?
}

#[tauri::command]
pub(crate) fn reveal_in_finder(app: AppHandle, path: String) -> Result<(), String> {
    platform::reveal_in_finder(&app, &path)
}

#[tauri::command]
pub(crate) fn copy_text_to_clipboard(app: AppHandle, text: String) -> Result<(), String> {
    platform::copy_to_clipboard(&app, &text)
}

#[tauri::command]
pub(crate) fn open_privacy_settings(app: AppHandle, pane: String) -> Result<(), String> {
    platform::open_privacy_settings(&app, &pane)
}

#[tauri::command]
pub(crate) fn open_external_url(app: AppHandle, url: String) -> Result<(), String> {
    platform::open_https_url(&app, &url)
}

#[tauri::command]
pub(crate) fn open_legal_document(app: AppHandle, document: String) -> Result<(), String> {
    let filename = match document.as_str() {
        "privacy" => "PrivacyPolicy.md",
        "notices" => "ThirdPartyNotices.txt",
        _ => return Err("Unknown legal document".to_string()),
    };
    let path = app
        .path()
        .resource_dir()
        .map_err(|err| format!("Cannot locate app resources: {err}"))?
        .join("Legal")
        .join(filename);
    platform::open_file(&app, &path)
}
