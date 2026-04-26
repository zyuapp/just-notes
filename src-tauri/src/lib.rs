use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager};

mod app;
mod capture;
mod ipc;
mod recording;
mod threads;
mod transcription;

use app::AppPaths;
use ipc::{AppInfo, RecordingPayload};
use recording::RecorderState;
use threads::repository::{
    create_thread as create_thread_record, list_threads as list_thread_records, load_thread_by_id,
    reset_stale_recording_threads,
};
use threads::{ThreadDetail, ThreadSummary};
use transcription::{transcription_status, TranscriptionStatusPayload};

#[tauri::command]
fn get_app_info(paths: tauri::State<'_, AppPaths>) -> AppInfo {
    let paths = paths.inner();
    AppInfo {
        data_dir: paths.data_dir.display().to_string(),
        threads_dir: paths.threads_dir.display().to_string(),
        fixture_mode: cfg!(any(debug_assertions, feature = "qa-fixtures")),
    }
}

#[tauri::command]
fn list_threads(paths: tauri::State<'_, AppPaths>) -> Result<Vec<ThreadSummary>, String> {
    list_thread_records(paths.inner())
}

#[tauri::command]
fn create_thread(paths: tauri::State<'_, AppPaths>) -> Result<ThreadDetail, String> {
    create_thread_record(paths.inner())
}

#[tauri::command]
fn get_thread(
    paths: tauri::State<'_, AppPaths>,
    thread_id: String,
) -> Result<ThreadDetail, String> {
    load_thread_by_id(paths.inner(), &thread_id)
}

#[tauri::command]
fn get_transcription_status(
    paths: tauri::State<'_, AppPaths>,
) -> Result<TranscriptionStatusPayload, String> {
    Ok(transcription_status(paths.inner()))
}

#[tauri::command]
async fn start_recording(
    app: AppHandle,
    paths: tauri::State<'_, AppPaths>,
    recorder: tauri::State<'_, RecorderState>,
    thread_id: Option<String>,
) -> Result<RecordingPayload, String> {
    let paths = paths.inner().clone();
    let recorder = recorder.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        recording::start_recording(app, paths, recorder, thread_id)
    })
    .await
    .map_err(|err| format!("Audio startup task failed: {err}"))?
}

#[cfg(any(debug_assertions, feature = "qa-fixtures"))]
#[tauri::command]
async fn start_fixture_recording(
    app: AppHandle,
    paths: tauri::State<'_, AppPaths>,
    recorder: tauri::State<'_, RecorderState>,
    thread_id: Option<String>,
) -> Result<RecordingPayload, String> {
    let paths = paths.inner().clone();
    let recorder = recorder.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        recording::start_fixture_recording(app, paths, recorder, thread_id)
    })
    .await
    .map_err(|err| format!("Fixture startup task failed: {err}"))?
}

#[tauri::command]
async fn stop_recording(
    paths: tauri::State<'_, AppPaths>,
    recorder: tauri::State<'_, RecorderState>,
) -> Result<ThreadDetail, String> {
    let paths = paths.inner().clone();
    let recorder = recorder.inner().clone();
    tauri::async_runtime::spawn_blocking(move || recording::stop_recording(paths, recorder))
        .await
        .map_err(|err| format!("Audio stop task failed: {err}"))?
}

pub(crate) fn now_ms() -> Result<u64, String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("System clock is before UNIX epoch: {err}"))?
        .as_millis() as u64)
}

pub fn run() {
    let paths = AppPaths::discover().expect("failed to locate Just Notes data directory");

    let builder = tauri::Builder::default()
        .manage(paths)
        .manage(RecorderState::default())
        .setup(|app| {
            reset_stale_recording_threads(app.state::<AppPaths>().inner())?;
            Ok(())
        });

    #[cfg(any(debug_assertions, feature = "qa-fixtures"))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        get_app_info,
        list_threads,
        create_thread,
        get_thread,
        get_transcription_status,
        start_recording,
        start_fixture_recording,
        stop_recording
    ]);

    #[cfg(not(any(debug_assertions, feature = "qa-fixtures")))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        get_app_info,
        list_threads,
        create_thread,
        get_thread,
        get_transcription_status,
        start_recording,
        stop_recording
    ]);

    builder
        .run(tauri::generate_context!())
        .expect("error while running Just Notes");
}
