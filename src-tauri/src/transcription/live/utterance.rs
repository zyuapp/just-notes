use super::SpeechEvidence;

/// A closed run of speech plus the absolute sample index of its first sample.
pub(crate) struct Utterance {
    pub(crate) start_index: u64,
    pub(crate) sample_rate: u32,
    pub(crate) samples: Vec<f32>,
}

pub(super) struct ActiveUtterance {
    pub(super) start_index: u64,
    pub(super) samples: Vec<f32>,
    pub(super) evidence: SpeechEvidence,
    pub(super) trailing_silence_frames: usize,
    /// Sample offset just past the most recent sub-gate frame; the preferred
    /// cut point when the duration cap forces a split.
    pub(super) last_quiet_offset: Option<usize>,
}
