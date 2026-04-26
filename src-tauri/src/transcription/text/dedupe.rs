mod spans;
#[cfg(test)]
mod tests;
mod trimming;

use std::collections::VecDeque;

use super::words::{
    normalized_words, remove_internal_repeated_sentences, transcript_words, word_ngrams,
};
use trimming::{remove_duplicate_word_spans, trim_duplicate_prefix, trim_duplicate_suffix};

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
