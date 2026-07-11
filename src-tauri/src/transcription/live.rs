use std::collections::VecDeque;

use super::{rms, Transcriber};

const FRAME_MS: u64 = 20;
const REDEMPTION_MS: u64 = 600;
const MAX_UTTERANCE_MS: u64 = 24_000;
/// Audio kept from just before the gate opens, so quiet word onsets
/// (unvoiced consonants, soft first syllables) reach the recognizer.
const PRE_ROLL_MS: u64 = 240;
/// Trailing sub-gate audio kept after the last speech frame, so word decays
/// are not amputated at the gate.
const TAIL_KEEP_MS: u64 = 160;
/// When the duration cap force-cuts, prefer the most recent quiet frame
/// within this window so the boundary does not land inside a word.
const FORCE_CUT_LOOKBACK_MS: u64 = 2_000;
/// Peak frame RMS below which an utterance counts as faint. The gate sits
/// near the noise floor to favor recall, so faint non-speech audio reaches
/// the recognizer; filler-only decodes of it are dropped as hallucinations.
pub(crate) const QUIET_CONFIRMATION_RMS: f32 = 0.02;
/// Multiple of the tracked noise floor a frame must exceed to count as
/// speech under the adaptive gate.
const NOISE_FLOOR_GATE_RATIO: f32 = 2.5;
/// Per-frame EMA rate of the noise-floor estimate (~1 s time constant).
const NOISE_FLOOR_SMOOTHING: f32 = 0.02;

mod config;
pub(crate) use config::SegmenterConfig;

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
    /// Sample offset just past the most recent sub-gate frame; the preferred
    /// cut point when the duration cap forces a split.
    last_quiet_offset: Option<usize>,
}

/// Splits a mono stream into bounded speech utterances. Silence closes an
/// utterance after a redemption window that bridges short pauses, and a hard
/// duration cap force-cuts continuous speech, so a downstream recognizer never
/// receives more than [`MAX_UTTERANCE_MS`] of audio at once.
pub(crate) struct LiveSegmenter {
    sample_rate: u32,
    frame_len: usize,
    speech_rms: f32,
    adaptive_gate: bool,
    noise_floor: f32,
    redemption_frames: usize,
    min_samples: usize,
    max_samples: usize,
    pre_roll_max: usize,
    tail_keep_samples: usize,
    lookback_samples: usize,
    consumed: u64,
    frame: Vec<f32>,
    frame_start: u64,
    pre_roll: VecDeque<f32>,
    active: Option<ActiveUtterance>,
}

