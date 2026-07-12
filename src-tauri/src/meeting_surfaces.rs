use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager};

use crate::{
    commands::recording::to_payload as to_recording_payload,
    ipc::MeetingPromptPayload,
    meetings::{self, MeetingPrompt, MeetingSchedulerState},
    tray::{self, TrayMeetingPrompt},
};

static SURFACE_SYNC: Mutex<()> = Mutex::new(());

pub(crate) fn start_native_meeting(app: &AppHandle, request_id: String) {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        start_native_meeting_now(&app, &request_id);
    });
}

pub(crate) fn start_native_meeting_now(app: &AppHandle, request_id: &str) {
    match meetings::start_meeting_recording(app, request_id) {
        Ok(started) => {
            let payload = to_recording_payload(started);
            let _ = app.emit("recording-started", &payload);
        }
        Err(err) => meetings::show_start_failure(request_id, &err.title, &err.message),
    }
    sync_current(app);
}

pub(crate) fn to_payload(prompt: MeetingPrompt) -> MeetingPromptPayload {
    MeetingPromptPayload {
        request_id: prompt.request_id,
        title: prompt.meeting.title,
        start_at_ms: prompt.meeting.start_at_ms,
        end_at_ms: prompt.meeting.end_at_ms,
    }
}

fn publish(app: &AppHandle, prompt: Option<MeetingPrompt>) -> Option<MeetingPromptPayload> {
    let payload = prompt.clone().map(to_payload);
    let _ = app.emit("meeting-prompt-updated", payload.clone());
    tray::set_meeting_prompt(
        app,
        prompt.map(|prompt| TrayMeetingPrompt {
            request_id: prompt.request_id,
            title: prompt.meeting.title,
        }),
    );
    payload
}

pub(crate) fn sync_current(app: &AppHandle) -> Option<MeetingPromptPayload> {
    let _sync = SURFACE_SYNC
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let prompt = app.state::<MeetingSchedulerState>().current_start_prompt();
    publish(app, prompt)
}
