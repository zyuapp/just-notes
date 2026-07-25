use tauri::State;

use crate::{
    app::AppPaths,
    ipc::StorageUsagePayload,
    recording::{self, RecorderState},
    threads::{create, edits, repository, storage, StorageUsage, ThreadDetail, ThreadSummary},
};

#[tauri::command]
pub(crate) fn list_threads(paths: State<'_, AppPaths>) -> Result<Vec<ThreadSummary>, String> {
    repository::list_threads(&paths)
}

#[tauri::command]
pub(crate) fn create_thread(paths: State<'_, AppPaths>) -> Result<ThreadDetail, String> {
    create::create_thread(&paths)
}

#[tauri::command]
pub(crate) fn get_thread(
    paths: State<'_, AppPaths>,
    thread_id: String,
) -> Result<ThreadDetail, String> {
    repository::load_thread_by_id(&paths, &thread_id)
}

#[tauri::command]
pub(crate) fn rename_thread(
    paths: State<'_, AppPaths>,
    thread_id: String,
    title: String,
) -> Result<ThreadDetail, String> {
    edits::rename_thread(&paths, &thread_id, &title)
}

#[tauri::command]
pub(crate) fn list_archived_threads(
    paths: State<'_, AppPaths>,
) -> Result<Vec<ThreadSummary>, String> {
    repository::list_archived_threads(&paths)
}

#[tauri::command]
pub(crate) fn archive_thread(paths: State<'_, AppPaths>, thread_id: String) -> Result<(), String> {
    edits::archive_thread(&paths, &thread_id)
}

#[tauri::command]
pub(crate) fn restore_thread(paths: State<'_, AppPaths>, thread_id: String) -> Result<(), String> {
    edits::restore_thread(&paths, &thread_id)
}

#[tauri::command]
pub(crate) fn delete_thread(paths: State<'_, AppPaths>, thread_id: String) -> Result<(), String> {
    edits::delete_thread(&paths, &thread_id)
}

#[tauri::command]
pub(crate) fn search_threads(
    paths: State<'_, AppPaths>,
    query: String,
) -> Result<Vec<ThreadSummary>, String> {
    edits::search_threads(&paths, &query)
}

#[tauri::command]
pub(crate) fn export_thread_markdown(
    paths: State<'_, AppPaths>,
    thread_id: String,
) -> Result<String, String> {
    repository::export_thread_markdown(&paths, &thread_id)
}

#[tauri::command]
pub(crate) async fn get_storage_usage(
    paths: State<'_, AppPaths>,
) -> Result<StorageUsagePayload, String> {
    let paths = paths.inner().clone();
    tauri::async_runtime::spawn_blocking(move || storage::usage(&paths).map(to_usage_payload))
        .await
        .map_err(|err| format!("Storage usage task failed: {err}"))?
}

#[tauri::command]
pub(crate) async fn delete_reclaimable_raw_audio(
    paths: State<'_, AppPaths>,
    recorder: State<'_, RecorderState>,
) -> Result<StorageUsagePayload, String> {
    let paths = paths.inner().clone();
    let recorder = recorder.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        recording::reclaim_raw_audio(&paths, &recorder).map(to_usage_payload)
    })
    .await
    .map_err(|err| format!("Raw audio cleanup task failed: {err}"))?
}

fn to_usage_payload(usage: StorageUsage) -> StorageUsagePayload {
    StorageUsagePayload {
        total_bytes: usage.total_bytes,
        raw_audio_bytes: usage.raw_audio_bytes,
        reclaimable_bytes: usage.reclaimable_bytes,
        thread_count: usage.thread_count,
    }
}
