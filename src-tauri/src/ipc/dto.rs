use crate::{
    threads::{ThreadDetail, TranscriptSegment},
    transcription::TranscriptionStatusPayload,
};

#[derive(serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) struct AppInfo {
    pub(crate) data_dir: String,
    pub(crate) threads_dir: String,
    pub(crate) fixture_mode: bool,
}

#[derive(Clone, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) struct LegacyImportPreview {
    pub(crate) active_recordings: usize,
    pub(crate) archived_recordings: usize,
    pub(crate) duplicates: usize,
    pub(crate) conflicts: usize,
    pub(crate) bytes_to_copy: u64,
    pub(crate) source_path: String,
}

#[derive(serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) struct LegacyImportResult {
    pub(crate) imported: usize,
    pub(crate) active_recordings: usize,
    pub(crate) archived_recordings: usize,
    pub(crate) duplicates: usize,
    pub(crate) conflicts: usize,
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
pub(crate) struct FinalizationStatusPayload {
    pub(crate) thread_id: String,
    pub(crate) state: String,
    pub(crate) message: String,
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
}
