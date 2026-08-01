use crate::{
    agent_access::{AgentGuideState, AgentGuideStatus, AgentId},
    threads::{ThreadDetail, TranscriptSegment},
    transcription::TranscriptionStatusPayload,
};

#[derive(serde::Serialize, ts_rs::TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) struct AgentGuideStatusPayload {
    pub(crate) agent: AgentId,
    pub(crate) state: AgentGuideState,
    pub(crate) path: String,
    pub(crate) detail: Option<String>,
}

impl From<AgentGuideStatus> for AgentGuideStatusPayload {
    fn from(status: AgentGuideStatus) -> Self {
        Self {
            agent: status.agent,
            state: status.state,
            path: status.path,
            detail: status.detail,
        }
    }
}

#[derive(serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) struct AppInfo {
    pub(crate) data_dir: String,
    pub(crate) threads_dir: String,
    pub(crate) fixture_mode: bool,
    pub(crate) version: String,
}

#[derive(serde::Serialize, ts_rs::TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) struct MeterPayload {
    pub(crate) thread_id: String,
    pub(crate) mic_level: f32,
    pub(crate) system_level: f32,
    pub(crate) elapsed_ms: u64,
}

#[derive(serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) struct RecordingPayload {
    pub(crate) thread: ThreadDetail,
    pub(crate) transcription: TranscriptionStatusPayload,
}

#[derive(serde::Serialize, ts_rs::TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) struct LiveTranscriptPayload {
    pub(crate) thread_id: String,
    pub(crate) segment: TranscriptSegment,
}

#[derive(serde::Serialize, ts_rs::TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) struct StorageUsagePayload {
    pub(crate) total_bytes: u64,
    pub(crate) raw_audio_bytes: u64,
    pub(crate) reclaimable_bytes: u64,
    pub(crate) thread_count: u64,
}

#[derive(serde::Serialize, ts_rs::TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) struct PermissionsPayload {
    pub(crate) microphone: String,
    pub(crate) system_audio: String,
}

#[derive(serde::Serialize, ts_rs::TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) struct MeetingCalendarPayload {
    pub(crate) id: String,
    pub(crate) title: String,
    /// Owning account, e.g. "iCloud" or a Google address. Calendars from
    /// different accounts frequently share a title, so this disambiguates them.
    pub(crate) account: String,
    /// `#rrggbb`, matching the calendar's colour in macOS Calendar.
    pub(crate) color: String,
}

#[derive(serde::Serialize, ts_rs::TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) struct MeetingAccessPayload {
    pub(crate) calendar_authorization: String,
    pub(crate) notification_authorization: String,
    pub(crate) calendars: Vec<MeetingCalendarPayload>,
}

#[derive(serde::Serialize, ts_rs::TS, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) struct MeetingPromptPayload {
    pub(crate) request_id: String,
    pub(crate) title: String,
    pub(crate) start_at_ms: u64,
    pub(crate) end_at_ms: u64,
    pub(crate) auto_start: bool,
}
