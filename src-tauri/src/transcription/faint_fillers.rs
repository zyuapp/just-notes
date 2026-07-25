//! Rejects filler-only ASR decodes that lack convincing speech energy.
//!
//! The segmenter deliberately uses a low gate so quiet substantive speech can
//! reach the recognizer. Local ASR models can turn the same low-level room
//! noise, hum, or breath into short backchannels such as "Okay" or "Mm-hmm".
//! Those decodes need stronger acoustic evidence than ordinary transcript
//! text, regardless of whether they occur near a real conversational turn.

use std::path::Path;

use crate::threads::TranscriptSegment;

use super::text::is_probable_filler_text;

mod audio;
use audio::SpeechProfile;
pub(crate) use audio::CONFIDENT_SPEECH_RMS;

/// Filters a freshly decoded mic utterance before it is ever shown or stored.
/// Faint substantive text stays on the recall-biased path; only filler-only
/// text needs sustained evidence at the higher confirmation threshold.
pub(crate) fn suppress_unconfirmed_mic_fillers(
    segments: Vec<TranscriptSegment>,
    samples: &[f32],
    sample_rate: u32,
    channel_source: &str,
) -> Vec<TranscriptSegment> {
    if channel_source != "mic" {
        return segments;
    }
    segments
        .into_iter()
        .filter(|segment| {
            !is_probable_filler_text(&segment.text)
                || audio::segment_has_confident_speech(
                    samples,
                    sample_rate,
                    segment.start_ms,
                    segment.end_ms,
                )
        })
        .collect()
}

/// Stop-time defense for mic transcripts produced by an older or
/// bypassed live path. Unlike the former isolation rule, conversational
/// proximity is not evidence that the mic audio contained speech. System
/// fillers are never filtered: quiet remote speech is valid conversation.
///
/// During a resumed recording, transcript timestamps stay on the thread's
/// absolute timeline while `mic_path` contains only the current session. The
/// offset translates current-session timestamps into that WAV's local time;
/// older transcript segments are left untouched.
pub(crate) fn suppress_faint_mic_fillers(
    segments: Vec<TranscriptSegment>,
    mic_path: &Path,
    audio_offset_ms: u64,
) -> Result<Vec<TranscriptSegment>, String> {
    let mic_profile = optional_speech_profile(mic_path)?;
    Ok(suppress_with_profile(
        segments,
        mic_profile.as_ref(),
        audio_offset_ms,
    ))
}

fn optional_speech_profile(path: &Path) -> Result<Option<SpeechProfile>, String> {
    if !path.is_file() {
        return Ok(None);
    }
    SpeechProfile::from_wav(path).map(Some)
}

fn suppress_with_profile(
    segments: Vec<TranscriptSegment>,
    mic_profile: Option<&SpeechProfile>,
    audio_offset_ms: u64,
) -> Vec<TranscriptSegment> {
    segments
        .into_iter()
        .filter(|segment| {
            segment.source != "mic"
                || segment.start_ms < audio_offset_ms
                || !is_probable_filler_text(&segment.text)
                || !is_faint_mic_filler(segment, mic_profile, audio_offset_ms)
        })
        .collect()
}

/// Faint means the current session's mic channel never reaches the
/// conservative confirmation level. Without readable mic audio, keep it.
fn is_faint_mic_filler(
    segment: &TranscriptSegment,
    mic_profile: Option<&SpeechProfile>,
    audio_offset_ms: u64,
) -> bool {
    let local_start_ms = segment.start_ms - audio_offset_ms;
    let local_end_ms = segment.end_ms.saturating_sub(audio_offset_ms);
    mic_profile
        .and_then(|profile| profile.confident_speech_evidence(local_start_ms, local_end_ms))
        .is_some_and(|confirmed| !confirmed)
}

#[cfg(test)]
mod tests;
