use crate::threads::TranscriptSegment;

use super::{resample_to_rate, rms, samples_to_ms, Transcriber};

const FRAME_MS: u64 = 20;
const REDEMPTION_MS: u64 = 600;
const MAX_UTTERANCE_MS: u64 = 24_000;

/// Tuning for turning a raw sample stream into speech utterances.
#[derive(Clone, Copy)]
pub(crate) struct SegmenterConfig {
    /// Frame RMS at or above which a [`FRAME_MS`] frame counts as speech.
    pub(crate) speech_rms: f32,
    /// Shortest run worth transcribing; briefer speech islands are dropped.
    pub(crate) min_utterance_ms: u64,
}

impl SegmenterConfig {
    /// Live preview: a higher gate keeps idle silence from spawning work.
    pub(crate) fn live() -> Self {
        Self {
            speech_rms: 0.006,
            min_utterance_ms: 250,
        }
    }

    /// Finalization: a low gate favors recall since this pass is authoritative.
    pub(crate) fn finalize() -> Self {
        Self {
            speech_rms: 0.0008,
            min_utterance_ms: 60,
        }
    }
}

/// A closed run of speech plus the absolute sample index of its first sample.
pub(crate) struct Utterance {
    pub(crate) start_index: u64,
    pub(crate) sample_rate: u32,
    pub(crate) samples: Vec<f32>,
}

struct ActiveUtterance {
    start_index: u64,
    samples: Vec<f32>,
    trailing_silence_frames: usize,
}

/// Splits a mono stream into bounded speech utterances. Silence closes an
/// utterance after a redemption window that bridges short pauses, and a hard
/// duration cap force-cuts continuous speech, so a downstream recognizer never
/// receives more than [`MAX_UTTERANCE_MS`] of audio at once.
pub(crate) struct LiveSegmenter {
    sample_rate: u32,
    frame_len: usize,
    speech_rms: f32,
    redemption_frames: usize,
    min_samples: usize,
    max_samples: usize,
    consumed: u64,
    frame: Vec<f32>,
    frame_start: u64,
    active: Option<ActiveUtterance>,
}

impl LiveSegmenter {
    pub(crate) fn new(sample_rate: u32, config: SegmenterConfig) -> Self {
        let frame_len = samples_for_ms(sample_rate, FRAME_MS).max(1);
        Self {
            sample_rate,
            frame_len,
            speech_rms: config.speech_rms,
            redemption_frames: (samples_for_ms(sample_rate, REDEMPTION_MS) / frame_len).max(1),
            min_samples: samples_for_ms(sample_rate, config.min_utterance_ms),
            max_samples: samples_for_ms(sample_rate, MAX_UTTERANCE_MS).max(frame_len),
            consumed: 0,
            frame: Vec::with_capacity(frame_len),
            frame_start: 0,
            active: None,
        }
    }

    /// Feeds a block of samples whose first sample sits at absolute index
    /// `start_index`, appending any utterances that close while processing. A
    /// `start_index` past the next expected sample means upstream dropped audio;
    /// the open utterance is abandoned and timing realigns so segment times stay
    /// anchored to absolute sample positions.
    pub(crate) fn push(&mut self, start_index: u64, samples: &[f32], out: &mut Vec<Utterance>) {
        if start_index != self.consumed {
            self.active = None;
            self.frame.clear();
            self.consumed = start_index;
        }
        for &sample in samples {
            if self.frame.is_empty() {
                self.frame_start = self.consumed;
            }
            self.frame.push(sample);
            self.consumed += 1;
            if self.frame.len() == self.frame_len {
                self.process_frame(out);
                self.frame.clear();
            }
        }
    }

    /// Closes any open utterance; call once after the final [`LiveSegmenter::push`].
    pub(crate) fn flush(&mut self, out: &mut Vec<Utterance>) {
        if let Some(active) = self.active.take() {
            self.close(active, out);
        }
    }

    fn process_frame(&mut self, out: &mut Vec<Utterance>) {
        let is_speech = rms(&self.frame) >= self.speech_rms;
        match self.active.as_mut() {
            None => {
                if is_speech {
                    self.active = Some(ActiveUtterance {
                        start_index: self.frame_start,
                        samples: self.frame.clone(),
                        trailing_silence_frames: 0,
                    });
                }
            }
            Some(active) => {
                active.samples.extend_from_slice(&self.frame);
                active.trailing_silence_frames = if is_speech {
                    0
                } else {
                    active.trailing_silence_frames + 1
                };
                let close = active.trailing_silence_frames >= self.redemption_frames
                    || active.samples.len() >= self.max_samples;
                if close {
                    let finished = self.active.take().expect("active utterance present");
                    self.close(finished, out);
                }
            }
        }
    }

    fn close(&self, mut active: ActiveUtterance, out: &mut Vec<Utterance>) {
        let trim = active.trailing_silence_frames * self.frame_len;
        let keep = active.samples.len().saturating_sub(trim);
        active.samples.truncate(keep);
        if active.samples.len() >= self.min_samples {
            out.push(Utterance {
                start_index: active.start_index,
                sample_rate: self.sample_rate,
                samples: active.samples,
            });
        }
    }
}

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
    for segment in &mut segments {
        segment.start_ms += start_ms;
        segment.end_ms += start_ms;
    }
    Ok(segments)
}

fn samples_for_ms(sample_rate: u32, ms: u64) -> usize {
    ((u64::from(sample_rate) * ms) / 1000) as usize
}

#[cfg(test)]
mod tests;
