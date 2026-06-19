use crate::transcription::TranscriptionModelDownloadState;

#[derive(Clone)]
pub(crate) struct DownloadSnapshot {
    pub(crate) state: TranscriptionModelDownloadState,
    pub(crate) progress_bytes: u64,
    pub(crate) total_bytes: u64,
    pub(crate) error_message: Option<String>,
}

impl DownloadSnapshot {
    pub(crate) fn new(
        state: TranscriptionModelDownloadState,
        progress_bytes: u64,
        total_bytes: u64,
        error_message: Option<String>,
    ) -> Self {
        Self {
            state,
            progress_bytes,
            total_bytes,
            error_message,
        }
    }

    pub(super) fn idle(total_bytes: u64) -> Self {
        Self::new(TranscriptionModelDownloadState::Idle, 0, total_bytes, None)
    }

    pub(crate) fn active(&self) -> bool {
        matches!(
            self.state,
            TranscriptionModelDownloadState::Downloading
                | TranscriptionModelDownloadState::Installing
        )
    }
}
