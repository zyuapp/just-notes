use tauri::{AppHandle, State};

use crate::{
    app::AppPaths,
    meeting_surfaces, meetings,
    settings::{save_settings, AppSettings, SettingsState},
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
    save_settings(&paths.data_dir, &settings)?;
    state.replace(settings.clone());
    meetings::settings_updated(&app, &previous, &settings);
    meeting_surfaces::sync_current(&app);
    Ok(settings)
}
