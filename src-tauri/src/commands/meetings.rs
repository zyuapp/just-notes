use tauri::{AppHandle, State};

use crate::{
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
    tauri::async_runtime::spawn_blocking(move || meeting_surfaces::start(&app, &request_id))
        .await
        .map_err(|err| format!("Meeting recording task failed: {err}"))?
}

#[tauri::command]
pub(crate) fn dismiss_meeting_prompt(
    app: AppHandle,
    scheduler: State<'_, MeetingSchedulerState>,
    request_id: String,
) -> Option<MeetingPromptPayload> {
    meeting_surfaces::dismiss(&app, &scheduler, &request_id)
}

#[tauri::command]
pub(crate) async fn request_meeting_calendar_access(
    app: AppHandle,
) -> Result<MeetingAccessPayload, String> {
    let request_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || meetings::request_calendar_access(&request_app))
        .await
        .map_err(|err| format!("Calendar permission task failed: {err}"))?
        .map(to_payload)
}

#[tauri::command]
pub(crate) async fn request_meeting_notification_access(
    app: AppHandle,
) -> Result<MeetingAccessPayload, String> {
    let request_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        meetings::request_notification_access(&request_app)
    })
    .await
    .map_err(|err| format!("Notification permission task failed: {err}"))?
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
                account: calendar.account,
                color: calendar.color,
            })
            .collect(),
    }
}
