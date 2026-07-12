use tauri::{AppHandle, State};

use crate::{
    commands::recording::to_payload as to_recording_payload,
    ipc::{MeetingAccessPayload, MeetingCalendarPayload, MeetingPromptPayload, RecordingPayload},
    meeting_surfaces,
    meetings::{self, MeetingSchedulerState},
};

#[tauri::command]
pub(crate) async fn get_meeting_access_status() -> Result<MeetingAccessPayload, String> {
    tauri::async_runtime::spawn_blocking(meetings::access_status)
        .await
        .map_err(|err| format!("Calendar status task failed: {err}"))
        .map(to_payload)
}

#[tauri::command]
pub(crate) fn get_meeting_prompt(
    scheduler: State<'_, MeetingSchedulerState>,
) -> Option<MeetingPromptPayload> {
    scheduler
        .current_start_prompt()
        .map(meeting_surfaces::to_payload)
}

#[tauri::command]
pub(crate) async fn start_meeting_recording(
    app: AppHandle,
    request_id: String,
) -> Result<RecordingPayload, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let result = meetings::start_meeting_recording(&app, &request_id)
            .map(to_recording_payload)
            .map_err(|err| err.message);
        meeting_surfaces::sync_current(&app);
        result
    })
    .await
    .map_err(|err| format!("Meeting recording task failed: {err}"))?
}

#[tauri::command]
pub(crate) fn dismiss_meeting_prompt(
    app: AppHandle,
    scheduler: State<'_, MeetingSchedulerState>,
    request_id: String,
) -> Option<MeetingPromptPayload> {
    meetings::dismiss_prompt(&scheduler, &request_id);
    meeting_surfaces::sync_current(&app)
}

#[tauri::command]
pub(crate) async fn request_meeting_access(app: AppHandle) -> Result<MeetingAccessPayload, String> {
    let request_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || meetings::request_access(&request_app))
        .await
        .map_err(|err| format!("Calendar permission task failed: {err}"))?
        .map(to_payload)
}

fn to_payload(access: meetings::MeetingAccess) -> MeetingAccessPayload {
    MeetingAccessPayload {
        calendar_authorization: access.calendar_authorization,
        notification_authorization: access.notification_authorization,
        calendars: access
            .calendars
            .into_iter()
            .map(|calendar| MeetingCalendarPayload {
                id: calendar.id,
                title: calendar.title,
            })
            .collect(),
    }
}
