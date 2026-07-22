use super::*;

#[derive(Clone)]
struct FixedTranscriber {
    segments: Vec<TranscriptSegment>,
}

impl Transcriber for FixedTranscriber {
    fn transcribe_segments(
        &self,
        _samples_16k: &[f32],
        _prompt: &str,
        _source: &str,
        _speaker: &str,
    ) -> Result<Vec<TranscriptSegment>, String> {
        Ok(self.segments.clone())
    }
}

struct FailingTranscriber;

impl Transcriber for FailingTranscriber {
    fn transcribe_segments(
        &self,
        _samples_16k: &[f32],
        _prompt: &str,
        _source: &str,
        _speaker: &str,
    ) -> Result<Vec<TranscriptSegment>, String> {
        Err("decode failed".to_string())
    }
}

fn segment(start_ms: u64, end_ms: u64, text: &str) -> TranscriptSegment {
    TranscriptSegment {
        speaker: "You".to_string(),
        source: "mic".to_string(),
        start_ms,
        end_ms,
        text: text.to_string(),
    }
}

fn utterance(samples: Vec<f32>) -> Utterance {
    Utterance {
        start_index: 16_000,
        sample_rate: 16_000,
        samples,
    }
}

fn role(source: &'static str) -> ChannelRole<'static> {
    ChannelRole {
        source,
        speaker: if source == "mic" { "You" } else { "Others" },
    }
}

#[test]
fn drops_filler_only_decode_from_faint_mic_audio() {
    let transcriber = FixedTranscriber {
        segments: vec![segment(100, 400, "Okay.")],
    };
    let result = transcribe_live_utterance(
        &transcriber,
        &utterance(vec![0.006; 16_000]),
        role("mic"),
        0,
    )
    .unwrap();
    assert!(result.is_empty());
}

#[test]
fn drops_reported_filler_variants_from_faint_mic_audio() {
    for text in ["Mm-hmm.", "Mmm-hmm.", "Okay.", "Yeah.", "Uh", "Um"] {
        let transcriber = FixedTranscriber {
            segments: vec![segment(100, 400, text)],
        };
        let result = transcribe_live_utterance(
            &transcriber,
            &utterance(vec![0.006; 16_000]),
            role("mic"),
            0,
        )
        .unwrap();
        assert!(result.is_empty(), "kept {text:?}");
    }
}

#[test]
fn keeps_a_confidently_voiced_backchannel() {
    let transcriber = FixedTranscriber {
        segments: vec![segment(100, 400, "Yeah.")],
    };
    let result =
        transcribe_live_utterance(&transcriber, &utterance(vec![0.04; 16_000]), role("mic"), 0)
            .unwrap();
    assert_eq!(result.len(), 1);
}

#[test]
fn keeps_faint_substantive_mic_speech() {
    let transcriber = FixedTranscriber {
        segments: vec![segment(100, 700, "Yeah, let me check the logs.")],
    };
    let result = transcribe_live_utterance(
        &transcriber,
        &utterance(vec![0.006; 16_000]),
        role("mic"),
        0,
    )
    .unwrap();
    assert_eq!(result.len(), 1);
}

#[test]
fn evaluates_each_decode_against_its_own_audio_window() {
    let transcriber = FixedTranscriber {
        segments: vec![segment(100, 300, "Okay."), segment(600, 900, "Yeah.")],
    };
    let mut samples = vec![0.006; 16_000];
    samples[9_600..14_400].fill(0.04);
    let result =
        transcribe_live_utterance(&transcriber, &utterance(samples), role("mic"), 0).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].text, "Yeah.");
}

#[test]
fn does_not_apply_the_mic_policy_to_system_audio() {
    let transcriber = FixedTranscriber {
        segments: vec![TranscriptSegment {
            speaker: "Others".to_string(),
            source: "system".to_string(),
            start_ms: 100,
            end_ms: 400,
            text: "Okay.".to_string(),
        }],
    };
    let result = transcribe_live_utterance(
        &transcriber,
        &utterance(vec![0.006; 16_000]),
        role("system"),
        0,
    )
    .unwrap();
    assert_eq!(result.len(), 1);
}

#[test]
fn shifts_kept_segment_times_by_utterance_and_resume_offsets() {
    let transcriber = FixedTranscriber {
        segments: vec![segment(100, 400, "A real sentence.")],
    };
    let result = transcribe_live_utterance(
        &transcriber,
        &utterance(vec![0.006; 16_000]),
        role("mic"),
        2_000,
    )
    .unwrap();
    assert_eq!(result[0].start_ms, 3_100);
    assert_eq!(result[0].end_ms, 3_400);
}

#[test]
fn propagates_recognizer_failures() {
    let result = transcribe_live_utterance(
        &FailingTranscriber,
        &utterance(vec![0.04; 16_000]),
        role("mic"),
        0,
    );
    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("expected recognizer failure"),
    };
    assert_eq!(error, "decode failed");
}
