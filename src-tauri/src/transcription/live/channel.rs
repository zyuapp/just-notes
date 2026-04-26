use std::{
    collections::VecDeque,
    path::Path,
    sync::{Arc, Mutex},
};

use tauri::{AppHandle, Emitter};

use super::{
    window::{advance_live_decode, prepare_live_decode_window, LiveDecodeWindow},
    LIVE_TRANSCRIPTION_STEP_MS,
};
use crate::threads::repository::touch_thread;
use crate::{
    capture::SharedBuffers,
    ipc::LiveTranscriptSegmentPayload,
    threads::{transcript_store::append_live_segment, TranscriptSegment},
    transcription::{
        common_transcript_prefix, completed_transcript_text, estimate_text_end_ms,
        resample_to_rate, unique_transcript_text, WhisperRuntime, LIVE_DUPLICATE_RECENT_SEGMENTS,
    },
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

    fn remember_prompt_text(&mut self, text: &str) {
        self.prompt_tail.push_back(text.to_string());
        while self.prompt_tail.len() > 8 {
            self.prompt_tail.pop_front();
        }
    }

    fn unique_text(&self, text: &str) -> Option<String> {
        unique_transcript_text(text, &self.emitted_text_tail)
    }

    fn remember_emitted_text(&mut self, text: &str) {
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

    fn mark_emitted_until(&mut self, end_ms: u64) {
        self.last_emitted_end_ms = self.last_emitted_end_ms.max(end_ms);
        self.decode_start_ms = self.decode_start_ms.max(end_ms);
        self.previous_hypothesis = None;
    }
}

pub(super) struct LiveChannelContext<'a> {
    pub(super) app: &'a AppHandle,
    pub(super) whisper: &'a WhisperRuntime,
    pub(super) buffers: &'a Arc<Mutex<SharedBuffers>>,
    pub(super) jsonl_path: &'a Path,
    pub(super) thread_dir: &'a Path,
    pub(super) thread_id: &'a str,
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
    let mut segments = context.whisper.transcribe(
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

fn emit_unique_live_segment(
    context: &LiveChannelContext<'_>,
    state: &mut LiveChannelState,
    window: &LiveDecodeWindow,
    text: &str,
    stable_end_ms: u64,
) -> Result<usize, String> {
    let Some(unique_text) = state.unique_text(text) else {
        state.mark_emitted_until(stable_end_ms);
        return Ok(0);
    };

    let unique_text = unique_text.trim().to_string();
    if unique_text.is_empty() {
        state.mark_emitted_until(stable_end_ms);
        return Ok(0);
    }

    let segment = TranscriptSegment {
        speaker: state.speaker.to_string(),
        source: state.source.to_string(),
        start_ms: state
            .last_emitted_end_ms
            .max(window.window_start_ms)
            .max(window.audible_start_ms.unwrap_or(window.window_start_ms)),
        end_ms: stable_end_ms,
        text: unique_text,
    };

    if segment.end_ms <= segment.start_ms {
        return Ok(0);
    }

    append_live_segment(context.jsonl_path, &segment)?;
    touch_thread(context.thread_dir)?;
    state.remember_prompt_text(&segment.text);
    state.remember_emitted_text(&segment.text);
    let _ = context.app.emit(
        "live-transcript-segment",
        LiveTranscriptSegmentPayload {
            thread_id: context.thread_id.to_string(),
            committed_until_ms: window.commit_until_ms,
            segment,
        },
    );
    state.mark_emitted_until(stable_end_ms);
    Ok(1)
}
