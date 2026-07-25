mod artifacts;
mod audio;
mod download;
mod faint_fillers;
mod finalize_audio;
mod live;
mod live_worker;
pub(crate) mod models;
mod polish;
#[cfg(test)]
mod quality;
mod runtime;
mod source_bleed;
mod status;
mod text;

pub(crate) use artifacts::{parakeet_artifact, ModelArtifact};
pub(crate) use audio::{resample_to_rate, rms, samples_for_ms, samples_to_ms, wav_duration_ms};
pub(crate) use download::ModelDownloadState;
pub(crate) use faint_fillers::{suppress_faint_mic_fillers, suppress_unconfirmed_mic_fillers};
pub(crate) use finalize_audio::FinalizationAudioArtifacts;
pub(crate) use live::{
    transcribe_live_utterance, ChannelRole, LiveSegmenter, SegmenterConfig, Utterance,
};
pub(crate) use live_worker::{
    spawn_live_transcription, LiveTranscription, LiveTranscriptionConfig,
};
pub(crate) use polish::polish_thread_transcript;

pub(crate) use models::{
    delete_parakeet_model, finalization_transcription_catalog,
    finalization_transcription_selection, parakeet_model_files, TranscriptionModelDownloadState,
    TranscriptionModelSelection, TranscriptionStatusPayload,
};
pub(crate) use runtime::{load_transcriber, Transcriber};
pub(crate) use source_bleed::suppress_system_dominated_mic_segments;
pub(crate) use status::{transcription_status, transcription_status_with_downloads};
pub(crate) use text::{
    clean_transcript_text, is_ignored_transcript_text, suppress_cross_channel_bleed,
};
