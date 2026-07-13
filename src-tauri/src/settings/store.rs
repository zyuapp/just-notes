use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use super::model::AppSettings;

#[derive(Clone, Default)]
pub(crate) struct SettingsState(Arc<Mutex<AppSettings>>);

impl SettingsState {
    #[cfg(test)]
    pub(crate) fn new(settings: AppSettings) -> Self {
        Self::new_state(settings)
    }

    pub(crate) fn new_state(settings: AppSettings) -> Self {
        Self(Arc::new(Mutex::new(settings)))
    }

    pub(crate) fn snapshot(&self) -> AppSettings {
        self.0
            .lock()
            .map(|settings| settings.clone())
            .unwrap_or_default()
    }

    pub(crate) fn replace(&self, settings: AppSettings) {
        if let Ok(mut current) = self.0.lock() {
            *current = settings;
        }
    }
}

pub(crate) fn settings_path(data_dir: &Path) -> PathBuf {
    data_dir.join("settings.json")
}

pub(crate) fn load_settings(data_dir: &Path) -> AppSettings {
    let path = settings_path(data_dir);
    let Ok(json) = fs::read_to_string(&path) else {
        return AppSettings::default();
    };
    serde_json::from_str::<AppSettings>(&json).unwrap_or_default()
}

pub(crate) fn save_settings(data_dir: &Path, settings: &AppSettings) -> Result<(), String> {
    let path = settings_path(data_dir);
    let json = serde_json::to_string_pretty(settings)
        .map_err(|err| format!("Failed to encode settings: {err}"))?;
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, json)
        .map_err(|err| format!("Failed to write {}: {err}", tmp_path.display()))?;
    fs::rename(&tmp_path, &path)
        .map_err(|err| format!("Failed to replace {}: {err}", path.display()))
}

#[cfg(test)]
mod tests;
