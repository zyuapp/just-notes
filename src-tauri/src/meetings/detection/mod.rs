//! Call detection: offers to record when a calling app or browser holds the
//! microphone, naming the thread after the calendar meeting it belongs to.

mod calendar_match;
mod call_app;
mod respond;
mod start;
mod state;
mod tracker;
mod watcher;

pub(super) use respond::{dismiss_offer, ignore_app, keep_recording, stop_after_call};
pub(super) use start::start_offer;
pub(crate) use state::CallDetectionState;
pub(crate) use watcher::spawn;
