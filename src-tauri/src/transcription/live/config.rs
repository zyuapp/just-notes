/// Tuning for turning a raw sample stream into speech utterances.
#[derive(Clone, Copy)]
pub(crate) struct SegmenterConfig {
    /// Frame RMS at or above which a 20 ms frame counts as speech; with
    /// `adaptive_gate` this is the gate's lower bound instead.
    pub(crate) speech_rms: f32,
    /// Shortest candidate audio span worth transcribing. A separate gated-frame
    /// minimum prevents pre-roll alone from satisfying this duration.
    pub(crate) min_utterance_ms: u64,
    /// Gate at a multiple of the tracked noise floor. A fixed gate cannot
    /// serve both silent rooms (quiet speech needs a low gate) and noisy
    /// rooms (room tone needs a high one).
    pub(crate) adaptive_gate: bool,
}

impl SegmenterConfig {
    /// Live: recall-biased, since the live transcript is authoritative. The
    /// gate rides the measured noise floor, and filler hallucinations on
    /// faint audio are rejected after decoding (see
    /// `transcribe_live_utterance`).
    pub(crate) fn live() -> Self {
        Self {
            speech_rms: 0.002,
            min_utterance_ms: 150,
            adaptive_gate: true,
        }
    }

    /// A low fixed gate that re-reads whole saved channels. No shipping path
    /// uses it; the quality harness measures it as a recall ceiling for
    /// [`Self::live`].
    #[cfg(test)]
    pub(crate) fn finalize() -> Self {
        Self {
            speech_rms: 0.0008,
            min_utterance_ms: 60,
            adaptive_gate: false,
        }
    }
}
