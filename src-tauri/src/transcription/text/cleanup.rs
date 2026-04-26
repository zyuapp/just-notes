use crate::threads::TranscriptSegment;

use super::words::{
    join_original_words, normalized_words, split_transcript_sentences, transcript_words,
};

pub(crate) fn is_ignored_transcript_text(text: &str) -> bool {
    matches!(
        text.trim().to_ascii_lowercase().as_str(),
        "[blank_audio]" | "[silence]" | "(silence)" | "[music]" | "(music)"
    )
}

pub(crate) fn clean_transcript_text(text: &str) -> String {
    let mut cleaned = text.trim();
    loop {
        let lower = cleaned.to_ascii_lowercase();
        let Some(prefix) = [
            "[blank_audio]",
            "[silence]",
            "(silence)",
            "[music]",
            "(music)",
            "(no audio)",
        ]
        .iter()
        .find(|prefix| lower.starts_with(**prefix)) else {
            break;
        };
        cleaned = cleaned[prefix.len()..].trim();
    }
    cleaned.to_string()
}

pub(crate) fn completed_transcript_text(text: &str, include_partial: bool) -> String {
    let text = text.trim();
    if include_partial || transcript_text_is_complete(text) {
        return text.to_string();
    }

    split_transcript_sentences(text)
        .into_iter()
        .filter(|sentence| transcript_text_is_complete(sentence))
        .collect::<Vec<_>>()
        .join(" ")
}

fn transcript_text_is_complete(text: &str) -> bool {
    text.trim()
        .chars()
        .next_back()
        .map(|character| matches!(character, '.' | '!' | '?'))
        .unwrap_or(false)
}

pub(crate) fn common_transcript_prefix(left: &str, right: &str) -> Option<String> {
    let left_words = normalized_words(left);
    let right_words = transcript_words(right);
    let prefix_len = left_words
        .iter()
        .zip(right_words.iter())
        .take_while(|(left, right)| left.as_str() == right.normalized.as_str())
        .count();

    (prefix_len > 0).then(|| join_original_words(&right_words[..prefix_len]))
}

pub(crate) fn estimate_text_end_ms(
    segments: &[TranscriptSegment],
    text: &str,
    window_start_ms: u64,
    fallback_end_ms: u64,
) -> u64 {
    let target_words = normalized_words(text).len();
    if target_words == 0 {
        return window_start_ms;
    }

    let mut seen_words = 0usize;
    for segment in segments {
        let segment_words = normalized_words(&segment.text).len();
        if segment_words == 0 {
            continue;
        }

        let next_seen_words = seen_words + segment_words;
        let segment_start_ms = window_start_ms + segment.start_ms;
        let segment_end_ms = window_start_ms + segment.end_ms;
        if target_words <= next_seen_words {
            let words_in_segment = target_words.saturating_sub(seen_words).max(1);
            let ratio = (words_in_segment as f64 / segment_words as f64).clamp(0.0, 1.0);
            let duration_ms = segment_end_ms.saturating_sub(segment_start_ms) as f64;
            let estimated_ms = (segment_start_ms as f64 + duration_ms * ratio)
                .round()
                .max(segment_start_ms as f64) as u64;
            return estimated_ms.min(fallback_end_ms);
        }

        seen_words = next_seen_words;
    }

    fallback_end_ms
}
