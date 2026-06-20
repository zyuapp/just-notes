pub(crate) mod artifacts;
pub(crate) mod commit;
pub(crate) mod edits;
pub(crate) mod model;
pub(crate) mod repository;
pub(crate) mod transcript_store;

pub(crate) use artifacts::RecordingAudioPaths;
pub(crate) use model::{
    ThreadDetail, ThreadMetadata, ThreadStatus, ThreadSummary, TranscriptSegment,
};
