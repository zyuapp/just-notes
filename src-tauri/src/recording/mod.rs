mod audio_sink;
mod meter;
mod selection;
mod state;
mod stop;
mod workflow;

pub(crate) use state::RecorderState;
pub(crate) use stop::stop_recording;
pub(crate) use workflow::start_recording;

#[cfg(any(debug_assertions, feature = "qa-fixtures"))]
pub(crate) use workflow::start_fixture_recording;
