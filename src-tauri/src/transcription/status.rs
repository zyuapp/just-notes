use crate::app::AppPaths;

use super::{
    finalization_transcription_catalog, ModelDownloadState, TranscriptionModelDownloadState,
    TranscriptionProvider, TranscriptionStatusPayload,
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

pub(crate) fn transcription_status_with_downloads(
    paths: &AppPaths,
    provider: TranscriptionProvider,
    downloads: &ModelDownloadState,
) -> TranscriptionStatusPayload {
    let mut status = transcription_status(paths, provider);
    for model in &mut status.available_models {
        if !model.downloadable {
            continue;
        }
        let snapshot = downloads.snapshot_for(model.provider, model.total_bytes);
        model.download_state = snapshot.state;
        model.progress_bytes = snapshot.progress_bytes;
        model.total_bytes = snapshot.total_bytes;
        model.can_cancel = matches!(
            snapshot.state,
            TranscriptionModelDownloadState::Downloading
                | TranscriptionModelDownloadState::Installing
        );
        let download_active = downloads.download_running(model.provider)
            || matches!(
                snapshot.state,
                TranscriptionModelDownloadState::Downloading
                    | TranscriptionModelDownloadState::Installing
            );
        model.can_download = !model.installed && !download_active;
        model.error_message = snapshot.error_message;
        if model.selected && !model.installed {
            status.message = selected_model_message(
                model.provider,
                model.installed,
                model.download_state,
                model.progress_bytes,
                model.total_bytes,
            );
        }
    }
    status
}

fn selected_model_message(
    provider: TranscriptionProvider,
    installed: bool,
    download_state: TranscriptionModelDownloadState,
    progress_bytes: u64,
    total_bytes: u64,
) -> String {
    if installed {
        return format!("{} transcription is ready", provider.display_name());
    }
    match download_state {
        TranscriptionModelDownloadState::Downloading => {
            let percent = if total_bytes == 0 {
                0
            } else {
                progress_bytes.saturating_mul(100) / total_bytes
            };
            format!("Downloading {} model ({percent}%)", provider.display_name())
        }
        TranscriptionModelDownloadState::Installing => {
            format!("Installing {} model", provider.display_name())
        }
        TranscriptionModelDownloadState::Failed => {
            format!("{} model download failed", provider.display_name())
        }
        TranscriptionModelDownloadState::Cancelled => {
            format!("{} model download cancelled", provider.display_name())
        }
        TranscriptionModelDownloadState::Idle => {
            format!("{} transcription model is missing", provider.display_name())
        }
    }
}
