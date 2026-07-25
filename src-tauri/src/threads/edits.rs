use std::fs;

use crate::app::AppPaths;

use self::fs_move::move_thread_dir;
use super::{
    repository::{
        list_threads, load_thread_by_id, load_thread_detail, read_thread_metadata,
        render_thread_markdown, update_thread_metadata_preserving_activity,
    },
    title::validated_title,
    transcript_store::read_transcript_jsonl,
    ThreadDetail, ThreadSummary,
};

mod fs_move;

pub(crate) fn rename_thread(
    paths: &AppPaths,
    thread_id: &str,
    title: &str,
) -> Result<ThreadDetail, String> {
    let title = validated_title(title)?;

    let thread_dir = existing_thread_dir(paths, thread_id)?;
    ensure_thread_not_busy(&thread_dir)?;
    update_thread_metadata_preserving_activity(&thread_dir, |metadata| {
        metadata.title = title.to_string();
    })?;
    rerender_markdown_if_present(&thread_dir)?;
    load_thread_detail(&thread_dir)
}

pub(crate) fn archive_thread(paths: &AppPaths, thread_id: &str) -> Result<(), String> {
    let detail = load_thread_by_id(paths, thread_id)?;
    if detail.summary.status.is_busy() {
        return Err(
            "Stop the active recording or transcription before archiving this thread".to_string(),
        );
    }
    move_thread_dir(
        &paths.thread_dir(thread_id),
        &paths.archived_thread_dir(thread_id),
    )
}

pub(crate) fn restore_thread(paths: &AppPaths, thread_id: &str) -> Result<(), String> {
    let source = paths.archived_thread_dir(thread_id);
    if !source.is_dir() {
        return Err(format!("Archived thread does not exist: {thread_id}"));
    }
    move_thread_dir(&source, &paths.thread_dir(thread_id))
}

// Permanent removal of an archived thread. The live store only ever archives
// (a reversible move), so the irreversible delete is scoped to the archive.
pub(crate) fn delete_thread(paths: &AppPaths, thread_id: &str) -> Result<(), String> {
    let thread_dir = paths.archived_thread_dir(thread_id);
    if !thread_dir.is_dir() {
        return Err(format!("Archived thread does not exist: {thread_id}"));
    }
    fs::remove_dir_all(&thread_dir)
        .map_err(|err| format!("Failed to delete {}: {err}", thread_dir.display()))
}

pub(crate) fn search_threads(paths: &AppPaths, query: &str) -> Result<Vec<ThreadSummary>, String> {
    let query = query.trim().to_lowercase();
    let threads = list_threads(paths)?;
    if query.is_empty() {
        return Ok(threads);
    }

    Ok(threads
        .into_iter()
        .filter(|thread| {
            thread.title.to_lowercase().contains(&query)
                || transcript_contains(paths, &thread.id, &query)
        })
        .collect())
}

fn transcript_contains(paths: &AppPaths, thread_id: &str, query: &str) -> bool {
    let jsonl_path = paths.thread_dir(thread_id).join("transcript.jsonl");
    read_transcript_jsonl(&jsonl_path)
        .map(|segments| {
            segments
                .iter()
                .any(|segment| segment.text.to_lowercase().contains(query))
        })
        .unwrap_or(false)
}

fn existing_thread_dir(paths: &AppPaths, thread_id: &str) -> Result<std::path::PathBuf, String> {
    let thread_dir = paths.thread_dir(thread_id);
    if !thread_dir.is_dir() {
        return Err(format!("Thread does not exist: {thread_id}"));
    }
    Ok(thread_dir)
}

// Renames write thread.json, which the live recorder and the finalization pass
// also write; they must wait until the thread is idle.
fn ensure_thread_not_busy(thread_dir: &std::path::Path) -> Result<(), String> {
    let metadata = read_thread_metadata(&thread_dir.join("thread.json"))?;
    if metadata.status.is_busy() {
        return Err(
            "Wait for recording or transcription to finish before editing this thread".to_string(),
        );
    }
    Ok(())
}

fn rerender_markdown_if_present(thread_dir: &std::path::Path) -> Result<(), String> {
    if thread_dir.join("transcript.md").is_file() {
        render_thread_markdown(thread_dir)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
