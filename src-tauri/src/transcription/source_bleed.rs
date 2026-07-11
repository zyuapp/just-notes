use std::path::Path;

use crate::threads::TranscriptSegment;

pub(super) mod profile;
#[cfg(test)]
mod tests;

use profile::{max_envelope_correlation, ChannelProfile};

const BLEED_TIME_PAD_MS: u64 = 2_000;
const SYSTEM_DOMINATED_MIN_SYSTEM_RMS: f32 = 0.005;
const SYSTEM_DOMINATED_MIC_MAX_RATIO: f32 = 0.65;
const ECHO_CORRELATION_THRESHOLD: f32 = 0.6;
const ECHO_MAX_LAG_MS: u64 = 500;
const ECHO_MIN_WINDOW_MS: u64 = 1_500;

pub(crate) fn mic_audio_is_system_dominated(mic_rms: f32, system_rms: f32) -> bool {
    system_rms >= SYSTEM_DOMINATED_MIN_SYSTEM_RMS
        && mic_rms < system_rms * SYSTEM_DOMINATED_MIC_MAX_RATIO
}

pub(crate) fn suppress_system_dominated_mic_segments(
    segments: Vec<TranscriptSegment>,
    mic_path: &Path,
    system_path: &Path,
) -> Result<Vec<TranscriptSegment>, String> {
    if !mic_path.is_file() || !system_path.is_file() {
        return Ok(segments);
    }

    let mic_profile = ChannelProfile::from_wav(mic_path)?;
    let system_profile = ChannelProfile::from_wav(system_path)?;
    Ok(suppress_system_dominated_mic_segments_with_profiles(
        segments,
        &mic_profile,
        &system_profile,
    ))
}

fn suppress_system_dominated_mic_segments_with_profiles(
    segments: Vec<TranscriptSegment>,
    mic_profile: &ChannelProfile,
    system_profile: &ChannelProfile,
) -> Vec<TranscriptSegment> {
    let system_spans = segments
        .iter()
        .filter(|segment| segment.source == "system")
        .map(|segment| (segment.start_ms, segment.end_ms))
        .collect::<Vec<_>>();

    segments
        .into_iter()
        .filter(|segment| {
            segment.source != "mic"
                || !overlaps_any_system_segment(segment, &system_spans)
                || !mic_segment_is_system_bleed(segment, mic_profile, system_profile)
        })
        .collect()
}

fn overlaps_any_system_segment(segment: &TranscriptSegment, system_spans: &[(u64, u64)]) -> bool {
    system_spans.iter().any(|(start_ms, end_ms)| {
        segment.start_ms <= end_ms.saturating_add(BLEED_TIME_PAD_MS)
            && *start_ms <= segment.end_ms.saturating_add(BLEED_TIME_PAD_MS)
    })
}

fn mic_segment_is_system_bleed(
    segment: &TranscriptSegment,
    mic_profile: &ChannelProfile,
    system_profile: &ChannelProfile,
) -> bool {
    let mic_rms = mic_profile.rms(segment.start_ms, segment.end_ms);
    let system_rms = system_profile.rms(segment.start_ms, segment.end_ms);
    if !mic_audio_is_system_dominated(mic_rms, system_rms) {
        return false;
    }

    // A quieter mic segment is only bleed when its energy envelope tracks the
    // overlapping system audio: echo is a delayed copy of that signal, whereas
    // genuinely quiet speech is uncorrelated with it. Short segments (a
    // hallucinated "mm-hmm" runs a few hundred ms) carry too few envelope
    // frames to judge on their own span, so the correlation window is widened
    // around them to keep the evidence requirement met.
    let (start_ms, end_ms) =
        correlation_window(segment.start_ms, segment.end_ms, ECHO_MIN_WINDOW_MS);
    let correlation = max_envelope_correlation(
        mic_profile,
        system_profile,
        start_ms,
        end_ms,
        ECHO_MAX_LAG_MS,
    );
    correlation >= ECHO_CORRELATION_THRESHOLD
}

/// The segment's span widened symmetrically to at least `min_window_ms`.
fn correlation_window(start_ms: u64, end_ms: u64, min_window_ms: u64) -> (u64, u64) {
    let length = end_ms.saturating_sub(start_ms);
    if length >= min_window_ms {
        return (start_ms, end_ms);
    }
    let pad = (min_window_ms - length) / 2;
    (start_ms.saturating_sub(pad), end_ms.saturating_add(pad))
}
