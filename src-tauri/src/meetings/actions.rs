use std::time::Duration;

use tauri::{AppHandle, Manager};

use super::{
    notifications::{KEEP_ACTION, SKIP_ACTION, START_ACTION, STOP_ACTION},
    scheduler::{meeting_is_current, notification_id},
    state::MeetingSchedulerState,
};
use crate::{
    app::{now_ms, AppPaths},
    platform::notifications::NotificationResponseAction,
    recording::{self, RecorderState},
    settings::SettingsState,
};

const END_REMINDER_DELAY: Duration = Duration::from_secs(10 * 60);

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
    let app_settings = settings_state.snapshot();
    let recorder = app.state::<RecorderState>().inner().clone();
    orchestrate_start(
        &scheduler,
        request_id,
        |meeting| meeting_is_current(&app_settings, meeting),
        |meeting| {
            recording::start_scheduled_recording(
                app.clone(),
                paths.inner().clone(),
                recorder.clone(),
                settings_state.inner().clone(),
                meeting.title.clone(),
            )
        },
        |meeting, _| {
            super::notifications::remove(&[request_id.to_string()]);
            if app_settings.meeting_end_reminders {
                if let Some(session_id) = recorder.active_session_id() {
                    scheduler.set_active(
                        meeting.clone(),
                        session_id,
                        notification_id("end", &meeting.id),
                    );
                }
            }
        },
    )
}

fn orchestrate_start<T>(
    scheduler: &MeetingSchedulerState,
    request_id: &str,
    is_current: impl FnOnce(&super::model::Meeting) -> bool,
    start: impl FnOnce(&super::model::Meeting) -> Result<T, String>,
    after_success: impl FnOnce(&super::model::Meeting, &T),
) -> Result<T, MeetingStartError> {
    let Some(meeting) = scheduler.take_start_prompt(request_id) else {
        return Err(MeetingStartError {
            title: "Meeting".to_string(),
            message: "This meeting prompt is no longer available".to_string(),
            retryable: false,
        });
    };
    if !is_current(&meeting) {
        return Err(MeetingStartError {
            title: meeting.title,
            message: "This meeting is no longer available to record".to_string(),
            retryable: false,
        });
    }
    let started = retryable_start(scheduler, request_id, &meeting, || start(&meeting))?;
    after_success(&meeting, &started);
    Ok(started)
}

fn retryable_start<T>(
    scheduler: &MeetingSchedulerState,
    request_id: &str,
    meeting: &super::model::Meeting,
    start: impl FnOnce() -> Result<T, String>,
) -> Result<T, MeetingStartError> {
    start().map_err(|message| {
        scheduler.restore_start_prompt(request_id.to_string(), meeting.clone());
        MeetingStartError {
            title: meeting.title.clone(),
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
    match recording::stop_recording(app.clone(), recorder) {
        Ok(_) => {
            if let Some(completed_request_id) = scheduler.clear_active() {
                super::notifications::remove(&[completed_request_id]);
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
    scheduler.defer_end_prompt(
        request_id,
        now.saturating_add(END_REMINDER_DELAY.as_millis() as u64),
    );
}

#[cfg(test)]
mod tests;
