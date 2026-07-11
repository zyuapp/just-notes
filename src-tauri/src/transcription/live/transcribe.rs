use crate::threads::TranscriptSegment;
use crate::transcription::{resample_to_rate, samples_to_ms};

use super::{Transcriber, Utterance};

/// The transcript channel an utterance belongs to and how it is attributed.
#[derive(Clone, Copy)]
pub(crate) struct ChannelRole<'a> {
    pub(crate) source: &'a str,
    pub(crate) speaker: &'a str,
}

/// Transcribes one utterance and shifts its segment times to the recording
/// timeline using the utterance's absolute sample offset plus `offset_ms`.
pub(crate) fn transcribe_live_utterance(
    transcriber: &dyn Transcriber,
    utterance: &Utterance,
    role: ChannelRole,
    offset_ms: u64,
) -> Result<Vec<TranscriptSegment>, String> {
    let samples_16k = resample_to_rate(&utterance.samples, utterance.sample_rate, 16_000);
    let start_ms = samples_to_ms(utterance.start_index, utterance.sample_rate) + offset_ms;
    let mut segments =
        transcriber.transcribe_segments(&samples_16k, "", role.source, role.speaker)?;
    for segment in &mut segments {
        segment.start_ms += start_ms;
        segment.end_ms += start_ms;
    }
    Ok(segments)
}
