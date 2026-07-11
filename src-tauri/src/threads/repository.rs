use std::{fs, path::Path};

use crate::app::{now_ms, AppPaths};

use super::{
    transcript_store::{
        count_jsonl_lines, read_first_segment_text, read_transcript_jsonl, write_text_atomic,
    },
    ThreadDetail, ThreadMetadata, ThreadStatus, ThreadSummary,
};

pub(crate) fn list_threads(paths: &AppPaths) -> Result<Vec<ThreadSummary>, String> {
    paths.ensure()?;
    list_thread_summaries_in(&paths.threads_dir)
}

pub(crate) fn list_archived_threads(paths: &AppPaths) -> Result<Vec<ThreadSummary>, String> {
    list_thread_summaries_in(&paths.archived_dir)
}

fn list_thread_summaries_in(dir: &Path) -> Result<Vec<ThreadSummary>, String> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut threads = fs::read_dir(dir)
        .map_err(|err| format!("Failed to read {}: {err}", dir.display()))?
        .filter_map(Result::ok)
        .filter_map(|entry| load_thread_summary(&entry.path()).ok())
        .collect::<Vec<_>>();

    threads.sort_by(|left, right| {
        right
            .updated_at_ms
            .cmp(&left.updated_at_ms)
            .then_with(|| right.created_at_ms.cmp(&left.created_at_ms))
    });
    Ok(threads)
}

pub(crate) fn load_thread_by_id(paths: &AppPaths, thread_id: &str) -> Result<ThreadDetail, String> {
    let thread_dir = paths.thread_dir(thread_id);
    if !thread_dir.is_dir() {
        return Err(format!("Thread does not exist: {thread_id}"));
    }

    load_thread_detail(&thread_dir)
}

pub(crate) fn set_thread_status(thread_dir: &Path, status: ThreadStatus) -> Result<(), String> {
    update_thread_metadata(thread_dir, |metadata| metadata.status = status)
}

pub(crate) fn set_thread_duration(thread_dir: &Path, duration_ms: u64) -> Result<(), String> {
    update_thread_metadata(thread_dir, |metadata| metadata.duration_ms = duration_ms)
}

pub(crate) fn touch_thread(thread_dir: &Path) -> Result<(), String> {
    update_thread_metadata(thread_dir, |_| {})
}

pub(crate) fn update_thread_metadata(
    thread_dir: &Path,
    apply: impl FnOnce(&mut ThreadMetadata),
) -> Result<(), String> {
    let mut metadata = metadata_with_change(thread_dir, apply)?;
    metadata.updated_at_ms = now_ms()?;
    save_thread_metadata(thread_dir, &metadata)
}

pub(crate) fn update_thread_metadata_preserving_activity(
    thread_dir: &Path,
    apply: impl FnOnce(&mut ThreadMetadata),
) -> Result<(), String> {
    let metadata = metadata_with_change(thread_dir, apply)?;
    save_thread_metadata(thread_dir, &metadata)
}

fn metadata_with_change(
    thread_dir: &Path,
    apply: impl FnOnce(&mut ThreadMetadata),
) -> Result<ThreadMetadata, String> {
    let mut metadata = read_thread_metadata(&thread_dir.join("thread.json"))?;
    apply(&mut metadata);
    Ok(metadata)
}

pub(crate) fn reset_stale_recording_threads(paths: &AppPaths) -> Result<(), String> {
    paths.ensure()?;
    for entry in fs::read_dir(&paths.threads_dir)
        .map_err(|err| format!("Failed to read {}: {err}", paths.threads_dir.display()))?
    {
        let entry = entry
            .map_err(|err| format!("Failed to read {}: {err}", paths.threads_dir.display()))?;
        let thread_dir = entry.path();
        if !thread_dir.is_dir() || !thread_dir.join("thread.json").is_file() {
            continue;
        }
        let metadata = read_thread_metadata(&thread_dir.join("thread.json"))?;
        if metadata.status.is_busy() {
            set_thread_status(&thread_dir, ThreadStatus::Idle)?;
        }
    }
    Ok(())
}

