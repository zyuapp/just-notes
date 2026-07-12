use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[cfg(test)]
use super::model::AppPreferencesUpdate;
use super::model::{AppSettings, PersistedSettings};
use crate::app::{AppPaths, ARCHIVED_DIR_NAME};

#[derive(Default)]
struct SettingsSlot {
    persisted: PersistedSettings,
    storage_mode: StorageMode,
}

#[derive(Default)]
enum StorageMode {
    #[default]
    Default,
    AuthorizedCustom,
    UnavailableCustom,
}

fn authorized_mode(settings: &PersistedSettings) -> StorageMode {
    if settings.settings.transcripts_dir.is_some() {
        StorageMode::AuthorizedCustom
    } else {
        StorageMode::Default
    }
}

#[derive(Clone, Default)]
pub(crate) struct SettingsState(Arc<Mutex<SettingsSlot>>);

impl SettingsState {
    #[cfg(test)]
    pub(crate) fn new(settings: AppSettings) -> Self {
        Self::from_persisted(PersistedSettings::new(settings))
    }

    pub(crate) fn from_persisted(settings: PersistedSettings) -> Self {
        Self(Arc::new(Mutex::new(SettingsSlot {
            storage_mode: authorized_mode(&settings),
            persisted: settings,
        })))
    }

    pub(crate) fn from_unavailable_folder(settings: PersistedSettings) -> Self {
        Self(Arc::new(Mutex::new(SettingsSlot {
            persisted: settings,
            storage_mode: StorageMode::UnavailableCustom,
        })))
    }

    pub(crate) fn snapshot(&self) -> AppSettings {
        self.0
            .lock()
            .map(|slot| {
                let mut settings = slot.persisted.settings.clone();
                if matches!(slot.storage_mode, StorageMode::UnavailableCustom) {
                    settings.transcripts_dir = None;
                    settings.transcripts_folder_unavailable = true;
                }
                settings
            })
            .unwrap_or_default()
    }

    pub(crate) fn persisted_snapshot(&self) -> PersistedSettings {
        self.0
            .lock()
            .map(|slot| slot.persisted.clone())
            .unwrap_or_default()
    }

    pub(crate) fn replace_persisted(&self, settings: PersistedSettings) {
        if let Ok(mut current) = self.0.lock() {
            current.persisted = settings;
        }
    }

    pub(crate) fn replace_authorized(&self, settings: PersistedSettings) {
        if let Ok(mut current) = self.0.lock() {
            current.persisted = settings;
            current.storage_mode = authorized_mode(&current.persisted);
        }
    }
}

pub(crate) fn settings_path(data_dir: &Path) -> PathBuf {
    data_dir.join("settings.json")
}

#[cfg(test)]
pub(crate) fn load_settings(data_dir: &Path) -> AppSettings {
    load_persisted_settings(data_dir).into_settings()
}

pub(crate) fn load_persisted_settings(data_dir: &Path) -> PersistedSettings {
    let path = settings_path(data_dir);
    let Ok(json) = fs::read_to_string(&path) else {
        return PersistedSettings::default();
    };
    serde_json::from_str::<PersistedSettings>(&json).unwrap_or_default()
}

#[cfg(test)]
pub(crate) fn save_settings(data_dir: &Path, settings: &AppSettings) -> Result<(), String> {
    let mut persisted = load_persisted_settings(data_dir);
    persisted.replace_all_settings(settings.clone());
    save_persisted_settings(data_dir, &persisted)
}

pub(crate) fn save_persisted_settings(
    data_dir: &Path,
    settings: &PersistedSettings,
) -> Result<(), String> {
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
