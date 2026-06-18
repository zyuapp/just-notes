use crate::app::AppPaths;

use super::{
    finalization_transcription_catalog, TranscriptionProvider, TranscriptionStatusPayload,
};

pub(crate) fn transcription_status(
    paths: &AppPaths,
    provider: TranscriptionProvider,
) -> TranscriptionStatusPayload {
    let catalog = finalization_transcription_catalog(paths, provider);
    let selection = catalog.selection;
    let model_exists = catalog
        .available_models
        .iter()
        .any(|model| model.selected && model.installed);
    let message = match model_exists {
        true => format!(
            "{} transcription is ready ({})",
            selection.provider.display_name(),
            selection.model_name
        ),
        false => format!(
            "{} transcription model is missing",
            selection.provider.display_name()
        ),
    };

    TranscriptionStatusPayload {
        ready: model_exists,
        engine_exists: true,
        model_exists,
        engine_path: selection.provider.runtime_name().to_string(),
        model_path: selection.model_path.display().to_string(),
        model_name: selection.model_name,
        provider: selection.provider,
        available_models: catalog.available_models,
        message,
    }
}
