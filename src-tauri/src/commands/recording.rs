use tauri::{AppHandle, State};

use super::effective_paths;
use crate::{
    app::AppPaths,
    ipc::RecordingPayload,
    recording::{self, RecorderState},
    settings::SettingsState,
    threads::ThreadDetail,
    transcription::FinalizeState,
};

#[tauri::command]
pub(crate) async fn start_recording(
    app: AppHandle,
    paths: State<'_, AppPaths>,
    recorder: State<'_, RecorderState>,
    settings: State<'_, SettingsState>,
    thread_id: Option<String>,
) -> Result<RecordingPayload, String> {
    let paths = effective_paths(&paths, &settings);
    let recorder = recorder.inner().clone();
    let settings = settings.snapshot();
    tauri::async_runtime::spawn_blocking(move || {
        recording::start_recording(app, paths, recorder, settings, thread_id)
    })
    .await
    .map_err(|err| format!("Audio startup task failed: {err}"))?
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
    let paths = effective_paths(&paths, &settings);
    let recorder = recorder.inner().clone();
    let settings = settings.snapshot();
    tauri::async_runtime::spawn_blocking(move || {
        recording::start_fixture_recording(app, paths, recorder, settings, thread_id)
    })
    .await
    .map_err(|err| format!("Fixture startup task failed: {err}"))?
}

#[tauri::command]
pub(crate) async fn stop_recording(
    app: AppHandle,
    paths: State<'_, AppPaths>,
    recorder: State<'_, RecorderState>,
    settings: State<'_, SettingsState>,
    finalize: State<'_, FinalizeState>,
) -> Result<ThreadDetail, String> {
    let paths = effective_paths(&paths, &settings);
    let recorder = recorder.inner().clone();
    let finalize = finalize.inner().clone();
    let settings = settings.snapshot();
    tauri::async_runtime::spawn_blocking(move || {
        recording::stop_recording(app, paths, recorder, settings, finalize)
    })
    .await
    .map_err(|err| format!("Audio stop task failed: {err}"))?
}

#[tauri::command]
pub(crate) fn cancel_finalization(finalize: State<'_, FinalizeState>, thread_id: String) -> bool {
    finalize.cancel(&thread_id)
}
