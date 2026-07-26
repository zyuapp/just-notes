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
    mut settings: AppSettings,
) -> Result<AppSettings, String> {
    let previous = state.snapshot();
    meetings::normalize_settings(&mut settings);
    save_settings(&paths.data_dir, &settings)?;
    meetings::settings_updated(&app, &previous, &settings);
    state.replace(settings.clone());
    meeting_surfaces::sync_current(&app);
    Ok(settings)
}
