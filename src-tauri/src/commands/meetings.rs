use tauri::AppHandle;

use crate::{
    ipc::{MeetingAccessPayload, MeetingCalendarPayload},
    meetings,
};

#[tauri::command]
pub(crate) async fn get_meeting_access_status() -> Result<MeetingAccessPayload, String> {
    tauri::async_runtime::spawn_blocking(meetings::access_status)
        .await
        .map_err(|err| format!("Calendar status task failed: {err}"))
        .map(to_payload)
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
