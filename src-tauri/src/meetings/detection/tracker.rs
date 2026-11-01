use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use super::call_app::{CallApp, CallKind};

const APP_STABLE_FOR: Duration = Duration::from_secs(2);
const BROWSER_STABLE_FOR: Duration = Duration::from_secs(20);
const CALL_ENDED_AFTER: Duration = Duration::from_secs(3);
const FORGET_AFTER: Duration = Duration::from_secs(60);

#[derive(Debug, PartialEq, Eq)]
pub(super) enum CallEvent {
    /// The app held the microphone long enough to count as a call.
    Detected(CallApp),
    /// The app left the microphone. `recorded` is true when a recording was
    /// running while the call was on the microphone and is still running.
    Ended { app: CallApp, recorded: bool },
}

#[derive(Debug)]
struct TrackedCall {
    app: CallApp,
    present_since: Option<Instant>,
    absent_since: Option<Instant>,
    /// Offered, declined, or recorded: not offered again until forgotten.
    handled: bool,
    recorded: bool,
    ended_reported: bool,
}

/// Turns microphone snapshots into call starts and ends. A call stays handled
/// until its app has been off the microphone for a minute, so a brief release
/// or a stopped recording does not offer the same call again.
#[derive(Debug, Default)]
pub(super) struct CallTracker {
    calls: HashMap<&'static str, TrackedCall>,
}

impl CallTracker {
    pub(super) fn observe(
        &mut self,
        now: Instant,
        present: &[CallApp],
        recording: bool,
    ) -> Vec<CallEvent> {
        let mut events = Vec::new();
        for app in present {
            let call = self.calls.entry(app.name).or_insert(TrackedCall {
                app: *app,
                present_since: None,
                absent_since: None,
                handled: false,
                recorded: false,
                ended_reported: false,
            });
            if call.present_since.is_none() {
                call.present_since = Some(now);
            }
            call.absent_since = None;
            call.ended_reported = false;
            let stable = call
                .present_since
                .is_some_and(|since| now >= since + stable_for(app));
            if !stable {
                continue;
            }
            if recording {
                call.handled = true;
                call.recorded = true;
            } else if !call.handled {
                call.handled = true;
                events.push(CallEvent::Detected(*app));
            }
        }
        self.report_ended_calls(now, present, recording, &mut events);
        self.calls.retain(|_, call| {
            call.absent_since
                .is_none_or(|absent_since| now < absent_since + FORGET_AFTER)
        });
        events
    }

    fn report_ended_calls(
        &mut self,
        now: Instant,
        present: &[CallApp],
        recording: bool,
        events: &mut Vec<CallEvent>,
    ) {
        for call in self.calls.values_mut() {
            if !recording {
                call.recorded = false;
            }
            if present.iter().any(|app| app.name == call.app.name) {
                continue;
            }
            let absent_since = *call.absent_since.get_or_insert(now);
            call.present_since = None;
            if call.handled && !call.ended_reported && now >= absent_since + CALL_ENDED_AFTER {
                call.ended_reported = true;
                events.push(CallEvent::Ended {
                    app: call.app,
                    recorded: call.recorded,
                });
                call.recorded = false;
            }
        }
    }

    /// When the next observation could produce an event without the
    /// microphone changing.
    pub(super) fn next_deadline(&self, recording: bool) -> Option<Instant> {
        self.calls
            .values()
            .filter_map(|call| match (call.present_since, call.absent_since) {
                (Some(since), _) if !call.handled || (recording && !call.recorded) => {
                    Some(since + stable_for(&call.app))
                }
                (_, Some(since)) if call.handled && !call.ended_reported => {
                    Some(since + CALL_ENDED_AFTER)
                }
                (_, Some(since)) => Some(since + FORGET_AFTER),
                _ => None,
            })
            .min()
    }
}

fn stable_for(app: &CallApp) -> Duration {
    match app.kind {
        CallKind::App => APP_STABLE_FOR,
        CallKind::Browser => BROWSER_STABLE_FOR,
    }
}

#[cfg(test)]
mod tests;
