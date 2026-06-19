use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use crate::app::{AppPaths, ARCHIVED_DIR_NAME};

#[derive(serde::Serialize, serde::Deserialize, ts_rs::TS, Clone)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub(crate) struct AppSettings {
    pub(crate) transcripts_dir: Option<String>,
    pub(crate) save_raw_audio: bool,
    pub(crate) markdown_copy: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            transcripts_dir: None,
            save_raw_audio: true,
            markdown_copy: true,
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct SettingsState(Arc<Mutex<AppSettings>>);

impl SettingsState {
    pub(crate) fn new(settings: AppSettings) -> Self {
        Self(Arc::new(Mutex::new(settings)))
    }

    pub(crate) fn snapshot(&self) -> AppSettings {
        self.0
            .lock()
            .map(|settings| settings.clone())
            .unwrap_or_default()
    }

    pub(crate) fn replace(&self, settings: AppSettings) {
        if let Ok(mut slot) = self.0.lock() {
            *slot = settings;
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

pub(crate) fn validate_settings(base: &AppPaths, settings: &AppSettings) -> Result<(), String> {
    let Some(dir) = settings.transcripts_dir.as_deref() else {
        return Ok(());
    };
    let dir = Path::new(dir);
    if !dir.is_absolute() {
        return Err("The transcripts folder must be an absolute path".to_string());
    }
    fs::create_dir_all(dir).map_err(|err| {
        format!(
            "The transcripts folder {} is not writable: {err}",
            dir.display()
        )
    })?;

    // The data folder holds the archive store. If the transcripts folder
    // contained or sat inside it, archived recordings would land back under the
    // folder agents crawl, defeating the point of archiving.
    let dir = dir.canonicalize().map_err(|err| {
        format!(
            "Cannot resolve the transcripts folder {}: {err}",
            dir.display()
        )
    })?;
    let data_dir = base
        .data_dir
        .canonicalize()
        .unwrap_or_else(|_| base.data_dir.clone());
    let archived_dir = data_dir.join(ARCHIVED_DIR_NAME);
    if data_dir.starts_with(&dir) || dir.starts_with(&archived_dir) {
        return Err(
            "The transcripts folder can't contain or sit inside the Just Notes data \
             folder, where archived recordings are stored. Pick a different folder."
                .to_string(),
        );
    }
    Ok(())
}

pub(crate) fn effective_paths(base: &AppPaths, settings: &AppSettings) -> AppPaths {
    let threads_dir = settings
        .transcripts_dir
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| base.data_dir.join("threads"));
    AppPaths {
        threads_dir,
        // The archive store stays under the data dir regardless of a custom
        // transcripts folder, so it never lands inside the crawled corpus.
        archived_dir: base.archived_dir.clone(),
        data_dir: base.data_dir.clone(),
    }
}

#[cfg(test)]
mod tests;
