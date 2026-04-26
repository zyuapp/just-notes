use std::collections::VecDeque;

use super::words::{
    join_original_word_refs, join_original_words, normalized_words,
    remove_internal_repeated_sentences, sentence_ends_after, transcript_words, word_ngrams,
    TranscriptWord,
};

pub(crate) const LIVE_DUPLICATE_RECENT_SEGMENTS: usize = 8;

const LIVE_DUPLICATE_NGRAM_SIZE: usize = 3;
const LIVE_DUPLICATE_COVERAGE_THRESHOLD: f32 = 0.72;
const LIVE_DUPLICATE_MIN_KEEP_WORDS: usize = 4;
const LIVE_DUPLICATE_MIN_TRIM_WORDS: usize = 6;

pub(crate) fn unique_transcript_text(
    candidate: &str,
    recent_texts: &VecDeque<String>,
) -> Option<String> {
    let candidate = remove_internal_repeated_sentences(candidate, LIVE_DUPLICATE_MIN_KEEP_WORDS);
    if candidate.trim().is_empty() {
        return None;
    }
    if recent_texts.is_empty() {
        return Some(candidate);
    }

    let candidate_word_pairs = transcript_words(&candidate);
    let candidate_words = candidate_word_pairs
        .iter()
        .map(|word| word.normalized.clone())
        .collect::<Vec<_>>();
    if candidate_words.len() < LIVE_DUPLICATE_NGRAM_SIZE {
        let recent_words = recent_transcript_words(recent_texts);
        return (!contains_word_sequence(&recent_words, &candidate_words))
            .then(|| candidate.to_string());
    }

    let recent_words = recent_transcript_words(recent_texts);
    if recent_words.len() < LIVE_DUPLICATE_NGRAM_SIZE {
        return Some(candidate);
    }

    if duplicate_coverage(&candidate_words, &recent_words) >= LIVE_DUPLICATE_COVERAGE_THRESHOLD {
        return None;
    }

    if let Some(text) = trim_duplicate_suffix(&candidate_word_pairs, &recent_words) {
        return Some(text);
    }
    if let Some(text) = trim_duplicate_prefix(&candidate_word_pairs, &recent_words) {
        return Some(text);
    }
    if let Some(text) = remove_duplicate_word_spans(&candidate_word_pairs, &recent_words) {
        return Some(remove_internal_repeated_sentences(
            &text,
            LIVE_DUPLICATE_MIN_KEEP_WORDS,
        ));
    }

    Some(candidate)
}

fn recent_transcript_words(recent_texts: &VecDeque<String>) -> Vec<String> {
    let recent = recent_texts
        .iter()
        .rev()
        .take(LIVE_DUPLICATE_RECENT_SEGMENTS)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join(" ");
    normalized_words(&recent)
}

fn remove_duplicate_word_spans(
    words: &[TranscriptWord],
    recent_words: &[String],
) -> Option<String> {
    let mut keep = vec![true; words.len()];
    let normalized = words
        .iter()
        .map(|word| word.normalized.clone())
        .collect::<Vec<_>>();
    let mut changed = false;

    loop {
        let Some(span) = longest_common_word_span(&normalized, recent_words, &keep) else {
            break;
        };
        for item in keep.iter_mut().take(span.end).skip(span.start) {
            *item = false;
        }
        changed = true;
    }

    if !changed {
        return None;
    }

    let kept_words = words
        .iter()
        .zip(keep.iter())
        .filter_map(|(word, keep)| keep.then_some(word))
        .collect::<Vec<_>>();
    (kept_words.len() >= LIVE_DUPLICATE_MIN_KEEP_WORDS)
        .then(|| join_original_word_refs(&kept_words))
}

struct WordSpan {
    start: usize,
    end: usize,
}

fn longest_common_word_span(
    candidate_words: &[String],
    recent_words: &[String],
    keep: &[bool],
) -> Option<WordSpan> {
    let mut best = None;
    for start in 0..candidate_words.len() {
        if !keep[start] {
            continue;
        }

        for recent_start in 0..recent_words.len() {
            let mut length = 0usize;
            while start + length < candidate_words.len()
                && recent_start + length < recent_words.len()
                && keep[start + length]
                && candidate_words[start + length] == recent_words[recent_start + length]
            {
                length += 1;
            }

            if length >= LIVE_DUPLICATE_MIN_TRIM_WORDS {
                let should_replace = best
                    .as_ref()
                    .map(|span: &WordSpan| length > span.end - span.start)
                    .unwrap_or(true);
                if should_replace {
                    best = Some(WordSpan {
                        start,
                        end: start + length,
                    });
                }
            }
        }
    }

    best
}

fn trim_duplicate_suffix(words: &[TranscriptWord], recent_words: &[String]) -> Option<String> {
    if words.len() < LIVE_DUPLICATE_MIN_KEEP_WORDS + LIVE_DUPLICATE_MIN_TRIM_WORDS {
        return None;
    }

    let mut first_valid_split = None;
    for split in LIVE_DUPLICATE_MIN_KEEP_WORDS..=(words.len() - LIVE_DUPLICATE_MIN_TRIM_WORDS) {
        let suffix = words[split..]
            .iter()
            .map(|word| word.normalized.clone())
            .collect::<Vec<_>>();
        if duplicate_coverage(&suffix, recent_words) >= LIVE_DUPLICATE_COVERAGE_THRESHOLD {
            if sentence_ends_after(&words[split - 1].original) {
                return Some(join_original_words(&words[..split]));
            }
            first_valid_split.get_or_insert(split);
        }
    }

    first_valid_split.map(|split| join_original_words(&words[..split]))
}

