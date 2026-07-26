use tauri::{AppHandle, Manager};

use super::{
    model::Meeting,
    notifications::{KEEP_ACTION, SKIP_ACTION, START_ACTION, STOP_ACTION},
    scheduler::{auto_record_eligible, meeting_is_current, notification_id},
    state::MeetingSchedulerState,
};
use crate::{
    app::{now_ms, AppPaths},
    platform::notifications::NotificationResponseAction,
    recording::{self, RecorderState, ScheduledMeeting},
    settings::SettingsState,
};

pub(crate) fn handle_notification_action(
    app: AppHandle,
    response: NotificationResponseAction,
    start_prompt: impl FnOnce(&AppHandle, &str) + Send + 'static,
    sync_prompt: impl FnOnce(&AppHandle) + Send + 'static,
) {
    tauri::async_runtime::spawn_blocking(move || {
        if response.action_id == START_ACTION {
            start_prompt(&app, &response.request_id);
            return;
        }
        match response.action_id.as_str() {
            SKIP_ACTION => skip_meeting(&app, &response.request_id),
            STOP_ACTION => stop_meeting_recording(&app, &response.request_id),
            KEEP_ACTION => keep_recording(&app, &response.request_id),
            _ => {}
        }
        sync_prompt(&app);
    });
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct MeetingStartError {
    pub(crate) title: String,
    pub(crate) message: String,
    pub(crate) retryable: bool,
}

pub(crate) fn start_meeting_recording(
    app: &AppHandle,
    request_id: &str,
) -> Result<recording::StartedRecording, MeetingStartError> {
    let scheduler = app.state::<MeetingSchedulerState>().inner().clone();
    let paths = app.state::<AppPaths>();
    let settings_state = app.state::<SettingsState>();
    let recorder = app.state::<RecorderState>().inner().clone();
    orchestrate_start(
        &scheduler,
        request_id,
        |prompt| {
            let settings = settings_state.snapshot();
            meeting_is_current(&settings, &prompt.meeting)
                && (!prompt.auto_start || auto_record_eligible(&settings, &prompt.meeting))
        },
        |prompt| {
            let start = || {
                recording::start_scheduled_recording(
                    app.clone(),
                    paths.inner().clone(),
                    recorder.clone(),
                    settings_state.inner().clone(),
                    scheduled_meeting(&prompt.meeting),
                )
            };
            if prompt.auto_start {
                scheduler.with_auto_start_consent(prompt.auto_start_revision, start)
            } else {
                start()
            }
        },
        |prompt, _| {
            let app_settings = settings_state.snapshot();
            super::notifications::remove(&[request_id.to_string()]);
            let auto_stop = prompt.auto_start;
            if app_settings.meeting_end_reminders || auto_stop {
                if let Some(session_id) = recorder.active_session_id() {
                    scheduler.set_active(
                        prompt.meeting.clone(),
                        session_id,
                        notification_id("end", &prompt.meeting.id),
                        auto_stop,
                    );
                }
            }
        },
    )
}

/// Hands the recording context the calendar facts a thread should remember it
/// was created from.
fn scheduled_meeting(meeting: &Meeting) -> ScheduledMeeting {
    ScheduledMeeting {
        title: meeting.title.clone(),
        event_id: meeting.id.clone(),
        calendar_id: meeting.calendar_id.clone(),
        attendees: meeting.attendees.clone(),
        start_at_ms: meeting.start_at_ms,
        end_at_ms: meeting.end_at_ms,
    }
}

fn orchestrate_start<T>(
    scheduler: &MeetingSchedulerState,
    request_id: &str,
    is_current: impl FnOnce(&super::model::MeetingPrompt) -> bool,
    start: impl FnOnce(&super::model::MeetingPrompt) -> Result<T, String>,
    after_success: impl FnOnce(&super::model::MeetingPrompt, &T),
) -> Result<T, MeetingStartError> {
    let Some(prompt) = scheduler.take_start_prompt(request_id) else {
        return Err(MeetingStartError {
            title: "Meeting".to_string(),
            message: "This meeting prompt is no longer available".to_string(),
            retryable: false,
        });
    };
    if !is_current(&prompt) {
        return Err(MeetingStartError {
            title: prompt.meeting.title,
            message: "This meeting is no longer available to record".to_string(),
            retryable: false,
        });
    }
    let started = retryable_start(scheduler, &prompt, || start(&prompt))?;
    after_success(&prompt, &started);
    Ok(started)
}

fn retryable_start<T>(
    scheduler: &MeetingSchedulerState,
    prompt: &super::model::MeetingPrompt,
    start: impl FnOnce() -> Result<T, String>,
) -> Result<T, MeetingStartError> {
    start().map_err(|message| {
        scheduler.restore_start_prompt(prompt.clone());
        MeetingStartError {
            title: prompt.meeting.title.clone(),
            message,
            retryable: true,
        }
    })
}

fn skip_meeting(app: &AppHandle, request_id: &str) {
    let scheduler = app.state::<MeetingSchedulerState>().inner().clone();
    let _ = scheduler.take_start_prompt(request_id);
}

fn stop_meeting_recording(app: &AppHandle, request_id: &str) {
    let scheduler = app.state::<MeetingSchedulerState>().inner().clone();
    let recorder = app.state::<RecorderState>().inner().clone();
    let Some(active_session_id) = recorder.active_session_id() else {
        if let Some(stale_request_id) = scheduler.clear_active() {
            super::notifications::remove(&[stale_request_id]);
        }
        return;
    };
    if !scheduler.matches_active_end_prompt(request_id, active_session_id) {
        return;
    }
    match recording::stop_recording_session(app.clone(), recorder, active_session_id) {
        Ok(Some(_)) => {
            if let Some(completed_request_id) = scheduler.clear_active() {
                super::notifications::remove(&[completed_request_id]);
            }
        }
        Ok(None) => {
            if let Some(stale_request_id) = scheduler.clear_active() {
                super::notifications::remove(&[stale_request_id]);
            }
        }
        Err(err) => eprintln!("scheduled recording stop failed: {err}"),
    }
}

fn keep_recording(app: &AppHandle, request_id: &str) {
    let Ok(now) = now_ms() else {
        return;
    };
    let scheduler = app.state::<MeetingSchedulerState>().inner().clone();
    scheduler.defer_end_prompt_for_later(request_id, now);
}

#[cfg(test)]
mod tests;
