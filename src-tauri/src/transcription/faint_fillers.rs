//! Removes faint, temporally isolated filler-only segments — the decode
//! signature of ASR hallucinations on non-speech audio. This runs with the
//! other transcript suppressors (finalize and the stop-time polish), where
//! both channels' full timelines are available, so the isolation judgment is
//! symmetric across channels and independent of decode order.

use std::path::Path;

use crate::threads::TranscriptSegment;

use super::live::QUIET_CONFIRMATION_RMS;
use super::source_bleed::{mic_audio_is_system_dominated, profile::ChannelProfile};
use super::text::is_probable_filler_text;

/// Separation from the nearest other segment beyond which a faint filler
/// counts as a hallucination. Real backchannels land mid-conversation;
/// hallucinated fillers cluster in long silence.
const FILLER_ISOLATION_MS: u64 = 10_000;

pub(crate) fn suppress_isolated_faint_fillers(
    segments: Vec<TranscriptSegment>,
    mic_path: &Path,
    system_path: &Path,
) -> Result<Vec<TranscriptSegment>, String> {
    let mic_profile = optional_profile(mic_path)?;
    let system_profile = optional_profile(system_path)?;
    Ok(suppress_with_profiles(
        segments,
        mic_profile.as_ref(),
        system_profile.as_ref(),
    ))
}

fn optional_profile(path: &Path) -> Result<Option<ChannelProfile>, String> {
    if !path.is_file() {
        return Ok(None);
    }
    ChannelProfile::from_wav(path).map(Some)
}

fn suppress_with_profiles(
    segments: Vec<TranscriptSegment>,
    mic_profile: Option<&ChannelProfile>,
    system_profile: Option<&ChannelProfile>,
) -> Vec<TranscriptSegment> {
    let candidates: Vec<bool> = segments
        .iter()
        .map(|segment| {
            is_probable_filler_text(&segment.text) && is_faint(segment, mic_profile, system_profile)
        })
        .collect();
    let dropped: Vec<bool> = segments
        .iter()
        .enumerate()
        .map(|(index, segment)| {
            candidates[index]
                && (is_isolated(index, &segments, &candidates)
                    || is_shadowed_by_system_audio(segment, &segments, mic_profile, system_profile))
        })
        .collect();
    segments
        .into_iter()
        .zip(dropped)
        .filter_map(|(segment, dropped)| (!dropped).then_some(segment))
        .collect()
}

/// Faint means no confident speech level in the segment's own channel audio.
/// Without a readable recording for that channel the segment is kept.
fn is_faint(
    segment: &TranscriptSegment,
    mic_profile: Option<&ChannelProfile>,
    system_profile: Option<&ChannelProfile>,
) -> bool {
    let profile = if segment.source == "mic" {
        mic_profile
    } else {
        system_profile
    };
    profile.is_some_and(|profile| {
        profile.rms(segment.start_ms, segment.end_ms) < QUIET_CONFIRMATION_RMS
    })
}

/// A faint mic filler spoken while system audio dominates the mic is far
/// more likely a decode of breath or bleed than a real backchannel: anything
/// the user actually voices lands well above the confirmation level. Phantom
/// words attributed to "You" cost more than a lost whisper-level "mm-hmm".
fn is_shadowed_by_system_audio(
    segment: &TranscriptSegment,
    segments: &[TranscriptSegment],
    mic_profile: Option<&ChannelProfile>,
    system_profile: Option<&ChannelProfile>,
) -> bool {
    if segment.source != "mic" {
        return false;
    }
    let overlaps_system = segments.iter().any(|other| {
        other.source == "system"
            && other.start_ms < segment.end_ms
            && segment.start_ms < other.end_ms
    });
    if !overlaps_system {
        return false;
    }
    let (Some(mic), Some(system)) = (mic_profile, system_profile) else {
        return false;
    };
    mic_audio_is_system_dominated(
        mic.rms(segment.start_ms, segment.end_ms),
        system.rms(segment.start_ms, segment.end_ms),
    )
}