impl LiveSegmenter {
    pub(crate) fn new(sample_rate: u32, config: SegmenterConfig) -> Self {
        let frame_len = samples_for_ms(sample_rate, FRAME_MS).max(1);
        Self {
            sample_rate,
            frame_len,
            speech_rms: config.speech_rms,
            adaptive_gate: config.adaptive_gate,
            noise_floor: config.speech_rms,
            redemption_frames: (samples_for_ms(sample_rate, REDEMPTION_MS) / frame_len).max(1),
            min_samples: samples_for_ms(sample_rate, config.min_utterance_ms),
            max_samples: samples_for_ms(sample_rate, MAX_UTTERANCE_MS).max(frame_len),
            pre_roll_max: samples_for_ms(sample_rate, PRE_ROLL_MS),
            tail_keep_samples: samples_for_ms(sample_rate, TAIL_KEEP_MS),
            lookback_samples: samples_for_ms(sample_rate, FORCE_CUT_LOOKBACK_MS),
            consumed: 0,
            frame: Vec::with_capacity(frame_len),
            frame_start: 0,
            pre_roll: VecDeque::new(),
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
            self.pre_roll.clear();
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
        let frame_rms = rms(&self.frame);
        let is_speech = frame_rms >= self.gate();
        if self.active.is_none() {
            if is_speech {
                self.open_utterance();
            } else {
                self.track_noise_floor(frame_rms);
                self.buffer_pre_roll();
            }
            return;
        }

        let active = self.active.as_mut().expect("active utterance present");
        active.samples.extend_from_slice(&self.frame);
        if is_speech {
            active.trailing_silence_frames = 0;
        } else {
            active.trailing_silence_frames += 1;
            active.last_quiet_offset = Some(active.samples.len());
        }
        let close_now = active.trailing_silence_frames >= self.redemption_frames;
        let cap_hit = active.samples.len() >= self.max_samples;
        if close_now {
            let finished = self.active.take().expect("active utterance present");
            self.close(finished, out);
        } else if cap_hit {
            self.force_cut(out);
        }
    }

    fn gate(&self) -> f32 {
        if self.adaptive_gate {
            (self.noise_floor * NOISE_FLOOR_GATE_RATIO)
                .clamp(self.speech_rms, QUIET_CONFIRMATION_RMS)
        } else {
            self.speech_rms
        }
    }

    /// Tracks ambient level from frames classified as silence outside any
    /// utterance, so room tone raises the gate without speech inflating it.
    fn track_noise_floor(&mut self, frame_rms: f32) {
        if self.adaptive_gate {
            self.noise_floor += (frame_rms - self.noise_floor) * NOISE_FLOOR_SMOOTHING;
        }
    }

    fn open_utterance(&mut self) {
        let mut samples = Vec::with_capacity(self.pre_roll.len() + self.frame_len);
        samples.extend(self.pre_roll.drain(..));
        let start_index = self.frame_start - samples.len() as u64;
        samples.extend_from_slice(&self.frame);
        self.active = Some(ActiveUtterance {
            start_index,
            samples,
            trailing_silence_frames: 0,
            last_quiet_offset: None,
        });
    }

    fn buffer_pre_roll(&mut self) {
        self.pre_roll.extend(self.frame.iter().copied());
        let excess = self.pre_roll.len().saturating_sub(self.pre_roll_max);
        if excess > 0 {
            self.pre_roll.drain(0..excess);
        }
    }

    /// The duration cap hit mid-speech. Cut at the most recent quiet frame so
    /// the boundary avoids the middle of a word; the remainder seeds the next
    /// utterance so no audio is lost.
    fn force_cut(&mut self, out: &mut Vec<Utterance>) {
        let mut active = self.active.take().expect("active utterance present");
        let cut = active
            .last_quiet_offset
            .filter(|offset| *offset > 0 && active.samples.len() - offset <= self.lookback_samples);
        let Some(offset) = cut else {
            self.close(active, out);
            return;
        };

        let remainder = active.samples.split_off(offset);
        out.push(Utterance {
            start_index: active.start_index,
            sample_rate: self.sample_rate,
            samples: active.samples,
        });
        if !remainder.is_empty() {
            self.active = Some(ActiveUtterance {
                start_index: active.start_index + offset as u64,
                samples: remainder,
                trailing_silence_frames: 0,
                last_quiet_offset: None,
            });
        }
    }

    fn close(&self, mut active: ActiveUtterance, out: &mut Vec<Utterance>) {
        let trailing = active.trailing_silence_frames * self.frame_len;
        let speech_len = active.samples.len().saturating_sub(trailing);
        if speech_len < self.min_samples {
            return;
        }
        // Keep a short silence tail so the final word's decay stays intact.
        active
            .samples
            .truncate(speech_len + trailing.min(self.tail_keep_samples));
        out.push(Utterance {
            start_index: active.start_index,
            sample_rate: self.sample_rate,
            samples: active.samples,
        });
    }
}

fn samples_for_ms(sample_rate: u32, ms: u64) -> usize {
    ((u64::from(sample_rate) * ms) / 1000) as usize
}

mod transcribe;
pub(crate) use transcribe::{transcribe_live_utterance, ChannelRole};

#[cfg(test)]
mod tests;
