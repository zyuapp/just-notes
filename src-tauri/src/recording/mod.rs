mod audio_sink;
mod entrypoints;
mod meter;
mod model;
mod reprocess;
mod selection;
mod state;
mod stop;
mod workflow;

pub(crate) use entrypoints::{start_recording, start_scheduled_recording};
pub(crate) use model::StartedRecording;
pub(crate) use reprocess::{reprocess_thread, ReprocessRequest};
pub(crate) use state::RecorderState;
pub(crate) use stop::stop_recording;

#[cfg(any(debug_assertions, feature = "qa-fixtures"))]
pub(crate) use entrypoints::start_fixture_recording;
