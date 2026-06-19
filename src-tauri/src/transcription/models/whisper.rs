use std::path::Path;

use super::{TranscriptionModelDownloadState, TranscriptionModelStatus, TranscriptionProvider};

const WHISPER_MODEL_CANDIDATES: [WhisperModelCandidate; 3] = [
    WhisperModelCandidate {
        name: "medium.en",
        filename: "ggml-medium.en.bin",
    },
    WhisperModelCandidate {
        name: "small.en",
        filename: "ggml-small.en.bin",
    },
    WhisperModelCandidate {
        name: "base.en",
        filename: "ggml-base.en.bin",
    },
];

struct WhisperModelCandidate {
    name: &'static str,
    filename: &'static str,
}

pub(super) fn discover_whisper_models(model_dir: &Path) -> Vec<TranscriptionModelStatus> {
    WHISPER_MODEL_CANDIDATES
        .iter()
        .map(|candidate| {
            let path = model_dir.join(candidate.filename);
            TranscriptionModelStatus {
                name: candidate.name.to_string(),
                filename: candidate.filename.to_string(),
                provider: TranscriptionProvider::Whisper,
                installed: path.is_file(),
                path,
                selected: false,
                downloadable: false,
                download_state: TranscriptionModelDownloadState::Idle,
                progress_bytes: 0,
                total_bytes: 0,
                display_size: String::new(),
                can_download: false,
                can_cancel: false,
                error_message: None,
            }
        })
        .collect()
}
