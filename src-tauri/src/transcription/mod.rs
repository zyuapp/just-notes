mod audio;
mod live;
pub(crate) mod models;
mod runtime;
mod status;
mod text;

pub(crate) use audio::{first_audible_ms, ms_to_samples, resample_to_rate, rms, samples_to_ms};
pub(crate) use live::{spawn_live_transcription_thread, LiveTranscriptionThreadConfig};
pub(crate) use models::{transcription_paths, TranscriptionPaths, TranscriptionStatusPayload};
pub(crate) use runtime::WhisperRuntime;
pub(crate) use status::transcription_status;
pub(crate) use text::{
    clean_transcript_text, common_transcript_prefix, completed_transcript_text,
    estimate_text_end_ms, is_ignored_transcript_text, unique_transcript_text,
    LIVE_DUPLICATE_RECENT_SEGMENTS,
};
