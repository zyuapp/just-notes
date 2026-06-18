use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use crate::app::AppPaths;

const LEGACY_WHISPER_MODEL_FILES: [&str; 3] = [
    "ggml-medium.en.bin",
    "ggml-small.en.bin",
    "ggml-base.en.bin",
];
const PARAKEET_MODEL_ID: &str = "sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8";
const PARAKEET_MODEL_FILES: [&str; 4] = [
    "encoder.int8.onnx",
    "decoder.int8.onnx",
    "joiner.int8.onnx",
    "tokens.txt",
];

#[derive(serde::Serialize, serde::Deserialize, ts_rs::TS, Clone)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub(crate) struct AppSettings {
    pub(crate) transcripts_dir: Option<String>,
    pub(crate) save_raw_audio: bool,
    pub(crate) markdown_copy: bool,
    pub(crate) transcription_provider: TranscriptionProviderPreference,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            transcripts_dir: None,
            save_raw_audio: true,
            markdown_copy: true,
            transcription_provider: TranscriptionProviderPreference::default(),
        }
    }
}

#[derive(
    serde::Serialize, serde::Deserialize, ts_rs::TS, Clone, Copy, Debug, Default, PartialEq, Eq,
)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) enum TranscriptionProviderPreference {
    #[default]
    Parakeet,
    Whisper,
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
        return settings_with_upgrade_provider(data_dir);
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&json) else {
        return AppSettings::default();
    };
    let missing_provider = value.get("transcriptionProvider").is_none();
    let mut settings = serde_json::from_value::<AppSettings>(value).unwrap_or_default();
    if missing_provider {
        settings.transcription_provider = default_provider_for_upgrade(data_dir);
    }
    settings
}

fn settings_with_upgrade_provider(data_dir: &Path) -> AppSettings {
    AppSettings {
        transcription_provider: default_provider_for_upgrade(data_dir),
        ..AppSettings::default()
    }
}

fn default_provider_for_upgrade(data_dir: &Path) -> TranscriptionProviderPreference {
    if !parakeet_model_exists(data_dir) && whisper_model_exists(data_dir) {
        TranscriptionProviderPreference::Whisper
    } else {
        TranscriptionProviderPreference::default()
    }
}

fn parakeet_model_exists(data_dir: &Path) -> bool {
    let model_dir = data_dir
        .join("models")
        .join("parakeet")
        .join(PARAKEET_MODEL_ID);
    PARAKEET_MODEL_FILES
        .iter()
        .all(|filename| model_dir.join(filename).is_file())
}

fn whisper_model_exists(data_dir: &Path) -> bool {
    let model_dir = data_dir.join("models").join("whisper");
    LEGACY_WHISPER_MODEL_FILES
        .iter()
        .any(|filename| model_dir.join(filename).is_file())
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

pub(crate) fn validate_settings(settings: &AppSettings) -> Result<(), String> {
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
    })
}

pub(crate) fn effective_paths(base: &AppPaths, settings: &AppSettings) -> AppPaths {
    let threads_dir = settings
        .transcripts_dir
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| base.data_dir.join("threads"));
    AppPaths {
        data_dir: base.data_dir.clone(),
        threads_dir,
    }
}

#[cfg(test)]
mod tests;
