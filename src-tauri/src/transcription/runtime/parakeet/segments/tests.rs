use super::{
    clean_segment_text, parakeet_result_segments, SegmentIdentity, TranscriptSegment,
    PARAKEET_MAX_SEGMENT_MS,
};
use sherpa_onnx::OfflineRecognizerResult;

/// A recognizer token as `(token, start_seconds, duration_seconds)`.
type TimedWord<'a> = (&'a str, f32, f32);

fn timed_result(words: &[TimedWord<'_>]) -> OfflineRecognizerResult {
    OfflineRecognizerResult {
        text: words
            .iter()
            .map(|(token, _, _)| token.trim_start_matches('\u{2581}'))
            .collect::<Vec<_>>()
            .join(" "),
        tokens: words
            .iter()
            .map(|(token, _, _)| (*token).to_string())
            .collect(),
        timestamps: Some(words.iter().map(|(_, start, _)| *start).collect()),
        durations: Some(words.iter().map(|(_, _, duration)| *duration).collect()),
    }
}

fn you(result: &OfflineRecognizerResult, total_duration_ms: u64) -> Vec<TranscriptSegment> {
    parakeet_result_segments(
        result,
        total_duration_ms,
        SegmentIdentity {
            source: "mic",
            speaker: "You",
        },
    )
}

fn others(result: &OfflineRecognizerResult, total_duration_ms: u64) -> Vec<TranscriptSegment> {
    parakeet_result_segments(
        result,
        total_duration_ms,
        SegmentIdentity {
            source: "system",
            speaker: "Others",
        },
    )
}

#[test]
fn parakeet_segments_use_token_timestamps() {
    let result = timed_result(&[
        ("\u{2581}Hello", 1.0, 0.3),
        ("\u{2581}there", 1.4, 0.3),
        (".", 1.8, 0.3),
        ("\u{2581}Back", 4.0, 0.3),
        ("\u{2581}to", 4.4, 0.3),
        ("\u{2581}notes", 4.8, 0.3),
        (".", 5.2, 0.3),
    ]);

    let segments = you(&result, 8_000);

    assert_eq!(segments.len(), 2);
    assert_eq!(segments[0].text, "Hello there.");
    assert_eq!(segments[0].start_ms, 1_000);
    assert_eq!(segments[0].end_ms, 2_100);
    assert_eq!(segments[1].text, "Back to notes.");
    assert_eq!(segments[1].start_ms, 4_000);
    assert_eq!(segments[1].end_ms, 5_500);
}

#[test]
fn parakeet_does_not_split_on_sentence_end_before_minimum_duration() {
    let result = timed_result(&[
        ("\u{2581}Hi", 0.0, 0.05),
        (".", 0.1, 0.05),
        ("\u{2581}there", 0.6, 0.05),
        ("\u{2581}friend", 1.0, 0.05),
        (".", 1.4, 0.05),
    ]);

    let segments = you(&result, 4_000);

    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].text, "Hi. there friend.");
    assert_eq!(segments[0].start_ms, 0);
    assert_eq!(segments[0].end_ms, 1_450);
}

// A sentence long enough to span the other channel's turn must not sort every
// word it holds ahead of that turn, so it is emitted in capped pieces.
#[test]
fn parakeet_caps_the_span_of_one_long_sentence() {
    const WORDS: usize = 23;
    let tokens = (0..WORDS)
        .map(|index| {
            let stop = if index + 1 == WORDS { "." } else { "" };
            format!("\u{2581}word{index}{stop}")
        })
        .collect::<Vec<_>>();
    let result = OfflineRecognizerResult {
        text: tokens.join(" "),
        tokens,
        timestamps: Some((0..WORDS).map(|index| index as f32 * 0.4).collect()),
        durations: Some(vec![0.3; WORDS]),
    };

    let segments = others(&result, 9_400);

    assert!(segments.len() > 1);
    for segment in &segments {
        let span_ms = segment.end_ms - segment.start_ms;
        assert!(
            span_ms <= PARAKEET_MAX_SEGMENT_MS,
            "segment spans {span_ms}ms"
        );
    }
    let last = segments.last().expect("a final segment");
    assert!(
        last.start_ms >= PARAKEET_MAX_SEGMENT_MS,
        "the sentence tail still starts at {}ms",
        last.start_ms
    );
}

#[test]
fn parakeet_cuts_a_capped_sentence_at_its_longest_pause() {
    let result = timed_result(&[
        ("\u{2581}one", 0.0, 0.3),
        ("\u{2581}two", 0.4, 0.3),
        ("\u{2581}three", 0.8, 0.3),
        ("\u{2581}four", 2.0, 0.3),
        ("\u{2581}five", 2.4, 0.3),
        ("\u{2581}six", 2.8, 0.3),
        ("\u{2581}seven", 3.2, 0.3),
        ("\u{2581}eight.", 5.0, 0.3),
    ]);

    let segments = others(&result, 5_300);

    assert_eq!(segments.len(), 2);
    assert_eq!(segments[0].text, "one two three four five six seven");
    assert_eq!(segments[0].start_ms, 0);
    assert_eq!(segments[0].end_ms, 3_500);
    assert_eq!(segments[1].text, "eight.");
    assert_eq!(segments[1].start_ms, 5_000);
}

// Parakeet emits sub-word tokens, so the longest gap in a capped sentence can
// fall inside a word. A word start reachable within the cap wins over it.
#[test]
fn parakeet_prefers_a_word_boundary_over_a_wider_mid_word_gap() {
    let result = timed_result(&[
        ("\u{2581}one", 0.0, 0.3),
        ("\u{2581}two", 0.4, 0.3),
        ("\u{2581}ex", 0.8, 0.3),
        ("ist", 2.0, 0.3),
        ("ence", 2.4, 0.3),
        ("\u{2581}three", 2.8, 0.3),
        ("\u{2581}four.", 5.0, 0.3),
    ]);

    let segments = others(&result, 5_300);

    assert_eq!(segments.len(), 2);
    assert_eq!(segments[0].text, "one two existence three");
    assert_eq!(segments[1].text, "four.");
}

#[test]
fn parakeet_ignores_empty_results() {
    let result = OfflineRecognizerResult {
        text: String::new(),
        tokens: Vec::new(),
        timestamps: None,
        durations: None,
    };

    assert!(you(&result, 4_000).is_empty());
}

#[test]
fn clean_segment_text_removes_space_before_punctuation() {
    assert_eq!(clean_segment_text("hello , world ?"), "hello, world?");
    assert_eq!(clean_segment_text("one ; two : three"), "one; two: three");
}

#[test]
fn parakeet_fallback_splits_long_untimed_results() {
    let result = OfflineRecognizerResult {
        text: "one two three four five six seven eight".to_string(),
        tokens: Vec::new(),
        timestamps: None,
        durations: None,
    };

    let segments = others(&result, PARAKEET_MAX_SEGMENT_MS * 2);

    assert_eq!(segments.len(), 2);
    assert_eq!(segments[0].start_ms, 0);
    assert!(segments[0].end_ms <= PARAKEET_MAX_SEGMENT_MS);
    assert_eq!(segments[1].source, "system");
}
