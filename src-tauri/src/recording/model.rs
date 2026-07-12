use crate::{threads::ThreadDetail, transcription::TranscriptionStatusPayload};

pub(crate) struct StartedRecording {
    pub(crate) thread: ThreadDetail,
    pub(crate) transcription: TranscriptionStatusPayload,
}
