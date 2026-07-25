//! Turns a recognizer result's parallel token/timestamp/duration arrays into
//! one timed token per word, the unit the segment grouping works from.

use sherpa_onnx::OfflineRecognizerResult;

const MIN_TOKEN_DURATION_MS: u64 = 20;

pub(super) struct TimedToken {
    pub(super) text: String,
    pub(super) start_ms: u64,
    pub(super) end_ms: u64,
}

pub(super) fn parakeet_timed_tokens(
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
