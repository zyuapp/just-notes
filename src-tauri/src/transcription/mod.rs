mod audio;
mod finalize;
mod live;
pub(crate) mod models;
mod runtime;
mod status;
mod text;

pub(crate) use audio::{audible_sample_span, ms_to_samples, resample_to_rate, rms, samples_to_ms};
pub(crate) use finalize::{spawn_finalization, FinalizationConfig, FinalizeState};
pub(crate) use live::{spawn_live_transcription_thread, LiveTranscriptionThreadConfig};
pub(crate) use models::{
    finalization_transcription_paths, live_transcription_paths, TranscriptionPaths,
    TranscriptionStatusPayload,
};
pub(crate) use runtime::WhisperRuntime;
pub(crate) use status::transcription_status;
pub(crate) use text::{
    clean_transcript_text, common_transcript_prefix, completed_transcript_text,
    estimate_text_end_ms, is_duplicate_of_recent, is_ignored_transcript_text,
    suppress_cross_channel_bleed, unique_transcript_text, LIVE_DUPLICATE_RECENT_SEGMENTS,
};
