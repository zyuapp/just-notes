use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use tauri::{AppHandle, Emitter};

use super::{
    common_transcript_prefix, completed_transcript_text, estimate_text_end_ms, first_audible_ms,
    ms_to_samples, resample_to_rate, rms, samples_to_ms, unique_transcript_text,
    TranscriptionPaths, WhisperRuntime, LIVE_DUPLICATE_RECENT_SEGMENTS,
};
use crate::threads::repository::touch_thread;
use crate::{
    capture::SharedBuffers,
    ipc::{LiveTranscriptSegmentPayload, LiveTranscriptStatusPayload},
    threads::{transcript_store::append_live_segment, TranscriptSegment},
};

type LiveSampleWindow = Option<Vec<f32>>;

const LIVE_TRANSCRIPTION_STEP_MS: u64 = 2_000;
const LIVE_TRANSCRIPTION_WINDOW_MS: u64 = 12_000;
const LIVE_TRANSCRIPTION_MAX_AGREEMENT_BUFFER_MS: u64 = 30_000;
const LIVE_TRANSCRIPTION_STABILITY_DELAY_MS: u64 = 2_000;
const LIVE_TRANSCRIPTION_POLL_MS: u64 = 250;
const LIVE_SILENCE_RMS_THRESHOLD: f32 = 0.005;

pub(crate) struct LiveTranscriptionThreadConfig {
    pub(crate) app: AppHandle,
    pub(crate) paths: TranscriptionPaths,
    pub(crate) buffers: Arc<Mutex<SharedBuffers>>,
    pub(crate) should_stop: Arc<AtomicBool>,
    pub(crate) thread_id: String,
    pub(crate) thread_dir: PathBuf,
    pub(crate) mic_sample_rate: u32,
    pub(crate) system_sample_rate: u32,
}

pub(crate) fn spawn_live_transcription_thread(
    config: LiveTranscriptionThreadConfig,
) -> Option<JoinHandle<()>> {
    let LiveTranscriptionThreadConfig {
        app,
        paths,
        buffers,
        should_stop,
        thread_id,
        thread_dir,
        mic_sample_rate,
        system_sample_rate,
    } = config;

    if !paths.model_path.is_file() {
        emit_live_status(
            &app,
            &thread_id,
            false,
            "Live transcription is disabled until the local model is installed",
        );
        return None;
    }

    Some(thread::spawn(move || {
        let config = LiveTranscriptionThreadConfig {
            app: app.clone(),
            paths,
            buffers,
            should_stop,
            thread_id: thread_id.clone(),
            thread_dir,
            mic_sample_rate,
            system_sample_rate,
        };
        if let Err(err) = run_live_transcription_loop(config) {
            let _ = app.emit(
                "live-transcript-error",
                LiveTranscriptStatusPayload {
                    thread_id,
                    active: false,
                    message: err,
                    chunk_ms: LIVE_TRANSCRIPTION_WINDOW_MS,
                    overlap_ms: LIVE_TRANSCRIPTION_STABILITY_DELAY_MS,
                },
            );
        }
    }))
}

fn run_live_transcription_loop(config: LiveTranscriptionThreadConfig) -> Result<(), String> {
    let LiveTranscriptionThreadConfig {
        app,
        paths,
        buffers,
        should_stop,
        thread_id,
        thread_dir,
        mic_sample_rate,
        system_sample_rate,
    } = config;
    let whisper = WhisperRuntime::load(&paths.model_path)?;
    emit_live_status(
        &app,
        &thread_id,
        true,
        format!("Live transcription is listening with {}", paths.model_name),
    );

    let jsonl_path = thread_dir.join("transcript.jsonl");
    let mut mic = LiveChannelState::new("mic", "You", mic_sample_rate);
    let mut system = LiveChannelState::new("system", "Others", system_sample_rate);
    let mut emitted_count = 0usize;

    loop {
        let context = LiveChannelContext {
            app: &app,
            whisper: &whisper,
            buffers: &buffers,
            jsonl_path: &jsonl_path,
            thread_dir: &thread_dir,
            thread_id: &thread_id,
        };
        let stopping = should_stop.load(Ordering::Relaxed);
        emitted_count += process_live_channel(&context, &mut mic, stopping)?;
        emitted_count += process_live_channel(&context, &mut system, stopping)?;

        if stopping {
            break;
        }

        thread::sleep(Duration::from_millis(LIVE_TRANSCRIPTION_POLL_MS));
    }

    emit_live_status(
        &app,
        &thread_id,
        false,
        format!("Live transcription stopped after {emitted_count} committed segments"),
    );
    Ok(())
}

pub(crate) struct LiveChannelState {
    source: &'static str,
    speaker: &'static str,
    sample_rate: u32,
    next_decode_ms: u64,
    decode_start_ms: u64,
    committed_until_ms: u64,
    last_emitted_end_ms: u64,
    prompt_tail: VecDeque<String>,
    emitted_text_tail: VecDeque<String>,
    previous_hypothesis: Option<String>,
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

struct LiveChannelContext<'a> {
    app: &'a AppHandle,
    whisper: &'a WhisperRuntime,
    buffers: &'a Arc<Mutex<SharedBuffers>>,
    jsonl_path: &'a Path,
    thread_dir: &'a Path,
    thread_id: &'a str,
}

