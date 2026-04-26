use std::sync::{Arc, Mutex};

use super::{
    channel::LiveChannelState, LIVE_SILENCE_RMS_THRESHOLD,
    LIVE_TRANSCRIPTION_MAX_AGREEMENT_BUFFER_MS, LIVE_TRANSCRIPTION_STABILITY_DELAY_MS,
    LIVE_TRANSCRIPTION_STEP_MS,
};
use crate::{
    capture::SharedBuffers,
    transcription::{first_audible_ms, ms_to_samples, rms, samples_to_ms},
};

type LiveSampleWindow = Option<Vec<f32>>;

pub(super) struct LiveDecodeWindow {
    pub(super) target_end_ms: u64,
    pub(super) commit_until_ms: u64,
    pub(super) window_start_ms: u64,
    pub(super) samples: Vec<f32>,
    pub(super) audible_start_ms: Option<u64>,
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

    if rms(&samples) < LIVE_SILENCE_RMS_THRESHOLD {
        advance_live_decode(state, commit_until_ms, target_end_ms);
        return Ok(None);
    }

    let audible_start_ms =
        first_audible_ms(&samples, state.sample_rate, LIVE_SILENCE_RMS_THRESHOLD)
            .map(|offset_ms| window_start_ms + offset_ms);
    Ok(Some(LiveDecodeWindow {
        target_end_ms,
        commit_until_ms,
        window_start_ms,
        samples,
        audible_start_ms,
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

fn live_target_end_ms(
    available_ms: u64,
    state: &LiveChannelState,
    final_flush: bool,
) -> Option<u64> {
    let target_end_ms = if final_flush {
        available_ms
    } else if available_ms >= state.next_decode_ms {
        state.next_decode_ms
    } else {
        return None;
    };
    Some(target_end_ms)
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
