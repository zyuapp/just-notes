use std::collections::VecDeque;

use super::{
    sink::{emit_unique_live_segment, LiveChannelContext},
    window::{advance_live_decode, prepare_live_decode_window},
    LIVE_TRANSCRIPTION_STEP_MS,
};
use crate::transcription::{
    common_transcript_prefix, completed_transcript_text, estimate_text_end_ms, resample_to_rate,
    unique_transcript_text, LIVE_DUPLICATE_RECENT_SEGMENTS,
};

pub(crate) struct LiveChannelState {
    pub(super) source: &'static str,
    pub(super) speaker: &'static str,
    pub(super) sample_rate: u32,
    pub(super) next_decode_ms: u64,
    pub(super) decode_start_ms: u64,
    pub(super) committed_until_ms: u64,
    pub(super) last_emitted_end_ms: u64,
    pub(super) prompt_tail: VecDeque<String>,
    pub(super) emitted_text_tail: VecDeque<String>,
    pub(super) previous_hypothesis: Option<String>,
}

impl LiveChannelState {
    pub(crate) fn new(source: &'static str, speaker: &'static str, sample_rate: u32) -> Self {
        Self {
            source,
            speaker,
            sample_rate,
            next_decode_ms: LIVE_TRANSCRIPTION_STEP_MS,
            decode_start_ms: 0,
            committed_until_ms: 0,
            last_emitted_end_ms: 0,
            prompt_tail: VecDeque::with_capacity(8),
            emitted_text_tail: VecDeque::with_capacity(LIVE_DUPLICATE_RECENT_SEGMENTS),
            previous_hypothesis: None,
        }
    }

    fn prompt(&self) -> String {
        self.prompt_tail
            .iter()
            .rev()
            .take(4)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub(super) fn remember_prompt_text(&mut self, text: &str) {
        self.prompt_tail.push_back(text.to_string());
        while self.prompt_tail.len() > 8 {
            self.prompt_tail.pop_front();
        }
    }

    pub(super) fn unique_text(&self, text: &str) -> Option<String> {
        unique_transcript_text(text, &self.emitted_text_tail)
    }

    pub(super) fn remember_emitted_text(&mut self, text: &str) {
        self.emitted_text_tail.push_back(text.to_string());
        while self.emitted_text_tail.len() > LIVE_DUPLICATE_RECENT_SEGMENTS {
            self.emitted_text_tail.pop_front();
        }
    }

    pub(crate) fn agreed_text(&mut self, text: &str, final_flush: bool) -> Option<String> {
        let hypothesis = text.trim();
        if hypothesis.is_empty() {
            self.previous_hypothesis = None;
            return None;
        }

        let agreed = if final_flush {
            Some(hypothesis.to_string())
        } else {
            self.previous_hypothesis
                .as_deref()
                .and_then(|previous| common_transcript_prefix(previous, hypothesis))
        };
        self.previous_hypothesis = Some(hypothesis.to_string());
        agreed
    }

    pub(super) fn mark_emitted_until(&mut self, end_ms: u64) {
        self.last_emitted_end_ms = self.last_emitted_end_ms.max(end_ms);
        self.decode_start_ms = self.decode_start_ms.max(end_ms);
        self.previous_hypothesis = None;
    }
}

pub(super) fn process_live_channel(
    context: &LiveChannelContext<'_>,
    state: &mut LiveChannelState,
    final_flush: bool,
) -> Result<usize, String> {
    let Some(window) = prepare_live_decode_window(context.buffers, state, final_flush)? else {
        return Ok(0);
    };

    let normalized_samples = resample_to_rate(&window.samples, state.sample_rate, 16_000);
    let mut segments = context.whisper.transcribe_live(
        &normalized_samples,
        &state.prompt(),
        state.source,
        state.speaker,
    )?;
    segments.sort_by_key(|segment| segment.start_ms);

    let decoded_text = segments
        .iter()
        .map(|segment| segment.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let Some(agreed_text) = state.agreed_text(&decoded_text, final_flush) else {
        advance_live_decode(state, window.commit_until_ms, window.target_end_ms);
        return Ok(0);
    };

    let completed_agreed_text = completed_transcript_text(&agreed_text, final_flush);
    if completed_agreed_text.is_empty() {
        advance_live_decode(state, window.commit_until_ms, window.target_end_ms);
        return Ok(0);
    }
    let stable_end_ms = estimate_text_end_ms(
        &segments,
        &completed_agreed_text,
        window.window_start_ms,
        window.commit_until_ms,
    );

    let emitted = emit_unique_live_segment(
        context,
        state,
        &window,
        &completed_agreed_text,
        stable_end_ms,
    )?;
    advance_live_decode(state, window.commit_until_ms, window.target_end_ms);
    Ok(emitted)
}

#[cfg(test)]
mod tests {
    use super::LiveChannelState;

    #[test]
    fn live_channel_state_requires_two_matching_hypotheses() {
        let mut state = LiveChannelState::new("system", "Others", 48_000);
        assert_eq!(
            state.agreed_text(
                "Section 1 says the green calendar moved beside the copper lamp.",
                false,
            ),
            None,
        );
        assert_eq!(
            state.agreed_text(
                "Section 1 says the green calendar moved beside the copper lamp. Section 2 says the yellow folder.",
                false,
            ),
            Some("Section 1 says the green calendar moved beside the copper lamp.".to_string()),
        );
    }
}
