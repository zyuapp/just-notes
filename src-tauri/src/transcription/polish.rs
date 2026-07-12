//! Stop-time transcript polish. The live worker publishes each channel's
//! segments with no cross-channel context, and no re-transcription pass runs
//! on stop — so speaker bleed (system audio attributed to "You") and
//! hallucinated fillers would otherwise stay in the transcript forever. This
//! pass runs the transcript suppressors over the committed transcript while
//! the recording's WAVs are still on disk.

use std::path::Path;

use crate::threads::{
    commit::commit_transcript, transcript_store::read_transcript_jsonl, RecordingAudioPaths,
    TranscriptSegment,
};

use super::{
    suppress_cross_channel_bleed, suppress_isolated_faint_fillers,
    suppress_system_dominated_mic_segments,
};

pub(crate) fn polish_thread_transcript(
    thread_dir: &Path,
    audio: &RecordingAudioPaths,
) -> Result<(), String> {
    let jsonl_path = thread_dir.join("transcript.jsonl");
    let segments = read_transcript_jsonl(&jsonl_path)?;
    if segments.is_empty() {
        return Ok(());
    }

    let mut segments = segments;
    segments.sort_by(compare_segments);
    let before = segments.len();
    let segments = suppress_cross_channel_bleed(segments);
    let segments =
        suppress_system_dominated_mic_segments(segments, audio.mic_path(), audio.system_path())?;
    let segments =
        suppress_isolated_faint_fillers(segments, audio.mic_path(), audio.system_path())?;
    if segments.len() == before {
        return Ok(());
    }
    commit_transcript(thread_dir, &segments, false)
}

fn compare_segments(left: &TranscriptSegment, right: &TranscriptSegment) -> std::cmp::Ordering {
    left.start_ms
        .cmp(&right.start_ms)
        .then_with(|| left.end_ms.cmp(&right.end_ms))
        .then_with(|| left.source.cmp(&right.source))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::polish_thread_transcript;
    use crate::app::AppPaths;
    use crate::threads::{
        commit::append_thread_segments, create::create_thread,
        transcript_store::read_transcript_jsonl, RecordingAudioPaths, TranscriptSegment,
    };

    fn segment(source: &str, start_ms: u64, end_ms: u64, text: &str) -> TranscriptSegment {
        TranscriptSegment {
            speaker: if source == "mic" { "You" } else { "Others" }.to_string(),
            source: source.to_string(),
            start_ms,
            end_ms,
            text: text.to_string(),
        }
    }

    fn temp_thread_dir() -> (PathBuf, PathBuf) {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "just-notes-polish-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let paths = AppPaths {
            data_dir: root.clone(),
            threads_dir: root.join("threads"),
            archived_dir: root.join("archived"),
        };
        let detail = create_thread(&paths).expect("create temp thread");
        (root, paths.thread_dir(&detail.summary.id))
    }

    #[test]
    fn removes_mic_bleed_from_the_committed_transcript() {
        let (root, dir) = temp_thread_dir();
        append_thread_segments(
            &dir,
            &[
                segment("mic", 1_100, 5_100, "The quarterly numbers look strong."),
                segment("system", 1_000, 5_000, "The quarterly numbers look strong."),
            ],
        )
        .expect("seed transcript");

        polish_thread_transcript(&dir, &RecordingAudioPaths::for_thread_dir(&dir))
            .expect("polish transcript");

        let kept = read_transcript_jsonl(&dir.join("transcript.jsonl")).expect("read transcript");
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].source, "system");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_transcript_is_a_no_op() {
        let (root, dir) = temp_thread_dir();
        polish_thread_transcript(&dir, &RecordingAudioPaths::for_thread_dir(&dir))
            .expect("polish empty thread");
        let _ = std::fs::remove_dir_all(&root);
    }
}
