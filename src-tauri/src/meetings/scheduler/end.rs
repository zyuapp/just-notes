use tauri::{AppHandle, Manager};

use crate::recording::{self, RecorderState};

use super::super::{notifications, state::MeetingSchedulerState};

pub(super) fn check(
    app: &AppHandle,
    scheduler: &MeetingSchedulerState,
    active_session_id: Option<u64>,
    now: u64,
    notifications_authorized: bool,
) {
    if let Some(prompt) =
        active_session_id.and_then(|session_id| scheduler.due_end_prompt(now, session_id))
    {
        let retry_scheduler = scheduler.clone();
        let retry_request_id = prompt.request_id.clone();
        notifications::show_end_prompt(
            &prompt.request_id,
            &prompt.meeting.title,
            prompt.auto_stop,
            move |error| {
                eprintln!("end reminder {retry_request_id} could not be delivered: {error}");
                retry_scheduler.retry_end_prompt(&retry_request_id);
            },
        );
        return;
    }
    if let Some(session_id) = active_session_id {
        if let Some(request_id) = scheduler.claim_due_auto_stop(now, session_id) {
            if notifications_authorized {
                stop_scheduled_recording(app, scheduler, &request_id, session_id);
            } else {
                scheduler.retry_auto_stop(&request_id);
                scheduler.defer_end_prompt_for_later(&request_id, now);
            }
        }
    }
}

fn stop_scheduled_recording(
    app: &AppHandle,
    scheduler: &MeetingSchedulerState,
    request_id: &str,
    expected_session_id: u64,
) {
    let recorder = app.state::<RecorderState>().inner().clone();
    match recording::stop_recording_session(app.clone(), recorder, expected_session_id) {
        Ok(Some(_)) => {
            if let Some(completed_request_id) = scheduler.clear_active() {
                notifications::remove(&[completed_request_id]);
            }
        }
        Ok(None) => {
            if let Some(stale_request_id) = scheduler.clear_active() {
                notifications::remove(&[stale_request_id]);
            }
        }
        Err(error) => {
            eprintln!("automatic meeting stop {request_id} failed: {error}");
            scheduler.retry_auto_stop(request_id);
        }
    }
}
