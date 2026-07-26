use tauri::AppHandle;

use super::{
    model::ScheduledMeeting,
    state::RecorderState,
    workflow::{start_recording_with_mode, RecordingRequest},
};
use crate::{app::AppPaths, capture::RecordingInputMode, settings::SettingsState};

pub(crate) fn start_recording(
    app: AppHandle,
    paths: AppPaths,
    recorder: RecorderState,
    settings: SettingsState,
    thread_id: Option<String>,
) -> Result<super::StartedRecording, String> {
    start_recording_with_mode(RecordingRequest {
        app,
        base_paths: paths,
        recorder,
        settings_state: settings,
        requested_thread_id: thread_id,
        scheduled_meeting: None,
        input_mode: RecordingInputMode::Devices,
    })
}

#[cfg(any(debug_assertions, feature = "qa-fixtures"))]
pub(crate) fn start_fixture_recording(
    app: AppHandle,
    paths: AppPaths,
    recorder: RecorderState,
    settings: SettingsState,
    thread_id: Option<String>,
) -> Result<super::StartedRecording, String> {
    let fixture_dir = paths.data_dir.join("fixtures");
    start_recording_with_mode(RecordingRequest {
        app,
        base_paths: paths,
        recorder,
        settings_state: settings,
        requested_thread_id: thread_id,
        scheduled_meeting: None,
        input_mode: RecordingInputMode::Fixture {
            mic_path: fixture_dir.join("qa-mic.wav"),
            system_path: fixture_dir.join("qa-system.wav"),
        },
    })
}

pub(crate) fn start_scheduled_recording(
    app: AppHandle,
    paths: AppPaths,
    recorder: RecorderState,
    settings: SettingsState,
    meeting: ScheduledMeeting,
) -> Result<super::StartedRecording, String> {
    start_recording_with_mode(RecordingRequest {
        app,
        base_paths: paths,
        recorder,
        settings_state: settings,
        requested_thread_id: None,
        scheduled_meeting: Some(meeting),
        input_mode: RecordingInputMode::Devices,
    })
}
