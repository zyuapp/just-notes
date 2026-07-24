use tauri::{AppHandle, State};

use crate::{
    app::AppPaths,
    ipc::RecordingPayload,
    recording::{self, RecorderState},
    recording_payload,
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

#[tauri::command]
// Tauri injects each managed state and command argument independently.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn reprocess_thread(
    app: AppHandle,
    paths: State<'_, AppPaths>,
    recorder: State<'_, RecorderState>,
    settings: State<'_, SettingsState>,
    finalize: State<'_, FinalizeState>,
    thread_id: String,
) -> Result<(), String> {
    let paths = paths.inner().clone();
    let recorder = recorder.inner().clone();
    let settings = settings.inner().clone();
    let finalize = finalize.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let app_settings = settings.snapshot();
        recording::reprocess_thread(recording::ReprocessRequest {
            app,
            paths,
            recorder,
            settings: app_settings,
            finalize,
            thread_id,
        })
    })
    .await
    .map_err(|err| format!("Reprocess task failed: {err}"))?
}

#[tauri::command]
pub(crate) fn cancel_finalization(finalize: State<'_, FinalizeState>, thread_id: String) -> bool {
    finalize.cancel(&thread_id)
}
