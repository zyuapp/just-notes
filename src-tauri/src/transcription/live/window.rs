use std::sync::{Arc, Mutex};

use super::{
    channel::LiveChannelState, LIVE_SILENCE_RMS_THRESHOLD, LIVE_TRANSCRIPTION_CATCH_UP_LAG_MS,
    LIVE_TRANSCRIPTION_MAX_AGREEMENT_BUFFER_MS, LIVE_TRANSCRIPTION_SPEECH_PAD_MS,
    LIVE_TRANSCRIPTION_STABILITY_DELAY_MS, LIVE_TRANSCRIPTION_STEP_MS,
};
use crate::{
    capture::SharedBuffers,
    transcription::{audible_sample_span, ms_to_samples, samples_to_ms},
};

type LiveSampleWindow = Option<Vec<f32>>;

pub(super) struct LiveDecodeWindow {
    pub(super) target_end_ms: u64,
    pub(super) commit_until_ms: u64,
    pub(super) window_start_ms: u64,
    pub(super) samples: Vec<f32>,
    pub(super) audible_start_ms: Option<u64>,
}

struct TrimmedLiveSamples {
    window_start_ms: u64,
    samples: Vec<f32>,
    audible_start_ms: u64,
}

pub(super) fn prepare_live_decode_window(
    buffers: &Arc<Mutex<SharedBuffers>>,
    state: &mut LiveChannelState,
    final_flush: bool,
) -> Result<Option<LiveDecodeWindow>, String> {
    let available_samples = live_available_samples(buffers, state.source)?;
    let available_ms = samples_to_ms(available_samples, state.sample_rate);
    if available_ms <= state.committed_until_ms {
        return Ok(None);
    }

    let Some(target_end_ms) = live_target_end_ms(available_ms, state, final_flush) else {
        return Ok(None);
    };
    let commit_until_ms = live_commit_until_ms(target_end_ms, final_flush);
    if commit_until_ms <= state.committed_until_ms {
        state.next_decode_ms += LIVE_TRANSCRIPTION_STEP_MS;
        return Ok(None);
    }

    let max_window_start_ms =
        target_end_ms.saturating_sub(LIVE_TRANSCRIPTION_MAX_AGREEMENT_BUFFER_MS);
    if state.decode_start_ms < max_window_start_ms {
        state.decode_start_ms = max_window_start_ms;
        state.previous_hypothesis = None;
    }
    let window_start_ms = state.decode_start_ms;
    let start_index = ms_to_samples(window_start_ms, state.sample_rate) as u64;
    let end_index = ms_to_samples(target_end_ms, state.sample_rate) as u64;

    let Some(samples) = live_samples(buffers, state.source, start_index, end_index)? else {
        advance_live_decode(state, commit_until_ms, target_end_ms);
        return Ok(None);
    };

    let Some(trimmed) =
        trim_live_samples_to_audible_span(samples, state.sample_rate, window_start_ms)
    else {
        advance_live_decode(state, commit_until_ms, target_end_ms);
        return Ok(None);
    };

    Ok(Some(LiveDecodeWindow {
        target_end_ms,
        commit_until_ms,
        window_start_ms: trimmed.window_start_ms,
        samples: trimmed.samples,
        audible_start_ms: Some(trimmed.audible_start_ms),
    }))
}

pub(super) fn advance_live_decode(
    state: &mut LiveChannelState,
    commit_until_ms: u64,
    target_end_ms: u64,
) {
    state.committed_until_ms = commit_until_ms;
    state.next_decode_ms = target_end_ms + LIVE_TRANSCRIPTION_STEP_MS;
}

pub(super) fn live_channel_has_pending_decode(
    buffers: &Arc<Mutex<SharedBuffers>>,
    state: &LiveChannelState,
) -> Result<bool, String> {
    let available_samples = live_available_samples(buffers, state.source)?;
    let available_ms = samples_to_ms(available_samples, state.sample_rate);
    Ok(live_channel_has_pending_decode_at(available_ms, state))
}

fn live_target_end_ms(
    available_ms: u64,
    state: &LiveChannelState,
    final_flush: bool,
) -> Option<u64> {
    let target_end_ms = if final_flush {
        available_ms
    } else if available_ms >= state.next_decode_ms {
        catch_up_target_end_ms(available_ms, state)
    } else {
        return None;
    };
    Some(target_end_ms)
}

fn catch_up_target_end_ms(available_ms: u64, state: &LiveChannelState) -> u64 {
    let lag_ms = available_ms.saturating_sub(state.next_decode_ms);
    if lag_ms > LIVE_TRANSCRIPTION_CATCH_UP_LAG_MS {
        let catch_up_target_ms = available_ms
            .saturating_sub(LIVE_TRANSCRIPTION_STEP_MS)
            .max(state.next_decode_ms);
        catch_up_target_ms.min(max_catch_up_target_ms(state))
    } else {
        state.next_decode_ms
    }
}

