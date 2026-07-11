use crate::threads::TranscriptSegment;
use crate::transcription::text::normalized_words;

use super::fixtures::ReferenceSegment;

/// A hypothesis segment counts against a reference segment when their spans
/// overlap within this pad, absorbing segmenter boundary jitter.
const SEGMENT_MATCH_PAD_MS: u64 = 1_000;

pub(super) struct FixtureMetrics {
    pub(super) wer: f32,
    pub(super) reference_words: usize,
    pub(super) missed_segments: usize,
    pub(super) reference_segments: usize,
    pub(super) hallucinated_segments: usize,
}

pub(super) fn evaluate(
    reference: &[ReferenceSegment],
    hypothesis: &[TranscriptSegment],
) -> FixtureMetrics {
    let mut distance = 0usize;
    let mut reference_words = 0usize;
    for source in ["mic", "system"] {
        let ref_words = source_words(
            reference
                .iter()
                .filter(|segment| segment.source == source)
                .map(|segment| (segment.start_ms, segment.text.as_str())),
        );
        let hyp_words = source_words(
            hypothesis
                .iter()
                .filter(|segment| segment.source == source)
                .map(|segment| (segment.start_ms, segment.text.as_str())),
        );
        distance += levenshtein(&ref_words, &hyp_words);
        reference_words += ref_words.len();
    }

    let missed_segments = reference
        .iter()
        .filter(|reference_segment| {
            !hypothesis.iter().any(|hypothesis_segment| {
                same_source_overlap(reference_segment, hypothesis_segment, 0)
            })
        })
        .count();
    let hallucinated_segments = hypothesis
        .iter()
        .filter(|hypothesis_segment| {
            !reference.iter().any(|reference_segment| {
                same_source_overlap(reference_segment, hypothesis_segment, SEGMENT_MATCH_PAD_MS)
            })
        })
        .count();

    FixtureMetrics {
        wer: if reference_words == 0 {
            0.0
        } else {
            distance as f32 / reference_words as f32
        },
        reference_words,
        missed_segments,
        reference_segments: reference.len(),
        hallucinated_segments,
    }
}

fn source_words<'a>(segments: impl Iterator<Item = (u64, &'a str)>) -> Vec<String> {
    let mut ordered: Vec<(u64, &str)> = segments.collect();
    ordered.sort_by_key(|(start_ms, _)| *start_ms);
    ordered
        .into_iter()
        .flat_map(|(_, text)| normalized_words(text))
        .collect()
}

fn same_source_overlap(
    reference: &ReferenceSegment,
    hypothesis: &TranscriptSegment,
    pad_ms: u64,
) -> bool {
    reference.source == hypothesis.source
        && hypothesis.start_ms <= reference.end_ms.saturating_add(pad_ms)
        && reference.start_ms <= hypothesis.end_ms.saturating_add(pad_ms)
}

/// Word-level edit distance; with the reference length it yields WER.
fn levenshtein(reference: &[String], hypothesis: &[String]) -> usize {
    let mut previous: Vec<usize> = (0..=hypothesis.len()).collect();
    let mut current = vec![0usize; hypothesis.len() + 1];
    for (row, reference_word) in reference.iter().enumerate() {
        current[0] = row + 1;
        for (column, hypothesis_word) in hypothesis.iter().enumerate() {
            let substitution = usize::from(reference_word != hypothesis_word);
            current[column + 1] = (previous[column] + substitution)
                .min(previous[column + 1] + 1)
                .min(current[column] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[hypothesis.len()]
}

#[cfg(test)]
mod tests {
    use super::levenshtein;

    fn words(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_string).collect()
    }

    #[test]
    fn levenshtein_counts_substitutions_insertions_and_deletions() {
        assert_eq!(levenshtein(&words("a b c"), &words("a b c")), 0);
        assert_eq!(levenshtein(&words("a b c"), &words("a x c")), 1);
        assert_eq!(levenshtein(&words("a b c"), &words("a c")), 1);
        assert_eq!(levenshtein(&words("a c"), &words("a b c")), 1);
        assert_eq!(levenshtein(&words("a b"), &words("")), 2);
    }
}
