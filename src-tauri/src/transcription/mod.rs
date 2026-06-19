mod artifacts;
mod audio;
mod download;
mod finalize;
mod finalize_audio;
pub(crate) mod models;
mod runtime;
mod source_bleed;
mod status;
mod text;

pub(crate) use artifacts::{artifact_for_provider, ModelArtifact};
pub(crate) use audio::{resample_to_rate, rms, samples_to_ms};
pub(crate) use download::ModelDownloadState;
pub(crate) use finalize::{
    emit_finalization_failure, spawn_finalization, FinalizationConfig, FinalizationStart,
    FinalizeState,
};
pub(crate) use finalize_audio::FinalizationAudioArtifacts;
pub(crate) use models::{
    default_provider_for_installed, finalization_transcription_catalog,
    finalization_transcription_selection, parakeet_model_files, TranscriptionModelDownloadState,
    TranscriptionModelSelection, TranscriptionProvider, TranscriptionStatusPayload,
};
pub(crate) use runtime::{load_transcriber, Transcriber};
pub(crate) use source_bleed::suppress_system_dominated_mic_segments;
pub(crate) use status::{transcription_status, transcription_status_with_downloads};
pub(crate) use text::{
    clean_transcript_text, is_ignored_transcript_text, suppress_cross_channel_bleed,
};
