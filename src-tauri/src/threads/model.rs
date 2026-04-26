#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ThreadMetadata {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) created_at_ms: u64,
    pub(crate) updated_at_ms: u64,
    pub(crate) status: ThreadStatus,
}

#[derive(serde::Serialize, serde::Deserialize, ts_rs::TS, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) enum ThreadStatus {
    Idle,
    Recording,
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
    pub(crate) transcript_markdown_path: String,
}