fn trim_duplicate_prefix(words: &[TranscriptWord], recent_words: &[String]) -> Option<String> {
    if words.len() < LIVE_DUPLICATE_MIN_KEEP_WORDS + LIVE_DUPLICATE_MIN_TRIM_WORDS {
        return None;
    }

    let mut best_split = None;
    let mut best_sentence_split = None;
    for split in LIVE_DUPLICATE_MIN_TRIM_WORDS..=(words.len() - LIVE_DUPLICATE_MIN_KEEP_WORDS) {
        let prefix = words[..split]
            .iter()
            .map(|word| word.normalized.clone())
            .collect::<Vec<_>>();
        if duplicate_coverage(&prefix, recent_words) >= LIVE_DUPLICATE_COVERAGE_THRESHOLD {
            best_split = Some(split);
            if sentence_ends_after(&words[split - 1].original) {
                best_sentence_split = Some(split);
            }
        }
    }

    best_sentence_split
        .or(best_split)
        .map(|split| join_original_words(&words[split..]))
}

fn duplicate_coverage(candidate_words: &[String], recent_words: &[String]) -> f32 {
    let candidate_ngrams = word_ngrams(candidate_words, LIVE_DUPLICATE_NGRAM_SIZE);
    if candidate_ngrams.is_empty() {
        return 0.0;
    }

    let recent_ngrams = word_ngrams(recent_words, LIVE_DUPLICATE_NGRAM_SIZE);
    let covered = candidate_ngrams
        .iter()
        .filter(|ngram| recent_ngrams.contains(ngram))
        .count();
    covered as f32 / candidate_ngrams.len() as f32
}

fn contains_word_sequence(words: &[String], sequence: &[String]) -> bool {
    if sequence.is_empty() {
        return true;
    }
    if sequence.len() > words.len() {
        return false;
    }

    words
        .windows(sequence.len())
        .any(|window| window == sequence)
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::unique_transcript_text;

    #[test]
    fn repeated_transcript_text_matches_overlapping_rollup() {
        let mut recent = VecDeque::new();
        recent.push_back("This is a Just Notes quality assurance test.".to_string());
        recent.push_back(
            "The blue notebook is beside the silver microphone. Every local transcript should preserve these exactly."
                .to_string(),
        );

        assert_eq!(unique_transcript_text(
            "This is a Just Notes quality assurance test. The blue notebook is beside the silver microphone. Every local transcript should preserve these exact words.",
            &recent,
        ), None);
    }

    #[test]
    fn repeated_transcript_text_allows_new_sentence() {
        let mut recent = VecDeque::new();
        recent.push_back("This is a Just Notes quality assurance test.".to_string());

        assert_eq!(
            unique_transcript_text(
                "Chapter two begins with a calendar reminder and a project checkpoint.",
                &recent,
            ),
            Some(
                "Chapter two begins with a calendar reminder and a project checkpoint.".to_string()
            )
        );
    }

    #[test]
    fn unique_transcript_text_trims_repeated_suffix() {
        let mut recent = VecDeque::new();
        recent.push_back(
            "Section 4 says the design notes mention a purple marker and a glass keyboard."
                .to_string(),
        );
        recent.push_back(
            "Section 5 says the engineering plan includes local storage. Audio can also include an"
                .to_string(),
        );

        assert_eq!(
            unique_transcript_text(
                "audio capture and live transcription. Section 4 says the design notes mention a purple marker and a glass keyboard. Section 5 says the engineering plan includes local storage, audio capture, and live transcription.",
                &recent,
            ),
            Some("audio capture and live transcription.".to_string()),
        );
    }

    #[test]
    fn unique_transcript_text_removes_repeated_middle_span() {
        let mut recent = VecDeque::new();
        recent.push_back(
            "Section 18 says the test is half-way through and the steady voice should continue."
                .to_string(),
        );

        assert_eq!(
            unique_transcript_text(
                "the local application and long-running stability. Section 18 says the test is half-way through and the steady voice should continue. Section 19 says the local app must not require cloud services for the transcript.",
                &recent,
            ),
            Some(
                "the local application and long-running stability. Section 19 says the local app must not require cloud services for the transcript."
                    .to_string()
            ),
        );
    }

    #[test]
    fn unique_transcript_text_trims_longest_repeated_prefix() {
        let mut recent = VecDeque::new();
        recent.push_back(
            "Section 1 says the green calendar moved beside the copper lamp.".to_string(),
        );

        assert_eq!(
            unique_transcript_text(
                "Section 1 says the green calendar moved beside the copper lamp. Section 2 says the yellow folder stayed under the quiet monitor.",
                &recent,
            ),
            Some(
                "Section 2 says the yellow folder stayed under the quiet monitor.".to_string()
            ),
        );
    }

    #[test]
    fn unique_transcript_text_removes_internal_repeated_sentence() {
        let recent = VecDeque::new();

        assert_eq!(
            unique_transcript_text(
                "Section 21 says the navy notebook contains project tasks. Section 21 says the navy notebook contains project tasks.",
                &recent,
            ),
            Some("Section 21 says the navy notebook contains project tasks.".to_string()),
        );
    }

    #[test]
    fn unique_transcript_text_drops_short_duplicate_fragments() {
        let mut recent = VecDeque::new();
        recent.push_back(
            "Section 7 says the transcript should advance steadily without repeating earlier phrases."
                .to_string(),
        );

        assert_eq!(unique_transcript_text("phrases.", &recent), None);
    }

    #[test]
    fn unique_transcript_text_removes_numeric_sentence_artifacts() {
        let recent = VecDeque::new();

        assert_eq!(
            unique_transcript_text(
                "3. Section 4 says the design notes mention a purple marker and a glass keyboard. 4.",
                &recent,
            ),
            Some(
                "Section 4 says the design notes mention a purple marker and a glass keyboard."
                    .to_string()
            ),
        );
    }
}
