use crate::{
    app::AppPaths,
    settings::{save_settings, SettingsState, TranscriptionProviderPreference},
    transcription::TranscriptionProvider,
};

pub(super) fn persist_selected_provider(
    paths: &AppPaths,
    settings: &SettingsState,
    provider: TranscriptionProvider,
) {
    let preference = match provider {
        TranscriptionProvider::Parakeet => TranscriptionProviderPreference::Parakeet,
        TranscriptionProvider::Whisper => TranscriptionProviderPreference::Whisper,
    };
    let mut next_settings = settings.snapshot();
    next_settings.transcription_provider = preference;
    if save_settings(&paths.data_dir, &next_settings).is_ok() {
        settings.replace(next_settings);
    }
}