struct LiveDecodeWindow {
    target_end_ms: u64,
    commit_until_ms: u64,
    window_start_ms: u64,
    samples: Vec<f32>,
    audible_start_ms: Option<u64>,
}

fn live_target_end_ms(
    available_ms: u64,
    state: &LiveChannelState,
    final_flush: bool,
) -> Option<u64> {
    let target_end_ms = if final_flush {
        available_ms
    } else if available_ms >= state.next_decode_ms {
        state.next_decode_ms
    } else {
        return None;
    };
    Some(target_end_ms)
}

fn live_commit_until_ms(target_end_ms: u64, final_flush: bool) -> u64 {
    if final_flush {
        target_end_ms
    } else {
        target_end_ms.saturating_sub(LIVE_TRANSCRIPTION_STABILITY_DELAY_MS)
    }
}

fn prepare_live_decode_window(
    buffers: &Arc<Mutex<SharedBuffers>>,
    state: &mut LiveChannelState,
    final_flush: bool,
) -> Result<Option<LiveDecodeWindow>, String> {
    let available_samples = live_available_samples(buffers, state.source)?;
    let available_ms = samples_to_ms(available_samples, state.sample_rate);
    if available_ms <= state.committed_until_ms {
        return Ok(None);
    }

    let Some(target_end_ms) = live_target_end_ms(available_ms, state, final_flush) else {
        return Ok(None);
    };
    let commit_until_ms = live_commit_until_ms(target_end_ms, final_flush);
    if commit_until_ms <= state.committed_until_ms {
        state.next_decode_ms += LIVE_TRANSCRIPTION_STEP_MS;
        return Ok(None);
    }

    let max_window_start_ms =
        target_end_ms.saturating_sub(LIVE_TRANSCRIPTION_MAX_AGREEMENT_BUFFER_MS);
    if state.decode_start_ms < max_window_start_ms {
        state.decode_start_ms = max_window_start_ms;
        state.previous_hypothesis = None;
    }
    let window_start_ms = state.decode_start_ms;
    let start_index = ms_to_samples(window_start_ms, state.sample_rate) as u64;
    let end_index = ms_to_samples(target_end_ms, state.sample_rate) as u64;

    let samples = live_samples(buffers, state.source, start_index, end_index)?;

    let Some(samples) = samples else {
        advance_live_decode(state, commit_until_ms, target_end_ms);
        return Ok(None);
    };

    if rms(&samples) < LIVE_SILENCE_RMS_THRESHOLD {
        advance_live_decode(state, commit_until_ms, target_end_ms);
        return Ok(None);
    }

    let audible_start_ms =
        first_audible_ms(&samples, state.sample_rate, LIVE_SILENCE_RMS_THRESHOLD)
            .map(|offset_ms| window_start_ms + offset_ms);
    Ok(Some(LiveDecodeWindow {
        target_end_ms,
        commit_until_ms,
        window_start_ms,
        samples,
        audible_start_ms,
    }))
}

fn live_available_samples(
    buffers: &Arc<Mutex<SharedBuffers>>,
    source: &str,
) -> Result<u64, String> {
    let shared = buffers
        .lock()
        .map_err(|_| "Audio buffer lock was poisoned".to_string())?;
    let available_samples = match source {
        "mic" => shared.mic.available_end_index(),
        "system" => shared.system.available_end_index(),
        _ => 0,
    };
    Ok(available_samples)
}

fn live_samples(
    buffers: &Arc<Mutex<SharedBuffers>>,
    source: &str,
    start_index: u64,
    end_index: u64,
) -> Result<LiveSampleWindow, String> {
    let shared = buffers
        .lock()
        .map_err(|_| "Audio buffer lock was poisoned".to_string())?;
    let samples = match source {
        "mic" => shared.mic.window(start_index, end_index),
        "system" => shared.system.window(start_index, end_index),
        _ => None,
    };
    Ok(samples)
}

fn advance_live_decode(state: &mut LiveChannelState, commit_until_ms: u64, target_end_ms: u64) {
    state.committed_until_ms = commit_until_ms;
    state.next_decode_ms = target_end_ms + LIVE_TRANSCRIPTION_STEP_MS;
}

fn process_live_channel(
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

fn emit_live_status(app: &AppHandle, thread_id: &str, active: bool, message: impl Into<String>) {
    let _ = app.emit(
        "live-transcript-status",
        LiveTranscriptStatusPayload {
            thread_id: thread_id.to_string(),
            active,
            message: message.into(),
            chunk_ms: LIVE_TRANSCRIPTION_WINDOW_MS,
            overlap_ms: LIVE_TRANSCRIPTION_STABILITY_DELAY_MS,
        },
    );
}
