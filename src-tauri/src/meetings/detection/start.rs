use tauri::{AppHandle, Manager};

use super::state::{CallDetectionState, CallOffer};
use crate::{
    app::AppPaths,
    meetings::{
        actions::{scheduled_meeting, MeetingStartError},
        notifications,
        scheduler::notification_id,
        state::MeetingSchedulerState,
    },
    recording::{self, RecorderState, StartedRecording},
    settings::{AppSettings, SettingsState},
};

/// Starts recording a detected call, or returns `None` when `request_id` is
/// not a call offer. A call during a calendar meeting is recorded as that
/// meeting, so its thread carries the event's title and attendees.
pub(in crate::meetings) fn start_offer(
    app: &AppHandle,
    request_id: &str,
) -> Option<Result<StartedRecording, MeetingStartError>> {
    let state = app.state::<CallDetectionState>().inner().clone();
    let offer = state.take_offer(request_id)?;
    let settings_state = app.state::<SettingsState>().inner().clone();
    let settings = settings_state.snapshot();
    if !settings.call_detection_enabled {
        return Some(Err(MeetingStartError {
            title: offer.title(),
            message: "Call detection is turned off".to_string(),
            retryable: false,
        }));
    }
    let paths = app.state::<AppPaths>().inner().clone();
    let recorder = app.state::<RecorderState>().inner().clone();
    let result = match &offer.meeting {
        Some(meeting) => recording::start_scheduled_recording(
            app.clone(),
            paths,
            recorder.clone(),
            settings_state,
            scheduled_meeting(meeting),
        ),
        None => recording::start_titled_recording(
            app.clone(),
            paths,
            recorder.clone(),
            settings_state,
            offer.app.thread_title(),
        ),
    };
    Some(match result {
        Ok(started) => {
            after_start(app, &settings, &offer, &recorder);
            Ok(started)
        }
        Err(message) => {
            let title = offer.title();
            state.restore_offer(CallOffer {
                automatic: false,
                ..offer
            });
            Err(MeetingStartError {
                title,
                message,
                retryable: true,
            })
        }
    })
}

/// Hands a calendar-matched recording to the meeting's end reminders and
/// automatic stop, exactly as if it had started from its calendar prompt.
fn after_start(
    app: &AppHandle,
    settings: &AppSettings,
    offer: &CallOffer,
    recorder: &RecorderState,
) {
    notifications::remove(std::slice::from_ref(&offer.request_id));
    if let Some(meeting) = &offer.meeting {
        let scheduler = app.state::<MeetingSchedulerState>();
        notifications::remove(&scheduler.claim_meeting(meeting));
        let track_end = settings.meeting_end_reminders || offer.automatic;
        if settings.meeting_reminders_enabled && track_end {
            if let Some(session_id) = recorder.active_session_id() {
                scheduler.set_active(
                    meeting.clone(),
                    session_id,
                    notification_id("end", &meeting.id),
                    offer.automatic,
                );
            }
        }
    }
    if offer.automatic {
        notifications::show_call_recording_started(
            &notification_id("call-started", &offer.request_id),
            &format!("Recording \u{201c}{}\u{201d}", offer.title()),
            &format!(
                "{} joined the meeting, so recording started automatically.",
                offer.app.name
            ),
        );
    }
}
