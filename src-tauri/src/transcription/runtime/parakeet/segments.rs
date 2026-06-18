use sherpa_onnx::OfflineRecognizerResult;

use crate::{
    threads::TranscriptSegment,
    transcription::{clean_transcript_text, is_ignored_transcript_text},
};

const PARAKEET_MAX_SEGMENT_MS: u64 = 30_000;
const PARAKEET_MIN_SENTENCE_MS: u64 = 500;
const MIN_TOKEN_DURATION_MS: u64 = 20;

#[derive(Clone)]
struct TimedToken {
    text: String,
    start_ms: u64,
    end_ms: u64,
}

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

fn parakeet_timed_tokens(
    result: &OfflineRecognizerResult,
    total_duration_ms: u64,
) -> Vec<TimedToken> {
    let Some(timestamps) = result.timestamps.as_ref() else {
        return Vec::new();
    };
    result
        .tokens
        .iter()
        .enumerate()
        .filter_map(|(index, token)| {
            let text = parakeet_token_text(token);
            if text.trim().is_empty() {
                return None;
            }
            let start_ms = seconds_to_ms(*timestamps.get(index)?).min(total_duration_ms);
            let end_ms = parakeet_token_end_ms(index, start_ms, result, total_duration_ms);
            Some(TimedToken {
                text,
                start_ms,
                end_ms,
            })
        })
        .collect()
}

fn parakeet_token_end_ms(
    index: usize,
    start_ms: u64,
    result: &OfflineRecognizerResult,
    total_duration_ms: u64,
) -> u64 {
    let duration_end = result
        .durations
        .as_ref()
        .and_then(|durations| durations.get(index))
        .copied()
        .filter(|duration| duration.is_finite() && *duration > 0.0)
        .map(|duration| start_ms.saturating_add(seconds_to_ms(duration)));
    let next_start = result
        .timestamps
        .as_ref()
        .and_then(|timestamps| timestamps.get(index + 1))
        .copied()
        .map(seconds_to_ms);
    let end_ms = duration_end
        .or(next_start)
        .unwrap_or(total_duration_ms)
        .max(start_ms.saturating_add(MIN_TOKEN_DURATION_MS));
    if total_duration_ms > start_ms {
        end_ms.min(total_duration_ms)
    } else {
        end_ms
    }
}

fn seconds_to_ms(seconds: f32) -> u64 {
    if seconds.is_finite() && seconds > 0.0 {
        (seconds * 1000.0).round() as u64
    } else {
        0
    }
}

fn parakeet_token_text(token: &str) -> String {
    token
        .replace('\u{2581}', " ")
        .replace("<blk>", "")
        .replace("<blank>", "")
}

fn grouped_parakeet_segments(
    tokens: Vec<TimedToken>,
    identity: SegmentIdentity<'_>,
) -> Vec<TranscriptSegment> {
    let mut segments = Vec::new();
    let mut current = Vec::new();
    let mut current_start_ms = 0;
    let mut current_end_ms = 0;

    for token in tokens {
        if current.is_empty() {
            current_start_ms = token.start_ms;
        }
        current_end_ms = current_end_ms.max(token.end_ms);
        let sentence_boundary = ends_sentence(&token.text)
            && current_end_ms.saturating_sub(current_start_ms) >= PARAKEET_MIN_SENTENCE_MS;
        let duration_boundary =
            current_end_ms.saturating_sub(current_start_ms) >= PARAKEET_MAX_SEGMENT_MS;
        current.push(token.text);
        if sentence_boundary || duration_boundary {
            push_parakeet_segment(
                &mut segments,
                &current,
                current_start_ms,
                current_end_ms,
                &identity,
            );
            current.clear();
        }
    }

    if !current.is_empty() {
        push_parakeet_segment(
            &mut segments,
            &current,
            current_start_ms,
            current_end_ms,
            &identity,
        );
    }
    segments
}

fn push_parakeet_segment(
    segments: &mut Vec<TranscriptSegment>,
    pieces: &[String],
    start_ms: u64,
    end_ms: u64,
    identity: &SegmentIdentity<'_>,
) {
    let text = clean_segment_text(&pieces.concat());
    if text.is_empty() || is_ignored_transcript_text(&text) {
        return;
    }
    segments.push(TranscriptSegment {
        speaker: identity.speaker.to_string(),
        source: identity.source.to_string(),
        start_ms,
        end_ms: end_ms.max(start_ms.saturating_add(MIN_TOKEN_DURATION_MS)),
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
                end_ms: end_ms.max(start_ms.saturating_add(MIN_TOKEN_DURATION_MS)),
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
