use tauri::State;

use super::effective_paths;
use crate::{
    app::AppPaths,
    settings::SettingsState,
    threads::{edits, repository, ThreadDetail, ThreadSummary},
};

#[tauri::command]
pub(crate) fn list_threads(
    paths: State<'_, AppPaths>,
    settings: State<'_, SettingsState>,
) -> Result<Vec<ThreadSummary>, String> {
    repository::list_threads(&effective_paths(&paths, &settings))
}

#[tauri::command]
pub(crate) fn create_thread(
    paths: State<'_, AppPaths>,
    settings: State<'_, SettingsState>,
) -> Result<ThreadDetail, String> {
    repository::create_thread(&effective_paths(&paths, &settings))
}

#[tauri::command]
pub(crate) fn get_thread(
    paths: State<'_, AppPaths>,
    settings: State<'_, SettingsState>,
    thread_id: String,
) -> Result<ThreadDetail, String> {
    repository::load_thread_by_id(&effective_paths(&paths, &settings), &thread_id)
}

#[tauri::command]
pub(crate) fn rename_thread(
    paths: State<'_, AppPaths>,
    settings: State<'_, SettingsState>,
    thread_id: String,
    title: String,
) -> Result<ThreadDetail, String> {
    edits::rename_thread(&effective_paths(&paths, &settings), &thread_id, &title)
}

#[tauri::command]
pub(crate) fn list_archived_threads(
    paths: State<'_, AppPaths>,
    settings: State<'_, SettingsState>,
) -> Result<Vec<ThreadSummary>, String> {
    repository::list_archived_threads(&effective_paths(&paths, &settings))
}

#[tauri::command]
pub(crate) fn archive_thread(
    paths: State<'_, AppPaths>,
    settings: State<'_, SettingsState>,
    thread_id: String,
) -> Result<(), String> {
    edits::archive_thread(&effective_paths(&paths, &settings), &thread_id)
}

#[tauri::command]
pub(crate) fn restore_thread(
    paths: State<'_, AppPaths>,
    settings: State<'_, SettingsState>,
    thread_id: String,
) -> Result<(), String> {
    edits::restore_thread(&effective_paths(&paths, &settings), &thread_id)
}

#[tauri::command]
pub(crate) fn delete_thread(
    paths: State<'_, AppPaths>,
    settings: State<'_, SettingsState>,
    thread_id: String,
) -> Result<(), String> {
    edits::delete_thread(&effective_paths(&paths, &settings), &thread_id)
}

#[tauri::command]
pub(crate) fn rename_speaker(
    paths: State<'_, AppPaths>,
    settings: State<'_, SettingsState>,
    thread_id: String,
    speaker: String,
    label: String,
) -> Result<ThreadDetail, String> {
    edits::rename_speaker(
        &effective_paths(&paths, &settings),
        &thread_id,
        &speaker,
        &label,
    )
}

#[tauri::command]
pub(crate) fn update_segment_text(
    paths: State<'_, AppPaths>,
    settings: State<'_, SettingsState>,
    thread_id: String,
    segment_index: usize,
    text: String,
) -> Result<ThreadDetail, String> {
    edits::update_segment_text(
        &effective_paths(&paths, &settings),
        &thread_id,
        segment_index,
        &text,
    )
}

#[tauri::command]
pub(crate) fn search_threads(
    paths: State<'_, AppPaths>,
    settings: State<'_, SettingsState>,
    query: String,
) -> Result<Vec<ThreadSummary>, String> {
    edits::search_threads(&effective_paths(&paths, &settings), &query)
}

#[tauri::command]
pub(crate) fn export_thread_markdown(
    paths: State<'_, AppPaths>,
    settings: State<'_, SettingsState>,
    thread_id: String,
) -> Result<String, String> {
    let paths = effective_paths(&paths, &settings);
    let thread_dir = paths.thread_dir(&thread_id);
    if !thread_dir.is_dir() {
        return Err(format!("Thread does not exist: {thread_id}"));
    }
    repository::render_thread_markdown(&thread_dir)?;
    Ok(thread_dir.join("transcript.md").display().to_string())
}
