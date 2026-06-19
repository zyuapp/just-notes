use super::{parakeet_result_segments, SegmentIdentity, PARAKEET_MAX_SEGMENT_MS};
use sherpa_onnx::OfflineRecognizerResult;

#[test]
fn parakeet_segments_use_token_timestamps() {
    let result = OfflineRecognizerResult {
        text: "Hello there. Back to notes.".to_string(),
        tokens: [
            "\u{2581}Hello",
            "\u{2581}there",
            ".",
            "\u{2581}Back",
            "\u{2581}to",
            "\u{2581}notes",
            ".",
        ]
        .iter()
        .map(|token| token.to_string())
        .collect(),
        timestamps: Some(vec![1.0, 1.4, 1.8, 4.0, 4.4, 4.8, 5.2]),
        durations: Some(vec![0.3; 7]),
    };

    let segments = parakeet_result_segments(
        &result,
        8_000,
        SegmentIdentity {
            source: "mic",
            speaker: "You",
        },
    );

    assert_eq!(segments.len(), 2);
    assert_eq!(segments[0].text, "Hello there.");
    assert_eq!(segments[0].start_ms, 1_000);
    assert!(segments[0].end_ms < segments[1].start_ms);
    assert_eq!(segments[1].text, "Back to notes.");
}

#[test]
fn parakeet_fallback_splits_long_untimed_results() {
    let result = OfflineRecognizerResult {
        text: "one two three four five six seven eight".to_string(),
        tokens: Vec::new(),
        timestamps: None,
        durations: None,
    };

    let segments = parakeet_result_segments(
        &result,
        PARAKEET_MAX_SEGMENT_MS * 2,
        SegmentIdentity {
            source: "system",
            speaker: "Others",
        },
    );

    assert_eq!(segments.len(), 2);
    assert_eq!(segments[0].start_ms, 0);
    assert!(segments[0].end_ms <= PARAKEET_MAX_SEGMENT_MS);
    assert_eq!(segments[1].source, "system");
}
