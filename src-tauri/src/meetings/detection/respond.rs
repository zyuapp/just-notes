use tauri::{AppHandle, Manager};

use super::{
    calendar_match::{matching_meeting, EARLY_JOIN_MS},
    call_app::CallApp,
    state::{CallDetectionState, EndedPrompt},
    tracker::CallEvent,
};
use crate::{
    app::{now_ms, AppPaths},
    meetings::{
        model::Meeting,
        notifications,
        scheduler::{auto_record_eligible, notification_id},
        state::MeetingSchedulerState,
    },
    platform::{calendar, notifications as platform_notifications},
    recording::{self, RecorderState},
    settings::{save_settings, AppSettings, SettingsState},
};

/// Withdraws offers once a recording runs and call-ended prompts once theirs stops.
pub(super) fn reconcile(app: &AppHandle, session_id: Option<u64>) {
    let state = app.state::<CallDetectionState>();
    if session_id.is_some() {
        notifications::remove(&state.clear_offers());
    }
    if let Some(request_id) = state.clear_stale_ended_prompt(session_id) {
        notifications::remove(&[request_id]);
    }
}

pub(super) fn handle(
    app: &AppHandle,
    settings: &AppSettings,
    session_id: Option<u64>,
    event: CallEvent,
    start_recording: &impl Fn(&AppHandle, &str),
) {
    match event {
        CallEvent::Detected(call) if session_id.is_none() => {
            offer(app, settings, call, start_recording);
        }
        CallEvent::Detected(_) => {}
        CallEvent::Ended {
            app: call,
            recorded,
        } => {
            let state = app.state::<CallDetectionState>();
            notifications::remove(&state.remove_offers_for(&call));
            if let Some(session_id) = session_id.filter(|_| recorded) {
                let request_id =
                    notification_id("call-ended", &format!("{}:{session_id}", call.name));
                let prompt = EndedPrompt {
                    request_id: request_id.clone(),
                    session_id,
                };
                if let Some(previous) = state.set_ended_prompt(prompt) {
                    notifications::remove(&[previous]);
                }
                notifications::show_call_ended(&request_id, &call.thread_title());
            }
        }
    }
}

/// Offers to record the call, or starts it when its calendar meeting is set
/// to record automatically. Skipped meetings and armed automatic starts are
/// left to the calendar flow.
fn offer(
    app: &AppHandle,
    settings: &AppSettings,
    call: CallApp,
    start_recording: &impl Fn(&AppHandle, &str),
) {
    let Ok(now) = now_ms() else {
        return;
    };
    let meeting = calendar_meeting(settings, &call, now);
    if let Some(meeting) = &meeting {
        let scheduler = app.state::<MeetingSchedulerState>();
        if scheduler.calendar_owns_call(meeting, now) {
            return;
        }
    }
    let automatic = meeting
        .as_ref()
        .is_some_and(|meeting| auto_record_eligible(settings, meeting))
        && platform_notifications::automation_authorized().unwrap_or(false);
    let state = app.state::<CallDetectionState>().inner().clone();
    let Some(offer) = state.add_offer(call, meeting, automatic) else {
        return;
    };
    if automatic {
        start_recording(app, &offer.request_id);
        return;
    }
    let body = match &offer.meeting {
        Some(meeting) => format!("Record \u{201c}{}\u{201d}?", meeting.title),
        None => "Record this call?".to_string(),
    };
    let request_id = offer.request_id.clone();
    notifications::show_call_offer(
        &offer.request_id,
        &call.detected_title(),
        &body,
        move |error| {
            eprintln!("call offer {request_id} could not be delivered: {error}");
            let _ = state.take_offer(&request_id);
        },
    );
}

fn calendar_meeting(settings: &AppSettings, call: &CallApp, now: u64) -> Option<Meeting> {
    if settings.meeting_calendar_ids.is_empty() || calendar::authorization_status() != "authorized"
    {
        return None;
    }
    let events = calendar::upcoming_events(
        &settings.meeting_calendar_ids,
        now.saturating_sub(EARLY_JOIN_MS),
        now.saturating_add(EARLY_JOIN_MS),
    )
    .inspect_err(|error| eprintln!("call detection could not read the calendar: {error}"))
    .ok()?;
    matching_meeting(call, events, now)
}

pub(in crate::meetings) fn dismiss_offer(app: &AppHandle, request_id: &str) {
    let _ = app.state::<CallDetectionState>().take_offer(request_id);
}

/// Stops offering the offer's app. Returns whether settings changed.
pub(in crate::meetings) fn ignore_app(app: &AppHandle, request_id: &str) -> bool {
    let Some(offer) = app.state::<CallDetectionState>().take_offer(request_id) else {
        return false;
    };
    let settings_state = app.state::<SettingsState>();
    let mut settings = settings_state.snapshot();
    if settings
        .call_detection_ignored_apps
        .iter()
        .any(|name| name == offer.app.name)
    {
        return false;
    }
    settings
        .call_detection_ignored_apps
        .push(offer.app.name.to_string());
    if let Err(error) = save_settings(&app.state::<AppPaths>().data_dir, &settings) {
        eprintln!("could not stop offering {}: {error}", offer.app.name);
        return false;
    }
    settings_state.replace(settings);
    app.state::<CallDetectionState>().wake();
    true
}

pub(in crate::meetings) fn stop_after_call(app: &AppHandle, request_id: &str) {
    let Some(prompt) = app
        .state::<CallDetectionState>()
        .take_ended_prompt(request_id)
    else {
        return;
    };
    let recorder = app.state::<RecorderState>().inner().clone();
    if recorder.active_session_id() != Some(prompt.session_id) {
        return;
    }
    match recording::stop_recording_session(app.clone(), recorder, prompt.session_id) {
        Ok(_) => {
            let scheduler = app.state::<MeetingSchedulerState>();
            if let Some(end_request_id) = scheduler.clear_active() {
                notifications::remove(&[end_request_id]);
            }
        }
        Err(error) => eprintln!("recording stop after the call ended failed: {error}"),
    }
}

pub(in crate::meetings) fn keep_recording(app: &AppHandle, request_id: &str) {
    let _ = app
        .state::<CallDetectionState>()
        .take_ended_prompt(request_id);
}
