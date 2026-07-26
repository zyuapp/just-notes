use crate::{
    threads::{CalendarProvenance, ThreadDetail},
    transcription::TranscriptionStatusPayload,
};

pub(crate) struct StartedRecording {
    pub(crate) thread: ThreadDetail,
    pub(crate) transcription: TranscriptionStatusPayload,
}

/// The calendar event a scheduled recording is being started for. The meetings
/// context builds this; `recording` maps it onto the thread domain's provenance
/// shape, so `meetings` never names a `threads` type.
pub(crate) struct ScheduledMeeting {
    pub(crate) title: String,
    pub(crate) event_id: String,
    pub(crate) calendar_id: String,
    pub(crate) attendees: Vec<String>,
    pub(crate) start_at_ms: u64,
    pub(crate) end_at_ms: u64,
}

impl ScheduledMeeting {
    /// Splits the meeting into the new thread's title and the provenance stored
    /// alongside it.
    pub(super) fn into_thread_parts(self) -> (String, CalendarProvenance) {
        (
            self.title,
            CalendarProvenance {
                event_id: self.event_id,
                calendar_id: self.calendar_id,
                attendees: self.attendees,
                start_at_ms: self.start_at_ms,
                end_at_ms: self.end_at_ms,
            },
        )
    }
}
