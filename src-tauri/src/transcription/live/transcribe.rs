use crate::threads::TranscriptSegment;
use crate::transcription::{is_probable_filler_text, resample_to_rate, rms, samples_to_ms};

use super::{samples_for_ms, Transcriber, Utterance, FRAME_MS, QUIET_CONFIRMATION_RMS};

/// Transcribes one utterance and shifts its segment times to the recording
/// timeline using the utterance's absolute sample offset plus `offset_ms`.
pub(crate) fn transcribe_live_utterance(
    transcriber: &dyn Transcriber,
    utterance: &Utterance,
    source: &str,
    speaker: &str,
    offset_ms: u64,
) -> Result<Vec<TranscriptSegment>, String> {
    let samples_16k = resample_to_rate(&utterance.samples, utterance.sample_rate, 16_000);
    let start_ms = samples_to_ms(utterance.start_index, utterance.sample_rate) + offset_ms;
    let mut segments = transcriber.transcribe_segments(&samples_16k, "", source, speaker)?;
    if utterance_is_faint(utterance) {
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
