pub(crate) mod indicator;
pub(crate) mod recording;
pub(crate) mod settings;
pub(crate) mod system;
pub(crate) mod threads;
pub(crate) mod transcription;

use crate::{
    app::AppPaths,
    settings::{SettingsState, TranscriptionProviderPreference},
    transcription::TranscriptionProvider,
};

fn effective_paths(paths: &AppPaths, settings: &SettingsState) -> AppPaths {
    crate::settings::effective_paths(paths, &settings.snapshot())
}

impl From<TranscriptionProviderPreference> for TranscriptionProvider {
    fn from(preference: TranscriptionProviderPreference) -> Self {
        match preference {
            TranscriptionProviderPreference::Parakeet => Self::Parakeet,
            TranscriptionProviderPreference::Whisper => Self::Whisper,
        }
    }
}

impl From<TranscriptionProvider> for TranscriptionProviderPreference {
    fn from(provider: TranscriptionProvider) -> Self {
        match provider {
            TranscriptionProvider::Parakeet => Self::Parakeet,
            TranscriptionProvider::Whisper => Self::Whisper,
        }
    }
}
