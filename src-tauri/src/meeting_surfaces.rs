use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager};

use crate::{
    ipc::{MeetingPromptPayload, RecordingPayload},
    meetings::{self, MeetingPrompt, MeetingSchedulerState},
    recording_payload,
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
    if let Ok(payload) = start(app, request_id) {
        let _ = app.emit("recording-started", &payload);
    }
}

pub(crate) fn start(app: &AppHandle, request_id: &str) -> Result<RecordingPayload, String> {
    let result = meetings::start_meeting_recording(app, request_id);
    if let Err(err) = &result {
        if err.retryable {
            meetings::show_start_failure(request_id, &err.title, &err.message);
        } else {
            meetings::remove_notifications(&[request_id.to_string()]);
        }
    }
    sync_current(app);
    result
        .map(recording_payload::from_started)
        .map_err(|err| err.message)
}

pub(crate) fn dismiss(
    app: &AppHandle,
    scheduler: &MeetingSchedulerState,
    request_id: &str,
) -> Option<MeetingPromptPayload> {
    meetings::dismiss_prompt(scheduler, request_id);
    sync_current(app)
}

pub(crate) fn to_payload(prompt: MeetingPrompt) -> MeetingPromptPayload {
    MeetingPromptPayload {
        request_id: prompt.request_id,
        title: prompt.meeting.title,
        start_at_ms: prompt.meeting.start_at_ms,
        end_at_ms: prompt.meeting.end_at_ms,
        auto_start: prompt.auto_start,
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
