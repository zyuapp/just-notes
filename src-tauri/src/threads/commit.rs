use std::path::Path;

use super::{
    repository::{render_thread_markdown, touch_thread},
    transcript_store::{read_transcript_jsonl, write_transcript_jsonl},
    TranscriptSegment,
};

#[derive(Clone, Copy)]
pub(crate) enum CommitMode {
    /// Overwrite the thread's transcript with these segments.
    Replace,
    /// Keep the existing transcript and append these segments, shifting their
    /// timestamps by `offset_ms` (the prior recording length) so a resumed
    /// session lands after the earlier content.
    Append { offset_ms: u64 },
}

/// Persists a finalized transcript for a thread: writes the JSONL per `mode`,
/// bumps the thread's updated-at, and re-renders markdown when copies are
/// enabled.
pub(crate) fn commit_transcript(
    thread_dir: &Path,
    segments: &[TranscriptSegment],
    mode: CommitMode,
    markdown_copy: bool,
) -> Result<(), String> {
    let path = thread_dir.join("transcript.jsonl");
    match mode {
        CommitMode::Replace => write_transcript_jsonl(&path, segments)?,
        CommitMode::Append { offset_ms } => append_transcript(&path, segments, offset_ms)?,
    }
    touch_thread(thread_dir)?;
    if markdown_copy {
        render_thread_markdown(thread_dir)?;
    }
    Ok(())
}

fn append_transcript(
    path: &Path,
    segments: &[TranscriptSegment],
    offset_ms: u64,
) -> Result<(), String> {
    let mut combined = read_transcript_jsonl(path)?;
    combined.extend(segments.iter().cloned().map(|mut segment| {
        segment.start_ms = segment.start_ms.saturating_add(offset_ms);
        segment.end_ms = segment.end_ms.saturating_add(offset_ms);
        segment
    }));
    combined.sort_by(|left, right| {
        left.start_ms
            .cmp(&right.start_ms)
            .then_with(|| left.source.cmp(&right.source))
    });
    write_transcript_jsonl(path, &combined)
}
