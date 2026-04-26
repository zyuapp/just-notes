use crate::app::AppPaths;

use super::TranscriptionStatusPayload;

pub(crate) fn transcription_status(paths: &AppPaths) -> TranscriptionStatusPayload {
    let transcription_paths = paths.transcription_paths();
    let model_exists = transcription_paths.model_path.is_file();
    let message = match model_exists {
        true => format!(
            "Local transcription is ready ({})",
            transcription_paths.model_name
        ),
        false => "Local transcription model is missing".to_string(),
    };

    TranscriptionStatusPayload {
        ready: model_exists,
        engine_exists: true,
        model_exists,
        engine_path: "embedded whisper.cpp runtime".to_string(),
        model_path: transcription_paths.model_path.display().to_string(),
        model_name: transcription_paths.model_name,
        available_models: transcription_paths.available_models,
        message,
    }
}
