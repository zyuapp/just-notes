use std::path::{Path, PathBuf};

use super::{
    repository::{render_thread_markdown, touch_thread},
    transcript_store::{append_transcript_jsonl, write_transcript_jsonl},
    TranscriptSegment,
};

fn transcript_jsonl_path(thread_dir: &Path) -> PathBuf {
    thread_dir.join("transcript.jsonl")
}

/// Appends live transcript segments to a thread as each utterance lands. O(1)
/// per call; readers sort, so append order is fine.
pub(crate) fn append_thread_segments(
    thread_dir: &Path,
    segments: &[TranscriptSegment],
) -> Result<(), String> {
    if segments.is_empty() {
        return Ok(());
    }
    append_transcript_jsonl(&transcript_jsonl_path(thread_dir), segments)
}

/// Overwrites a thread's transcript with `segments`, bumps its updated-at, and
/// re-renders markdown when copies are enabled. Used by the re-transcription
/// pass; live recording persists through [`append_thread_segments`].
pub(crate) fn commit_transcript(
    thread_dir: &Path,
    segments: &[TranscriptSegment],
    markdown_copy: bool,
) -> Result<(), String> {
    write_transcript_jsonl(&transcript_jsonl_path(thread_dir), segments)?;
    touch_thread(thread_dir)?;
    if markdown_copy {
        render_thread_markdown(thread_dir)?;
    }
    Ok(())
}
