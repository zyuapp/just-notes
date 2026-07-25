//! Stop-time transcript polish. The live worker publishes each channel's
//! segments with no cross-channel context, so speaker bleed (system audio
//! attributed to "You") and hallucinated fillers would otherwise stay in the
//! transcript forever. This is the only pass that sees both channels together,
//! and it runs while the recording's WAVs are still on disk.

use std::path::Path;

use crate::threads::{
    commit::commit_transcript, transcript_store::read_transcript_jsonl, RecordingAudioPaths,
    TranscriptSegment,
};

use super::{
    suppress_cross_channel_bleed, suppress_faint_mic_fillers,
    suppress_system_dominated_mic_segments,
};

pub(crate) fn polish_thread_transcript(
    thread_dir: &Path,
    audio: &RecordingAudioPaths,
    audio_offset_ms: u64,
) -> Result<(), String> {
    let jsonl_path = thread_dir.join("transcript.jsonl");
    let segments = read_transcript_jsonl(&jsonl_path)?;
    if segments.is_empty() {
        return Ok(());
    }

    let mut segments = segments;
    segments.sort_by(compare_segments);
    let before = segments.len();
    let segments = suppress_current_session(segments, audio, audio_offset_ms)?;
    if segments.len() == before {
        return Ok(());
    }
    commit_transcript(thread_dir, &segments, false)
}

/// A resumed recording's WAVs contain only the new session. Historical
/// transcript segments must not be compared with that unrelated audio or with
/// new turns that happen to fall within a suppressor's time padding.
fn suppress_current_session(
    segments: Vec<TranscriptSegment>,
    audio: &RecordingAudioPaths,
    audio_offset_ms: u64,
) -> Result<Vec<TranscriptSegment>, String> {
    let (mut historical, current): (Vec<_>, Vec<_>) = segments
        .into_iter()
        .partition(|segment| segment.start_ms < audio_offset_ms);
    let current = suppress_cross_channel_bleed(current);
    let current = suppress_system_dominated_mic_segments(
        current,
        audio.mic_path(),
        audio.system_path(),
        audio_offset_ms,
    )?;
    let current = suppress_faint_mic_fillers(current, audio.mic_path(), audio_offset_ms)?;
    historical.extend(current);
    Ok(historical)
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

    fn write_constant_wav(path: &std::path::Path, level: f32, duration_ms: u32) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).expect("create test wav");
        let sample = (level * f32::from(i16::MAX)).round() as i16;
        for _ in 0..(16_000 * duration_ms / 1_000) {
            writer.write_sample(sample).expect("write test sample");
        }
        writer.finalize().expect("finalize test wav");
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

        polish_thread_transcript(&dir, &RecordingAudioPaths::for_thread_dir(&dir), 0)
            .expect("polish transcript");

        let kept = read_transcript_jsonl(&dir.join("transcript.jsonl")).expect("read transcript");
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].source, "system");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn removes_faint_filler_next_to_real_conversation() {
        let (root, dir) = temp_thread_dir();
        append_thread_segments(
            &dir,
            &[
                segment("system", 1_000, 4_000, "Let me walk you through the plan."),
                segment("mic", 4_500, 4_820, "Okay."),
            ],
        )
        .expect("seed transcript");
        let audio = RecordingAudioPaths::for_thread_dir(&dir);
        write_constant_wav(audio.mic_path(), 0.006, 6_000);

        polish_thread_transcript(&dir, &audio, 0).expect("polish transcript");

        let kept = read_transcript_jsonl(&dir.join("transcript.jsonl")).expect("read transcript");
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].source, "system");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn keeps_a_confidently_voiced_backchannel() {
        let (root, dir) = temp_thread_dir();
        append_thread_segments(&dir, &[segment("mic", 500, 900, "Yeah.")])
            .expect("seed transcript");
        let audio = RecordingAudioPaths::for_thread_dir(&dir);
        write_constant_wav(audio.mic_path(), 0.04, 2_000);

        polish_thread_transcript(&dir, &audio, 0).expect("polish transcript");

        let kept = read_transcript_jsonl(&dir.join("transcript.jsonl")).expect("read transcript");
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].text, "Yeah.");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_transcript_is_a_no_op() {
        let (root, dir) = temp_thread_dir();
        polish_thread_transcript(&dir, &RecordingAudioPaths::for_thread_dir(&dir), 0)
            .expect("polish empty thread");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn resumed_session_uses_local_audio_for_new_fillers_and_preserves_history() {
        let (root, dir) = temp_thread_dir();
        append_thread_segments(
            &dir,
            &[
                segment("mic", 5_000, 5_400, "Okay."),
                segment("mic", 60_500, 60_900, "Yeah."),
            ],
        )
        .expect("seed transcript");
        let audio = RecordingAudioPaths::for_thread_dir(&dir);
        write_constant_wav(audio.mic_path(), 0.04, 2_000);

        polish_thread_transcript(&dir, &audio, 60_000).expect("polish resumed transcript");

        let kept = read_transcript_jsonl(&dir.join("transcript.jsonl")).expect("read transcript");
        assert_eq!(kept.len(), 2);
        assert_eq!(kept[0].text, "Okay.");
        assert_eq!(kept[1].text, "Yeah.");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn resumed_session_never_deduplicates_against_historical_turns() {
        let (root, dir) = temp_thread_dir();
        let repeated = "The quarterly numbers look strong this month.";
        append_thread_segments(
            &dir,
            &[
                segment("mic", 58_500, 59_900, repeated),
                segment("system", 60_000, 61_000, repeated),
                segment("mic", 60_100, 61_100, repeated),
            ],
        )
        .expect("seed transcript");

        polish_thread_transcript(&dir, &RecordingAudioPaths::for_thread_dir(&dir), 60_000)
            .expect("polish resumed transcript");

        let kept = read_transcript_jsonl(&dir.join("transcript.jsonl")).expect("read transcript");
        assert_eq!(kept.len(), 2);
        assert_eq!(kept[0].source, "mic");
        assert_eq!(kept[0].start_ms, 58_500);
        assert_eq!(kept[1].source, "system");
        let _ = std::fs::remove_dir_all(&root);
    }
}
