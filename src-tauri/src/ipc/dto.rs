use crate::{
    threads::ThreadDetail, threads::TranscriptSegment, transcription::TranscriptionStatusPayload,
};

#[derive(serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) struct AppInfo {
    pub(crate) data_dir: String,
    pub(crate) threads_dir: String,
    pub(crate) fixture_mode: bool,
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
pub(crate) struct LiveTranscriptSegmentPayload {
    pub(crate) thread_id: String,
    pub(crate) committed_until_ms: u64,
    pub(crate) segment: TranscriptSegment,
}

#[derive(serde::Serialize, ts_rs::TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) struct LiveTranscriptStatusPayload {
    pub(crate) thread_id: String,
    pub(crate) active: bool,
    pub(crate) message: String,
    pub(crate) chunk_ms: u64,
    pub(crate) overlap_ms: u64,
}
