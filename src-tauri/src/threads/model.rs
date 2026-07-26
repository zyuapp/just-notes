/// Where a thread came from, when it was started from a calendar event. Carried
/// so the thread can be found by who was in it and when, without reading the
/// transcript. `None` for manually started threads.
#[derive(serde::Serialize, serde::Deserialize, ts_rs::TS, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) struct CalendarProvenance {
    /// EventKit occurrence key: event identifier plus that occurrence's start.
    pub(crate) event_id: String,
    pub(crate) calendar_id: String,
    /// Invitees other than the current user, in invite order.
    pub(crate) attendees: Vec<String>,
    pub(crate) start_at_ms: u64,
    pub(crate) end_at_ms: u64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ThreadMetadata {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) created_at_ms: u64,
    pub(crate) updated_at_ms: u64,
    pub(crate) status: ThreadStatus,
    #[serde(default)]
    pub(crate) duration_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) calendar: Option<CalendarProvenance>,
}

#[derive(serde::Serialize, serde::Deserialize, ts_rs::TS, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) enum ThreadStatus {
    Idle,
    Recording,
    /// Never written. Retained so `thread.json` files left behind by earlier
    /// versions still deserialize; `reset_stale_recording_threads` clears them
    /// to `Idle` at startup.
    Transcribing,
}

impl ThreadStatus {
    pub(crate) fn is_busy(&self) -> bool {
        !matches!(self, Self::Idle)
    }
}

#[derive(serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) struct ThreadSummary {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) created_at_ms: u64,
    pub(crate) updated_at_ms: u64,
    pub(crate) status: ThreadStatus,
    pub(crate) segment_count: usize,
    pub(crate) duration_ms: u64,
    pub(crate) snippet: String,
    pub(crate) has_audio: bool,
    pub(crate) path: String,
    pub(crate) calendar: Option<CalendarProvenance>,
}

#[derive(serde::Serialize, serde::Deserialize, ts_rs::TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) struct TranscriptSegment {
    pub(crate) speaker: String,
    pub(crate) source: String,
    pub(crate) start_ms: u64,
    pub(crate) end_ms: u64,
    pub(crate) text: String,
}

#[derive(serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) struct ThreadDetail {
    pub(crate) summary: ThreadSummary,
    pub(crate) segments: Vec<TranscriptSegment>,
    pub(crate) transcript_markdown_path: String,
}
