use sherpa_onnx::OfflineRecognizerResult;

use crate::{
    threads::TranscriptSegment,
    transcription::{clean_transcript_text, is_ignored_transcript_text},
};

mod tokens;

use tokens::{parakeet_timed_tokens, TimedToken};

/// Longest span one segment may cover. The transcript is a single list ordered
/// by segment start, so a segment places every word it holds ahead of whatever
/// the other capture channel said meanwhile; this bounds that displacement.
const PARAKEET_MAX_SEGMENT_MS: u64 = 4_000;
const PARAKEET_MIN_SENTENCE_MS: u64 = 500;
const MIN_SEGMENT_DURATION_MS: u64 = 20;

pub(super) struct SegmentIdentity<'a> {
    pub(super) source: &'a str,
    pub(super) speaker: &'a str,
}

pub(super) fn parakeet_result_segments(
    result: &OfflineRecognizerResult,
    total_duration_ms: u64,
    identity: SegmentIdentity<'_>,
) -> Vec<TranscriptSegment> {
    let text = clean_transcript_text(&result.text);
    if text.is_empty() || is_ignored_transcript_text(&text) {
        return Vec::new();
    }

    let tokens = parakeet_timed_tokens(result, total_duration_ms);
    if tokens.is_empty() {
        return split_text_evenly(&text, total_duration_ms, identity);
    }
    grouped_parakeet_segments(tokens, identity)
}

fn grouped_parakeet_segments(
    tokens: Vec<TimedToken>,
    identity: SegmentIdentity<'_>,
) -> Vec<TranscriptSegment> {
    let mut segments = Vec::new();
    let mut sentence: Vec<TimedToken> = Vec::new();
    let mut sentence_end_ms = 0;

    for token in tokens {
        let closes_sentence = ends_sentence(&token.text);
        let start_ms = sentence
            .first()
            .map_or(token.start_ms, |first: &TimedToken| first.start_ms);
        sentence_end_ms = sentence_end_ms.max(token.end_ms);
        sentence.push(token);
        if closes_sentence && sentence_end_ms.saturating_sub(start_ms) >= PARAKEET_MIN_SENTENCE_MS {
            push_bounded_segments(&mut segments, std::mem::take(&mut sentence), &identity);
            sentence_end_ms = 0;
        }
    }

    push_bounded_segments(&mut segments, sentence, &identity);
    segments
}

/// Emits a sentence as one segment, or as several cut at its longest pauses
/// when it runs past [`PARAKEET_MAX_SEGMENT_MS`].
fn push_bounded_segments(
    segments: &mut Vec<TranscriptSegment>,
    sentence: Vec<TimedToken>,
    identity: &SegmentIdentity<'_>,
) {
    let mut rest = sentence;
    while !rest.is_empty() {
        let tail = rest.split_off(cap_split_index(&rest));
        push_parakeet_segment(segments, &rest, identity);
        rest = tail;
    }
}

/// How many leading tokens stay inside the duration cap. Candidates are ranked
/// by whether they start a word and then by the pause before them, so a cut
/// prefers a word boundary; a run with no word start inside the cap is still
/// cut, mid-word. Returns every token when the run already fits, and never
/// returns zero for a non-empty run.
fn cap_split_index(tokens: &[TimedToken]) -> usize {
    let Some(first) = tokens.first() else {
        return 0;
    };
    if group_end_ms(tokens).saturating_sub(first.start_ms) <= PARAKEET_MAX_SEGMENT_MS {
        return tokens.len();
    }

    let mut cut = 1;
    let mut best = (false, 0);
    let mut cut_end_ms = first.end_ms;
    for index in 1..tokens.len() {
        if cut_end_ms.saturating_sub(first.start_ms) > PARAKEET_MAX_SEGMENT_MS {
            break;
        }
        let candidate = (
            starts_word(&tokens[index].text),
            tokens[index]
                .start_ms
                .saturating_sub(tokens[index - 1].end_ms),
        );
        if candidate >= best {
            best = candidate;
            cut = index;
        }
        cut_end_ms = cut_end_ms.max(tokens[index].end_ms);
    }
    cut
}

/// Parakeet emits sub-word tokens; only a word's first token keeps the space
/// that `\u{2581}` was decoded into.
fn starts_word(text: &str) -> bool {
    text.starts_with(' ')
}

fn group_end_ms(tokens: &[TimedToken]) -> u64 {
    tokens.iter().map(|token| token.end_ms).max().unwrap_or(0)
}

fn push_parakeet_segment(
    segments: &mut Vec<TranscriptSegment>,
    tokens: &[TimedToken],
    identity: &SegmentIdentity<'_>,
) {
    let Some(first) = tokens.first() else {
        return;
    };
    let text = clean_segment_text(
        &tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<String>(),
    );
    if text.is_empty() || is_ignored_transcript_text(&text) {
        return;
    }
    let start_ms = first.start_ms;
    segments.push(TranscriptSegment {
        speaker: identity.speaker.to_string(),
        source: identity.source.to_string(),
        start_ms,
        end_ms: group_end_ms(tokens).max(start_ms.saturating_add(MIN_SEGMENT_DURATION_MS)),
        text,
    });
}

fn split_text_evenly(
    text: &str,
    total_duration_ms: u64,
    identity: SegmentIdentity<'_>,
) -> Vec<TranscriptSegment> {
    let words = text.split_whitespace().collect::<Vec<_>>();
    if words.is_empty() {
        return Vec::new();
    }
    let segment_count = total_duration_ms
        .div_ceil(PARAKEET_MAX_SEGMENT_MS)
        .max(1)
        .min(words.len() as u64) as usize;
    let words_per_segment = words.len().div_ceil(segment_count);
    words
        .chunks(words_per_segment)
        .enumerate()
        .filter_map(|(index, chunk)| {
            let start_ms = total_duration_ms.saturating_mul(index as u64) / segment_count as u64;
            let end_ms =
                total_duration_ms.saturating_mul((index + 1) as u64) / segment_count as u64;
            let text = clean_segment_text(&chunk.join(" "));
            (!text.is_empty()).then(|| TranscriptSegment {
                speaker: identity.speaker.to_string(),
                source: identity.source.to_string(),
                start_ms,
                end_ms: end_ms.max(start_ms.saturating_add(MIN_SEGMENT_DURATION_MS)),
                text,
            })
        })
        .collect()
}

fn clean_segment_text(text: &str) -> String {
    let mut cleaned = clean_transcript_text(&text.split_whitespace().collect::<Vec<_>>().join(" "));
    for punctuation in [".", ",", "?", "!", ":", ";"] {
        cleaned = cleaned.replace(&format!(" {punctuation}"), punctuation);
    }
    cleaned
}

fn ends_sentence(text: &str) -> bool {
    matches!(
        text.trim_end().chars().last(),
        Some('.') | Some('?') | Some('!')
    )
}

#[cfg(test)]
mod tests;
