mod audio;
mod finalize;
mod finalize_audio;
pub(crate) mod models;
mod runtime;
mod source_bleed;
mod status;
mod text;

pub(crate) use audio::{resample_to_rate, rms, samples_to_ms};
pub(crate) use finalize::{
    emit_finalization_failure, spawn_finalization, FinalizationConfig, FinalizationStart,
    FinalizeState,
};
pub(crate) use finalize_audio::FinalizationAudioArtifacts;
pub(crate) use models::{
    finalization_transcription_catalog, finalization_transcription_selection,
    TranscriptionModelSelection, TranscriptionProvider, TranscriptionStatusPayload,
};
pub(crate) use runtime::{load_transcriber, Transcriber};
pub(crate) use source_bleed::suppress_system_dominated_mic_segments;
pub(crate) use status::transcription_status;
pub(crate) use text::{
    clean_transcript_text, is_ignored_transcript_text, suppress_cross_channel_bleed,
};
