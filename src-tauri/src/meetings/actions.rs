use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use super::{
    notifications::{KEEP_ACTION, SKIP_ACTION, START_ACTION, STOP_ACTION},
    scheduler::{meeting_is_current, notification_id},
    state::MeetingSchedulerState,
};
use crate::{
    app::{now_ms, AppPaths},
    platform::notifications::NotificationResponseAction,
    recording::{self, RecorderState},
    settings::{self, SettingsState},
};

const END_REMINDER_DELAY: Duration = Duration::from_secs(10 * 60);

pub(crate) fn handle_notification_action(app: AppHandle, response: NotificationResponseAction) {
    tauri::async_runtime::spawn_blocking(move || match response.action_id.as_str() {
        START_ACTION => start_meeting_recording(&app, &response.request_id),
        SKIP_ACTION => skip_meeting(&app, &response.request_id),
        STOP_ACTION => stop_meeting_recording(&app, &response.request_id),
        KEEP_ACTION => keep_recording(&app, &response.request_id),
        _ => {}
    });
}

fn start_meeting_recording(app: &AppHandle, request_id: &str) {
    let scheduler = app.state::<MeetingSchedulerState>().inner().clone();
    let Some(meeting) = scheduler.take_start_prompt(request_id) else {
        return;
    };
    let paths = app.state::<AppPaths>();
    let settings_state = app.state::<SettingsState>();
    let app_settings = settings_state.snapshot();
    if !meeting_is_current(&app_settings, &meeting) {
        return;
    }
    let effective_paths = settings::effective_paths(&paths, &app_settings);
    let recorder = app.state::<RecorderState>().inner().clone();
    match recording::start_scheduled_recording(
        app.clone(),
        effective_paths,
        recorder.clone(),
        app_settings.clone(),
        meeting.title.clone(),
    ) {
        Ok(payload) => {
            if app_settings.meeting_end_reminders {
                let Some(session_id) = recorder.active_session_id() else {
                    return;
                };
                scheduler.set_active(
                    meeting.clone(),
                    session_id,
                    notification_id("end", &meeting.id),
                );
            }
            let _ = app.emit("recording-started", &payload);
        }
        Err(err) => {
            scheduler.restore_start_prompt(request_id.to_string(), meeting.clone());
            super::notifications::show_start_failure(request_id, &meeting.title, &err);
        }
    }
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