pub(crate) fn prepare_work_dir(thread_dir: &Path) -> Result<(), String> {
    let work_dir = thread_dir.join("work");
    if work_dir.exists() {
        fs::remove_dir_all(&work_dir).map_err(|err| {
            format!(
                "Failed to clear transcription work directory {}: {err}",
                work_dir.display()
            )
        })?;
    }
    fs::create_dir_all(&work_dir).map_err(|err| {
        format!(
            "Failed to create transcription work directory {}: {err}",
            work_dir.display()
        )
    })
}

pub(crate) fn render_thread_markdown(thread_dir: &Path) -> Result<(), String> {
    let metadata = read_thread_metadata(&thread_dir.join("thread.json"))?;
    let segments = read_transcript_jsonl(&thread_dir.join("transcript.jsonl"))?;
    let duration_ms = metadata.duration_ms;
    let mut markdown = String::new();
    markdown.push_str(&format!("# {}\n\n", metadata.title));
    markdown.push_str(&format!("Thread: `{}`\n\n", metadata.id));
    markdown.push_str(&format!(
        "Duration: `{}`\n\n",
        format_transcript_time(duration_ms)
    ));

    for segment in segments {
        let speaker = metadata
            .speaker_labels
            .get(&segment.speaker)
            .unwrap_or(&segment.speaker);
        markdown.push_str(&format!(
            "[{}] **{}:** {}\n\n",
            format_transcript_time(segment.start_ms),
            speaker,
            segment.text
        ));
    }

    write_text_atomic(&thread_dir.join("transcript.md"), &markdown)
}

pub(crate) fn export_thread_markdown(paths: &AppPaths, thread_id: &str) -> Result<String, String> {
    let thread_dir = paths.thread_dir(thread_id);
    if !thread_dir.is_dir() {
        return Err(format!("Thread does not exist: {thread_id}"));
    }
    render_thread_markdown(&thread_dir)?;
    Ok(thread_dir.join("transcript.md").display().to_string())
}

pub(crate) fn load_thread_detail(thread_dir: &Path) -> Result<ThreadDetail, String> {
    let summary = load_thread_summary(thread_dir)?;
    let metadata = read_thread_metadata(&thread_dir.join("thread.json"))?;
    let segments = read_transcript_jsonl(&thread_dir.join("transcript.jsonl"))?;
    Ok(ThreadDetail {
        summary,
        segments,
        speaker_labels: metadata.speaker_labels,
        transcript_markdown_path: thread_dir.join("transcript.md").display().to_string(),
    })
}

pub(crate) fn thread_has_audio(thread_dir: &Path) -> bool {
    super::RecordingAudioPaths::for_thread_dir(thread_dir).has_any()
}

fn load_thread_summary(thread_dir: &Path) -> Result<ThreadSummary, String> {
    if !thread_dir.is_dir() {
        return Err(format!("Not a thread folder: {}", thread_dir.display()));
    }

    let metadata_path = thread_dir.join("thread.json");
    let metadata = read_thread_metadata(&metadata_path)?;
    let jsonl_path = thread_dir.join("transcript.jsonl");
    Ok(ThreadSummary {
        id: metadata.id,
        title: metadata.title,
        created_at_ms: metadata.created_at_ms,
        updated_at_ms: metadata.updated_at_ms,
        status: metadata.status,
        segment_count: count_jsonl_lines(&jsonl_path)?,
        duration_ms: metadata.duration_ms,
        // A live append can leave a torn last line; a missing snippet must not
        // drop the thread from the library while recording.
        snippet: read_first_segment_text(&jsonl_path)
            .unwrap_or_default()
            .unwrap_or_default(),
        has_audio: thread_has_audio(thread_dir),
        path: thread_dir.display().to_string(),
    })
}

pub(crate) fn read_thread_metadata(path: &Path) -> Result<ThreadMetadata, String> {
    let json = fs::read_to_string(path)
        .map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&json).map_err(|err| format!("Invalid {}: {err}", path.display()))
}

pub(super) fn save_thread_metadata(
    thread_dir: &Path,
    metadata: &ThreadMetadata,
) -> Result<(), String> {
    let json = serde_json::to_string_pretty(metadata)
        .map_err(|err| format!("Failed to encode thread metadata: {err}"))?;
    write_text_atomic(&thread_dir.join("thread.json"), &json)
}

fn format_transcript_time(ms: u64) -> String {
    let total_seconds = ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{minutes:02}:{seconds:02}")
}
