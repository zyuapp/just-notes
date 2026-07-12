use crate::{ipc::RecordingPayload, recording::StartedRecording};

pub(crate) fn from_started(started: StartedRecording) -> RecordingPayload {
    RecordingPayload {
        thread: started.thread,
        transcription: started.transcription,
    }
}
