use super::{
    duplicate_coverage, spans::longest_common_word_span, LIVE_DUPLICATE_COVERAGE_THRESHOLD,
    LIVE_DUPLICATE_MIN_KEEP_WORDS, LIVE_DUPLICATE_MIN_TRIM_WORDS,
};
use crate::transcription::text::words::{
    join_original_word_refs, join_original_words, sentence_ends_after, TranscriptWord,
};

pub(super) fn remove_duplicate_word_spans(
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
        let Some(span) = longest_common_word_span(
            &normalized,
            recent_words,
            &keep,
            LIVE_DUPLICATE_MIN_TRIM_WORDS,
        ) else {
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

pub(super) fn trim_duplicate_suffix(
    words: &[TranscriptWord],
    recent_words: &[String],
) -> Option<String> {
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

pub(super) fn trim_duplicate_prefix(
    words: &[TranscriptWord],
    recent_words: &[String],
) -> Option<String> {
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
