use crate::app::AppPaths;

use super::{finalization_transcription_catalog, TranscriptionStatusPayload};

pub(crate) fn transcription_status(paths: &AppPaths) -> TranscriptionStatusPayload {
    let catalog = finalization_transcription_catalog(paths);
    let selection = catalog.selection;
    let model_exists = selection.model_path.is_file();
    let message = match model_exists {
        true => format!("Local transcription is ready ({})", selection.model_name),
        false => "Local transcription model is missing".to_string(),
    };

    TranscriptionStatusPayload {
        ready: model_exists,
        engine_exists: true,
        model_exists,
        engine_path: selection.provider.runtime_name().to_string(),
        model_path: selection.model_path.display().to_string(),
        model_name: selection.model_name,
        available_models: catalog.available_models,
        message,
    }
}
