use tauri::{AppHandle, State};

use crate::{
    app::AppPaths,
    meetings, platform,
    settings::{save_settings, validate_settings, AppSettings, SettingsState},
};

#[tauri::command]
pub(crate) fn get_settings(settings: State<'_, SettingsState>) -> AppSettings {
    settings.snapshot()
}

#[tauri::command]
pub(crate) fn update_settings(
    app: AppHandle,
    paths: State<'_, AppPaths>,
    state: State<'_, SettingsState>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    let previous = state.snapshot();
    validate_settings(&paths, &settings)?;
    save_settings(&paths.data_dir, &settings)?;
    state.replace(settings.clone());
    meetings::settings_updated(&app, &previous, &settings);
    Ok(settings)
}

#[tauri::command]
pub(crate) async fn pick_folder() -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        platform::choose_folder("Choose where Just Notes saves transcripts")
    })
    .await
    .map_err(|err| format!("Folder picker task failed: {err}"))?
}
