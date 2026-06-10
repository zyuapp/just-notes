use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use tauri::{AppHandle, Emitter};

use std::collections::VecDeque;

use super::{channel::LiveChannelState, window::LiveDecodeWindow};
use crate::threads::repository::touch_thread;
use crate::{
    capture::SharedBuffers,
    ipc::LiveTranscriptSegmentPayload,
    threads::{transcript_store::append_live_segment, TranscriptSegment},
    transcription::{is_duplicate_of_recent, WhisperRuntime},
};

pub(super) struct LiveChannelContext<'a> {
    pub(super) app: &'a AppHandle,
    pub(super) whisper: &'a WhisperRuntime,
    pub(super) buffers: &'a Arc<Mutex<SharedBuffers>>,
    pub(super) jsonl_path: &'a Path,
    pub(super) thread_dir: &'a Path,
    pub(super) thread_id: &'a str,
    pub(super) cross_channel_tail: Option<VecDeque<String>>,
}

pub(super) fn emit_unique_live_segment(
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

    if let Some(tail) = &context.cross_channel_tail {
        if is_duplicate_of_recent(&unique_text, tail) {
            state.mark_emitted_until(stable_end_ms);
            return Ok(0);
        }
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