fn max_catch_up_target_ms(state: &LiveChannelState) -> u64 {
    let max_unskipped_target_ms =
        state.decode_start_ms + LIVE_TRANSCRIPTION_MAX_AGREEMENT_BUFFER_MS;
    let max_target_with_confirmation_room =
        max_unskipped_target_ms.saturating_sub(LIVE_TRANSCRIPTION_STEP_MS);

    max_target_with_confirmation_room.max(state.next_decode_ms)
}

fn live_channel_has_pending_decode_at(available_ms: u64, state: &LiveChannelState) -> bool {
    available_ms > state.committed_until_ms && available_ms >= state.next_decode_ms
}

fn trim_live_samples_to_audible_span(
    samples: Vec<f32>,
    sample_rate: u32,
    window_start_ms: u64,
) -> Option<TrimmedLiveSamples> {
    let span = audible_sample_span(
        &samples,
        sample_rate,
        LIVE_SILENCE_RMS_THRESHOLD,
        LIVE_TRANSCRIPTION_SPEECH_PAD_MS,
    )?;
    let decode_offset_ms = samples_to_ms(span.decode_start_index as u64, sample_rate);
    let audible_start_ms =
        window_start_ms + samples_to_ms(span.audible_start_index as u64, sample_rate);

    Some(TrimmedLiveSamples {
        window_start_ms: window_start_ms + decode_offset_ms,
        samples: samples[span.decode_start_index..span.decode_end_index].to_vec(),
        audible_start_ms,
    })
}

fn live_commit_until_ms(target_end_ms: u64, final_flush: bool) -> u64 {
    if final_flush {
        target_end_ms
    } else {
        target_end_ms.saturating_sub(LIVE_TRANSCRIPTION_STABILITY_DELAY_MS)
    }
}

fn live_available_samples(
    buffers: &Arc<Mutex<SharedBuffers>>,
    source: &str,
) -> Result<u64, String> {
    let shared = buffers
        .lock()
        .map_err(|_| "Audio buffer lock was poisoned".to_string())?;
    let available_samples = match source {
        "mic" => shared.mic.available_end_index(),
        "system" => shared.system.available_end_index(),
        _ => 0,
    };
    Ok(available_samples)
}

fn live_samples(
    buffers: &Arc<Mutex<SharedBuffers>>,
    source: &str,
    start_index: u64,
    end_index: u64,
) -> Result<LiveSampleWindow, String> {
    let shared = buffers
        .lock()
        .map_err(|_| "Audio buffer lock was poisoned".to_string())?;
    let samples = match source {
        "mic" => shared.mic.window(start_index, end_index),
        "system" => shared.system.window(start_index, end_index),
        _ => None,
    };
    Ok(samples)
}

#[cfg(test)]
mod tests {
    use super::{
        catch_up_target_end_ms, live_channel_has_pending_decode_at, live_target_end_ms,
        trim_live_samples_to_audible_span,
    };
    use crate::transcription::live::channel::LiveChannelState;

    #[test]
    fn live_target_steps_normally_when_caught_up() {
        let state = LiveChannelState::new("system", "Others", 16_000);

        assert_eq!(live_target_end_ms(2_500, &state, false), Some(2_000));
    }

    #[test]
    fn live_target_jumps_forward_when_decode_lags_far_behind() {
        let state = LiveChannelState::new("system", "Others", 16_000);

        assert_eq!(live_target_end_ms(20_000, &state, false), Some(18_000));
        assert_eq!(catch_up_target_end_ms(20_000, &state), 18_000);
    }

    #[test]
    fn live_target_caps_catchup_to_unprocessed_window_capacity() {
        let mut state = LiveChannelState::new("system", "Others", 16_000);

        assert_eq!(live_target_end_ms(120_000, &state, false), Some(28_000));

        state.next_decode_ms = 30_000;
        assert_eq!(live_target_end_ms(120_000, &state, false), Some(30_000));
    }

    #[test]
    fn pending_decode_tracks_whether_channel_can_continue_without_sleep() {
        let mut state = LiveChannelState::new("system", "Others", 16_000);

        assert!(!live_channel_has_pending_decode_at(1_999, &state));
        assert!(live_channel_has_pending_decode_at(2_000, &state));

        state.committed_until_ms = 2_000;
        state.next_decode_ms = 4_000;
        assert!(!live_channel_has_pending_decode_at(2_000, &state));
    }

    #[test]
    fn trim_live_samples_preserves_audible_timestamp_after_padding() {
        let mut samples = vec![0.0; 16_000 * 2];
        samples.extend(vec![0.04; 16_000]);
        samples.extend(vec![0.0; 16_000 * 2]);

        let trimmed = trim_live_samples_to_audible_span(samples, 16_000, 10_000).unwrap();

        assert_eq!(trimmed.window_start_ms, 11_750);
        assert_eq!(trimmed.audible_start_ms, 12_000);
        assert_eq!(trimmed.samples.len(), 24_000);
    }
}
