use std::path::Path;

use crate::threads::TranscriptSegment;

pub(super) mod profile;
#[cfg(test)]
mod tests;

use profile::{max_envelope_correlation, ChannelProfile};

const BLEED_TIME_PAD_MS: u64 = 2_000;
const SYSTEM_DOMINATED_MIN_SYSTEM_RMS: f32 = 0.005;
const SYSTEM_DOMINATED_MIC_MAX_RATIO: f32 = 0.65;
const EXTREME_SYSTEM_DOMINANCE_MIC_MAX_RATIO: f32 = 0.08;
const EXTREME_SYSTEM_SPEECH_MIN_COVERAGE_PERCENT: u64 = 80;
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
    audio_offset_ms: u64,
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
        audio_offset_ms,
    ))
}

fn suppress_system_dominated_mic_segments_with_profiles(
    segments: Vec<TranscriptSegment>,
    mic_profile: &ChannelProfile,
    system_profile: &ChannelProfile,
    audio_offset_ms: u64,
) -> Vec<TranscriptSegment> {
    let system_spans = segments
        .iter()
        .filter(|segment| segment.source == "system" && segment.start_ms >= audio_offset_ms)
        .map(|segment| (segment.start_ms, segment.end_ms))
        .collect::<Vec<_>>();

    segments
        .into_iter()
        .filter(|segment| {
            segment.source != "mic"
                || segment.start_ms < audio_offset_ms
                || !overlaps_any_system_segment(segment, &system_spans)
                || !mic_segment_is_system_bleed(
                    segment,
                    &system_spans,
                    mic_profile,
                    system_profile,
                    audio_offset_ms,
                )
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
    system_spans: &[(u64, u64)],
    mic_profile: &ChannelProfile,
    system_profile: &ChannelProfile,
    audio_offset_ms: u64,
) -> bool {
    let local_start_ms = segment.start_ms - audio_offset_ms;
    let local_end_ms = segment.end_ms.saturating_sub(audio_offset_ms);
    let mic_rms = mic_profile.rms(local_start_ms, local_end_ms);
    let system_rms = system_profile.rms(local_start_ms, local_end_ms);
    if !mic_audio_is_system_dominated(mic_rms, system_rms) {
        return false;
    }

    // The bleed-only recording that motivated this fallback had mic/system
    // RMS ratios of 0.061-0.069, while the quiet-speech regression fixtures
    // bottom out at 0.16. Keep the cutoff inside that gap and require decoded
    // system speech across most of the mic span, rather than treating the
    // two-second proximity allowance above as evidence. At this degree of
    // dominance, a low envelope correlation is expected when the microphone
    // captures only a faint, distorted residue of the system channel.
    if mic_rms < system_rms * EXTREME_SYSTEM_DOMINANCE_MIC_MAX_RATIO
        && system_speech_coverage_percent(segment, system_spans)
            >= EXTREME_SYSTEM_SPEECH_MIN_COVERAGE_PERCENT
    {
        return true;
    }

    // A quieter mic segment is only bleed when its energy envelope tracks the
    // overlapping system audio: echo is a delayed copy of that signal, whereas
    // genuinely quiet speech is uncorrelated with it. Short segments (a
    // hallucinated "mm-hmm" runs a few hundred ms) carry too few envelope
    // frames to judge on their own span, so the correlation window is widened
    // around them to keep the evidence requirement met.
    let (start_ms, end_ms) = correlation_window(local_start_ms, local_end_ms, ECHO_MIN_WINDOW_MS);
    let correlation = max_envelope_correlation(
        mic_profile,
        system_profile,
        start_ms,
        end_ms,
        ECHO_MAX_LAG_MS,
    );
    correlation >= ECHO_CORRELATION_THRESHOLD
}

fn system_speech_coverage_percent(segment: &TranscriptSegment, system_spans: &[(u64, u64)]) -> u64 {
    let duration = segment.end_ms.saturating_sub(segment.start_ms);
    if duration == 0 {
        return 0;
    }

    let mut overlaps = system_spans
        .iter()
        .filter_map(|(start_ms, end_ms)| {
            let start_ms = segment.start_ms.max(*start_ms);
            let end_ms = segment.end_ms.min(*end_ms);
            (start_ms < end_ms).then_some((start_ms, end_ms))
        })
        .collect::<Vec<_>>();
    overlaps.sort_unstable();

    let mut covered_ms = 0u64;
    let mut merged: Option<(u64, u64)> = None;
    for (start_ms, end_ms) in overlaps {
        match merged {
            Some((merged_start, merged_end)) if start_ms <= merged_end => {
                merged = Some((merged_start, merged_end.max(end_ms)));
            }
            Some((merged_start, merged_end)) => {
                covered_ms = covered_ms.saturating_add(merged_end - merged_start);
                merged = Some((start_ms, end_ms));
            }
            None => merged = Some((start_ms, end_ms)),
        }
    }
    if let Some((start_ms, end_ms)) = merged {
        covered_ms = covered_ms.saturating_add(end_ms - start_ms);
    }

    ((covered_ms as u128 * 100) / duration as u128) as u64
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
