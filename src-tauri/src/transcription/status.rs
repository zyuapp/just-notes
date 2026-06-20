use crate::app::AppPaths;

use super::{
    finalization_transcription_catalog,
    models::{TranscriptionModelStatus, PARAKEET_DISPLAY_NAME, PARAKEET_RUNTIME_NAME},
    ModelDownloadState, TranscriptionModelDownloadState, TranscriptionStatusPayload,
};

pub(crate) fn transcription_status(paths: &AppPaths) -> TranscriptionStatusPayload {
    let catalog = finalization_transcription_catalog(paths);
    let selection = catalog.selection;
    let model_exists = catalog
        .available_models
        .iter()
        .any(|model| model.selected && model.installed);
    let message = match model_exists {
        true => "Ready".to_string(),
        false => format!("{PARAKEET_DISPLAY_NAME} transcription model is missing"),
    };

    TranscriptionStatusPayload {
        ready: model_exists,
        engine_exists: true,
        model_exists,
        engine_path: PARAKEET_RUNTIME_NAME.to_string(),
        model_path: selection.model_path.display().to_string(),
        model_name: selection.model_name,
        available_models: catalog.available_models,
        message,
    }
}

pub(crate) fn transcription_status_with_downloads(
    paths: &AppPaths,
    downloads: &ModelDownloadState,
) -> TranscriptionStatusPayload {
    let mut status = transcription_status(paths);
    for model in &mut status.available_models {
        if model.downloadable {
            apply_download_state(model, downloads);
        }
    }
    if let Some(message) = selected_download_message(&status.available_models) {
        status.message = message;
    }
    status
}

fn apply_download_state(model: &mut TranscriptionModelStatus, downloads: &ModelDownloadState) {
    let snapshot = downloads.snapshot_for(model.total_bytes);
    let active = snapshot.active();
    model.download_state = snapshot.state;
    model.progress_bytes = snapshot.progress_bytes;
    model.total_bytes = snapshot.total_bytes;
    model.can_cancel = active;
    model.can_download = !model.installed && !active && !downloads.download_running();
    model.error_message = snapshot.error_message;
}

fn selected_download_message(models: &[TranscriptionModelStatus]) -> Option<String> {
    let model = models
        .iter()
        .find(|model| model.selected && !model.installed)?;
    Some(selected_model_message(
        model.download_state,
        model.progress_bytes,
        model.total_bytes,
    ))
}

fn selected_model_message(
    download_state: TranscriptionModelDownloadState,
    progress_bytes: u64,
    total_bytes: u64,
) -> String {
    match download_state {
        TranscriptionModelDownloadState::Downloading => {
            let percent = if total_bytes == 0 {
                0
            } else {
                progress_bytes.saturating_mul(100) / total_bytes
            };
            format!("Downloading {PARAKEET_DISPLAY_NAME} model ({percent}%)")
        }
        TranscriptionModelDownloadState::Installing => {
            format!("Installing {PARAKEET_DISPLAY_NAME} model")
        }
        TranscriptionModelDownloadState::Failed => {
            format!("{PARAKEET_DISPLAY_NAME} model download failed")
        }
        TranscriptionModelDownloadState::Cancelled => {
            format!("{PARAKEET_DISPLAY_NAME} model download cancelled")
        }
        TranscriptionModelDownloadState::Idle => {
            format!("{PARAKEET_DISPLAY_NAME} transcription model is missing")
        }
    }
}

#[cfg(test)]
mod tests;
