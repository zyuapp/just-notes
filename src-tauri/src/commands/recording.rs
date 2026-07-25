use tauri::{AppHandle, State};

use crate::{
    app::AppPaths,
    ipc::RecordingPayload,
    recording::{self, RecorderState},
    recording_payload,
    settings::SettingsState,
    threads::ThreadDetail,
};

#[tauri::command]
pub(crate) async fn start_recording(
    app: AppHandle,
    paths: State<'_, AppPaths>,
    recorder: State<'_, RecorderState>,
    settings: State<'_, SettingsState>,
    thread_id: Option<String>,
) -> Result<RecordingPayload, String> {
    let paths = paths.inner().clone();
    let recorder = recorder.inner().clone();
    let settings = settings.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        recording::start_recording(app, paths, recorder, settings, thread_id)
    })
    .await
    .map_err(|err| format!("Audio startup task failed: {err}"))?
    .map(recording_payload::from_started)
}

#[cfg(any(debug_assertions, feature = "qa-fixtures"))]
#[tauri::command]
pub(crate) async fn start_fixture_recording(
    app: AppHandle,
    paths: State<'_, AppPaths>,
    recorder: State<'_, RecorderState>,
    settings: State<'_, SettingsState>,
    thread_id: Option<String>,
) -> Result<RecordingPayload, String> {
    let paths = paths.inner().clone();
    let recorder = recorder.inner().clone();
    let settings = settings.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        recording::start_fixture_recording(app, paths, recorder, settings, thread_id)
    })
    .await
    .map_err(|err| format!("Fixture startup task failed: {err}"))?
    .map(recording_payload::from_started)
}

#[tauri::command]
pub(crate) async fn stop_recording(
    app: AppHandle,
    recorder: State<'_, RecorderState>,
) -> Result<ThreadDetail, String> {
    let recorder = recorder.inner().clone();
    tauri::async_runtime::spawn_blocking(move || recording::stop_recording(app, recorder))
        .await
        .map_err(|err| format!("Audio stop task failed: {err}"))?
}
