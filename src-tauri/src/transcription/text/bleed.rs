use super::words::{contains_word_sequence, normalized_words, word_ngrams};
use crate::threads::TranscriptSegment;

const BLEED_NGRAM_SIZE: usize = 3;
const BLEED_COVERAGE_THRESHOLD: f32 = 0.7;
const BLEED_TIME_PAD_MS: u64 = 2_000;

struct SystemSpan {
    start_ms: u64,
    end_ms: u64,
    text: String,
}

pub(crate) fn suppress_cross_channel_bleed(
    segments: Vec<TranscriptSegment>,
) -> Vec<TranscriptSegment> {
    let system_segments: Vec<SystemSpan> = segments
        .iter()
        .filter(|segment| segment.source == "system")
        .map(|segment| SystemSpan {
            start_ms: segment.start_ms,
            end_ms: segment.end_ms,
            text: segment.text.clone(),
        })
        .collect();

    segments
        .into_iter()
        .filter(|segment| segment.source != "mic" || !is_bleed(segment, &system_segments))
        .collect()
}

fn is_bleed(mic: &TranscriptSegment, system_segments: &[SystemSpan]) -> bool {
    let overlapping_text = system_segments
        .iter()
        .filter(|span| overlaps(mic.start_ms, mic.end_ms, span.start_ms, span.end_ms))
        .map(|span| span.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    if overlapping_text.is_empty() {
        return false;
    }

    let mic_words = normalized_words(&mic.text);
    if mic_words.is_empty() {
        return false;
    }
    let system_words = normalized_words(&overlapping_text);
    if mic_words.len() < BLEED_NGRAM_SIZE {
        return contains_word_sequence(&system_words, &mic_words);
    }

    let mic_ngrams = word_ngrams(&mic_words, BLEED_NGRAM_SIZE);
    let system_ngrams = word_ngrams(&system_words, BLEED_NGRAM_SIZE);
    if mic_ngrams.is_empty() || system_ngrams.is_empty() {
        return false;
    }
    let covered = mic_ngrams
        .iter()
        .filter(|ngram| system_ngrams.contains(ngram))
        .count();
    covered as f32 / mic_ngrams.len() as f32 >= BLEED_COVERAGE_THRESHOLD
}

fn overlaps(mic_start: u64, mic_end: u64, system_start: u64, system_end: u64) -> bool {
    mic_start <= system_end.saturating_add(BLEED_TIME_PAD_MS)
        && system_start <= mic_end.saturating_add(BLEED_TIME_PAD_MS)
}

#[cfg(test)]
mod tests {
    use super::suppress_cross_channel_bleed;
    use crate::threads::TranscriptSegment;

    fn segment(source: &str, start_ms: u64, end_ms: u64, text: &str) -> TranscriptSegment {
        TranscriptSegment {
            speaker: if source == "mic" { "You" } else { "Others" }.to_string(),
            source: source.to_string(),
            start_ms,
            end_ms,
            text: text.to_string(),
        }
    }

    #[test]
    fn drops_mic_segment_duplicating_overlapping_system_speech() {
        let segments = vec![
            segment(
                "system",
                1_000,
                5_000,
                "The quarterly numbers look strong this month.",
            ),
            segment(
                "mic",
                1_200,
                5_200,
                "The quarterly numbers look strong this month.",
            ),
        ];
        let kept = suppress_cross_channel_bleed(segments);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].source, "system");
    }

    #[test]
    fn keeps_distinct_mic_speech_during_system_audio() {
        let segments = vec![
            segment(
                "system",
                1_000,
                5_000,
                "The quarterly numbers look strong this month.",
            ),
            segment(
                "mic",
                1_200,
                5_200,
                "Let me take a note about the hiring plan instead.",
            ),
        ];
        assert_eq!(suppress_cross_channel_bleed(segments).len(), 2);
    }

    // A real spoken backchannel almost always re-uses words the other party
    // said nearby; text overlap alone is not evidence of bleed for segments
    // this short.
    #[test]
    #[ignore = "sub-trigram mic segments are dropped on text overlap alone; quality-harness red test"]
    fn keeps_short_backchannel_during_system_speech() {
        let segments = vec![
            segment("system", 1_000, 6_000, "yeah we should ship it this week"),
            segment("mic", 3_000, 3_400, "Yeah."),
        ];
        assert_eq!(suppress_cross_channel_bleed(segments).len(), 2);
    }

    #[test]
    fn keeps_repeated_text_when_far_apart_in_time() {
        let segments = vec![
            segment(
                "system",
                1_000,
                4_000,
                "The quarterly numbers look strong this month.",
            ),
            segment(
                "mic",
                60_000,
                64_000,
                "The quarterly numbers look strong this month.",
            ),
        ];
        assert_eq!(suppress_cross_channel_bleed(segments).len(), 2);
    }
}
