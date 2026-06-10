use std::collections::BTreeMap;

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
    #[serde(default)]
    pub(crate) speaker_labels: BTreeMap<String, String>,
}

#[derive(serde::Serialize, serde::Deserialize, ts_rs::TS, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) enum ThreadStatus {
    Idle,
    Recording,
    Transcribing,
}

impl ThreadStatus {
    pub(crate) fn is_recording(&self) -> bool {
        matches!(self, Self::Recording)
    }

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
    pub(crate) speaker_labels: BTreeMap<String, String>,
    pub(crate) transcript_markdown_path: String,
}
