use crate::threads::TranscriptSegment;
use crate::transcription::{is_probable_filler_text, resample_to_rate, rms, samples_to_ms};

use super::{samples_for_ms, Transcriber, Utterance, FRAME_MS, QUIET_CONFIRMATION_RMS};

/// Separation from the nearest preceding speech beyond which a faint
/// filler-only decode counts as a hallucination. Real backchannels land
/// mid-conversation; hallucinated fillers cluster in long silence.
const FILLER_ISOLATION_MS: u64 = 10_000;

/// The transcript channel an utterance belongs to and how it is attributed.
#[derive(Clone, Copy)]
pub(crate) struct ChannelRole<'a> {
    pub(crate) source: &'a str,
    pub(crate) speaker: &'a str,
}

/// Transcribes one utterance and shifts its segment times to the recording
/// timeline using the utterance's absolute sample offset plus `offset_ms`.
/// `last_speech_end_ms` is the end of the most recent segment already
/// committed on any channel (recording timeline), used to judge isolation.
pub(crate) fn transcribe_live_utterance(
    transcriber: &dyn Transcriber,
    utterance: &Utterance,
    role: ChannelRole,
    offset_ms: u64,
    last_speech_end_ms: Option<u64>,
) -> Result<Vec<TranscriptSegment>, String> {
    let samples_16k = resample_to_rate(&utterance.samples, utterance.sample_rate, 16_000);
    let start_ms = samples_to_ms(utterance.start_index, utterance.sample_rate) + offset_ms;
    let mut segments =
        transcriber.transcribe_segments(&samples_16k, "", role.source, role.speaker)?;
    if utterance_is_faint(utterance) && is_isolated(start_ms, last_speech_end_ms) {
        segments.retain(|segment| !is_probable_filler_text(&segment.text));
    }
    for segment in &mut segments {
        segment.start_ms += start_ms;
        segment.end_ms += start_ms;
    }
    Ok(segments)
}

/// True when no 20 ms frame of the utterance reaches the confirmation level.
fn utterance_is_faint(utterance: &Utterance) -> bool {
    let frame_len = samples_for_ms(utterance.sample_rate, FRAME_MS).max(1);
    utterance
        .samples
        .chunks(frame_len)
        .all(|frame| rms(frame) < QUIET_CONFIRMATION_RMS)
}

fn is_isolated(utterance_start_ms: u64, last_speech_end_ms: Option<u64>) -> bool {
    last_speech_end_ms.map_or(true, |end_ms| {
        utterance_start_ms.saturating_sub(end_ms) > FILLER_ISOLATION_MS
    })
}

#[cfg(test)]
mod tests {
    use super::{transcribe_live_utterance, ChannelRole, Transcriber, Utterance};
    use crate::threads::TranscriptSegment;

    struct FixedTranscriber(&'static str);

    impl Transcriber for FixedTranscriber {
        fn transcribe_segments(
            &self,
            _samples_16k: &[f32],
            _prompt: &str,
            source: &str,
            speaker: &str,
        ) -> Result<Vec<TranscriptSegment>, String> {
            Ok(vec![TranscriptSegment {
                speaker: speaker.to_string(),
                source: source.to_string(),
                start_ms: 0,
                end_ms: 400,
                text: self.0.to_string(),
            }])
        }
    }

    fn utterance(amplitude: f32) -> Utterance {
        Utterance {
            start_index: 16_000 * 60, // one minute in
            sample_rate: 16_000,
            samples: vec![amplitude; 8_000],
        }
    }

    fn transcribe(text: &'static str, amplitude: f32, last_end: Option<u64>) -> usize {
        let role = ChannelRole {
            source: "mic",
            speaker: "You",
        };
        transcribe_live_utterance(
            &FixedTranscriber(text),
            &utterance(amplitude),
            role,
            0,
            last_end,
        )
        .expect("transcribe")
        .len()
    }

    #[test]
    fn isolated_faint_filler_is_rejected() {
        assert_eq!(transcribe("Okay.", 0.01, None), 0);
        assert_eq!(transcribe("Okay.", 0.01, Some(10_000)), 0);
    }

    #[test]
    fn faint_filler_near_other_speech_is_kept() {
        assert_eq!(transcribe("Okay.", 0.01, Some(55_000)), 1);
    }

    #[test]
    fn loud_or_substantive_speech_is_never_rejected() {
        assert_eq!(transcribe("Okay.", 0.1, None), 1);
        assert_eq!(transcribe("Let me check the logs.", 0.01, None), 1);
    }
}
