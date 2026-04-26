mod meter;
mod state;
mod workflow;

pub(crate) use state::RecorderState;
pub(crate) use workflow::{start_recording, stop_recording};

#[cfg(any(debug_assertions, feature = "qa-fixtures"))]
pub(crate) use workflow::start_fixture_recording;
