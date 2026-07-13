/// Tracks uninterrupted runs of gate-positive audio independently from
/// pre-roll and redeemed pauses, which remain useful recognizer context.
pub(super) struct SpeechEvidence {
    current_run: usize,
    longest_run: usize,
    longest_before_last_quiet: usize,
}

impl SpeechEvidence {
    pub(super) fn started(frame_len: usize) -> Self {
        Self {
            current_run: frame_len,
            longest_run: frame_len,
            longest_before_last_quiet: 0,
        }
    }

    pub(super) fn observe_speech(&mut self, frame_len: usize) {
        self.current_run += frame_len;
        self.longest_run = self.longest_run.max(self.current_run);
    }

    pub(super) fn observe_quiet(&mut self) {
        self.longest_before_last_quiet = self.longest_run;
        self.current_run = 0;
    }

    pub(super) fn longest_run(&self) -> usize {
        self.longest_run
    }

    /// Splits at the most recently observed quiet frame. The returned evidence
    /// belongs to audio after the cut; `self` retains only pre-cut evidence.
    pub(super) fn split_at_last_quiet(&mut self) -> Self {
        let remainder = Self {
            current_run: self.current_run,
            longest_run: self.current_run,
            longest_before_last_quiet: 0,
        };
        self.current_run = 0;
        self.longest_run = self.longest_before_last_quiet;
        self.longest_before_last_quiet = 0;
        remainder
    }
}
