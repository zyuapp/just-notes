use tauri::{AppHandle, Manager, State};

use crate::{
    app::{AppPaths, StorageGate},
    meetings,
    platform::{self, FolderAccessState},
    recording::RecorderState,
    settings::{
        save_persisted_settings, validate_settings, AppPreferencesUpdate, AppSettings,
        SettingsState,
    },
    transcription::FinalizeState,
};

fn with_storage_change<T>(
    app: &AppHandle,
    operation: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let recorder = app.state::<RecorderState>();
    let finalize = app.state::<FinalizeState>();
    let gate = app.state::<StorageGate>();
    let _guard = gate.lock()?;
    if recorder.is_busy() || finalize.is_active() {
        return Err(
            "Wait for the current recording or transcription to finish before changing storage"
                .to_string(),
        );
    }
    operation()
}

#[tauri::command]
pub(crate) fn get_settings(settings: State<'_, SettingsState>) -> AppSettings {
    settings.snapshot()
}

#[tauri::command]
pub(crate) fn update_settings(
    app: AppHandle,
    paths: State<'_, AppPaths>,
    state: State<'_, SettingsState>,
    preferences: AppPreferencesUpdate,
) -> Result<AppSettings, String> {
    let previous = state.snapshot();
    let mut persisted = state.persisted_snapshot();
    persisted.replace_preferences(preferences);
    save_persisted_settings(&paths.data_dir, &persisted)?;
    state.replace_persisted(persisted);
    let saved = state.snapshot();
    meetings::settings_updated(&app, &previous, &saved);
    Ok(saved)
}

#[tauri::command]
pub(crate) async fn choose_transcripts_folder(
    app: AppHandle,
    paths: State<'_, AppPaths>,
    state: State<'_, SettingsState>,
    access: State<'_, FolderAccessState>,
) -> Result<Option<AppSettings>, String> {
    let paths = paths.inner().clone();
    let state = state.inner().clone();
    let access = access.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        with_storage_change(&app, || {
            let Some(choice) =
                platform::choose_folder(&app, "Choose where Just Notes saves transcripts", false)?
            else {
                return Ok(None);
            };
            let mut persisted = state.persisted_snapshot();
            persisted.set_transcripts_folder(choice.path.clone(), choice.bookmark.clone());
            validate_settings(&paths, persisted.settings())?;
            save_persisted_settings(&paths.data_dir, &persisted)?;
            access.install(choice)?;
            let saved = persisted.settings().clone();
            state.replace_authorized(persisted);
            Ok(Some(saved))
        })
    })
    .await
    .map_err(|err| format!("Folder picker task failed: {err}"))?
}

#[tauri::command]
pub(crate) async fn import_legacy_data(
    app: AppHandle,
    paths: State<'_, AppPaths>,
    state: State<'_, SettingsState>,
) -> Result<Option<AppSettings>, String> {
    let paths = paths.inner().clone();
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || run_legacy_import(&app, &paths, &state))
        .await
        .map_err(|err| format!("Legacy import task failed: {err}"))?
}

fn run_legacy_import(
    app: &AppHandle,
    paths: &AppPaths,
    state: &SettingsState,
) -> Result<Option<AppSettings>, String> {
    with_storage_change(app, || {
        ensure_default_storage(state)?;
        let previous_settings = state.persisted_snapshot();
        let Some(choice) =
            platform::choose_folder(app, "Choose your existing .just-notes folder", true)?
        else {
            return Ok(None);
        };
        let source = std::path::Path::new(&choice.path);
        let has_legacy_settings = source.join("settings.json").is_file();
        let mut imported =
            has_legacy_settings.then(|| crate::settings::load_persisted_settings(source));
        let custom_choice = choose_custom_legacy_folder(app, imported.as_ref())?;
        let imported_settings = if let Some(mut imported) = imported.take() {
            // Old custom paths have no sandbox authorization. Imported notes
            // use container storage until the user explicitly chooses again.
            imported.use_default_transcripts_folder();
            save_persisted_settings(&paths.data_dir, &imported)?;
            let saved = imported.settings().clone();
            state.replace_authorized(imported);
            Some(saved)
        } else {
            None
        };
        let import_result = crate::app::migration::import_legacy_data(
            source,
            custom_choice
                .as_ref()
                .map(|folder| std::path::Path::new(&folder.path)),
            &paths.data_dir,
        );
        if let Err(import_error) = import_result {
            return restore_after_failed_import(
                paths,
                state,
                previous_settings,
                imported_settings.is_some(),
                import_error,
            );
        }
        Ok(Some(imported_settings.unwrap_or_else(|| state.snapshot())))
    })
}

fn choose_custom_legacy_folder(
    app: &AppHandle,
    imported: Option<&crate::settings::PersistedSettings>,
) -> Result<Option<platform::FolderChoice>, String> {
    let Some(custom_path) = imported.and_then(|value| value.settings().transcripts_dir.as_deref())
    else {
        return Ok(None);
    };
    let prompt = format!("Choose the previous custom transcripts folder ({custom_path})");
    platform::choose_folder(app, &prompt, false)?
        .ok_or_else(|| {
            "The legacy import needs access to the previous custom transcripts folder".to_string()
        })
        .map(Some)
}

fn restore_after_failed_import(
    paths: &AppPaths,
    state: &SettingsState,
    previous: crate::settings::PersistedSettings,
    settings_changed: bool,
    import_error: String,
) -> Result<Option<AppSettings>, String> {
    if settings_changed {
        save_persisted_settings(&paths.data_dir, &previous).map_err(|rollback| {
            format!("{import_error}. Restoring previous settings also failed: {rollback}")
        })?;
        state.replace_authorized(previous);
    }
    Err(import_error)
}

fn ensure_default_storage(state: &SettingsState) -> Result<(), String> {
    if state
        .persisted_snapshot()
        .settings()
        .transcripts_dir
        .is_some()
    {
        return Err(
            "Use Default storage before importing data from an earlier version".to_string(),
        );
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn use_default_transcripts_folder(
    app: AppHandle,
    paths: State<'_, AppPaths>,
    state: State<'_, SettingsState>,
    access: State<'_, FolderAccessState>,
) -> Result<AppSettings, String> {
    with_storage_change(&app, || {
        let mut persisted = state.persisted_snapshot();
        persisted.use_default_transcripts_folder();
        save_persisted_settings(&paths.data_dir, &persisted)?;
        access.clear()?;
        let saved = persisted.settings().clone();
        state.replace_authorized(persisted);
        Ok(saved)
    })
}