/// Only substantive or confident speech counts as conversational context;
/// another candidate hallucination cannot shield its neighbor, or clustered
/// fillers ("Okay" then "Mm-hmm" on room noise) would keep each other alive.
fn is_isolated(index: usize, segments: &[TranscriptSegment], candidates: &[bool]) -> bool {
    let segment = &segments[index];
    !segments.iter().enumerate().any(|(other_index, other)| {
        other_index != index
            && !candidates[other_index]
            && other.start_ms <= segment.end_ms.saturating_add(FILLER_ISOLATION_MS)
            && segment.start_ms <= other.end_ms.saturating_add(FILLER_ISOLATION_MS)
    })
}

#[cfg(test)]
mod tests {
    use super::{suppress_with_profiles, ChannelProfile};
    use crate::threads::TranscriptSegment;

    fn segment(source: &str, start_ms: u64, end_ms: u64, text: &str) -> TranscriptSegment {
        TranscriptSegment {
            speaker: if source == "mic" { "You" } else { "Others" }.to_string(),
            source: source.to_string(),
            start_ms,
            end_ms,
            text: text.to_string(),
        }
    }

    /// A mic profile of `frames` 25 ms envelope bins at a constant level.
    fn profile(level: f32, frames: usize) -> ChannelProfile {
        ChannelProfile::from_envelope(&vec![level; frames])
    }

    #[test]
    fn drops_isolated_faint_filler() {
        let mic = profile(0.005, 80);
        let kept =
            suppress_with_profiles(vec![segment("mic", 500, 900, "Okay.")], Some(&mic), None);
        assert!(kept.is_empty());
    }

    #[test]
    fn keeps_faint_filler_near_speech_on_either_channel() {
        let mic = profile(0.005, 800);
        let segments = vec![
            segment("system", 1_000, 4_000, "Let me walk you through the plan."),
            segment("mic", 6_000, 6_400, "Okay."),
        ];
        assert_eq!(suppress_with_profiles(segments, Some(&mic), None).len(), 2);
    }

    #[test]
    fn keeps_loud_fillers_and_substantive_faint_speech() {
        let mic = profile(0.05, 80);
        let loud =
            suppress_with_profiles(vec![segment("mic", 500, 900, "Okay.")], Some(&mic), None);
        assert_eq!(loud.len(), 1);

        let faint = profile(0.005, 80);
        let substantive = suppress_with_profiles(
            vec![segment("mic", 500, 1_500, "Remember to check the logs.")],
            Some(&faint),
            None,
        );
        assert_eq!(substantive.len(), 1);
    }

    #[test]
    fn clustered_faint_fillers_do_not_shield_each_other() {
        let mic = profile(0.005, 800);
        let segments = vec![
            segment("mic", 1_000, 1_400, "Okay."),
            segment("mic", 6_000, 6_400, "Mm-hmm."),
        ];
        assert!(suppress_with_profiles(segments, Some(&mic), None).is_empty());
    }

    // The live-test phantom: a 240 ms "Mm-hmm." right after the user's own
    // speech, overlapping dominant system audio — a breath or bleed decode.
    // Nearby real speech must not shield it.
    #[test]
    fn drops_faint_filler_shadowed_by_dominant_system_audio() {
        let mic = profile(0.005, 800);
        let system = profile(0.06, 800);
        let segments = vec![
            segment("mic", 1_000, 4_000, "What do the best engineers do?"),
            segment(
                "system",
                3_500,
                6_500,
                "This is my favorite skill of all time.",
            ),
            segment("mic", 4_600, 4_840, "Mm-hmm."),
        ];
        let kept = suppress_with_profiles(segments, Some(&mic), Some(&system));
        assert_eq!(kept.len(), 2);
        assert!(kept.iter().all(|segment| segment.text != "Mm-hmm."));
    }

    #[test]
    fn keeps_filler_when_its_channel_audio_is_unreadable() {
        let kept = suppress_with_profiles(vec![segment("mic", 500, 900, "Okay.")], None, None);
        assert_eq!(kept.len(), 1);
    }
}
