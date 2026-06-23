use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use tauri::{AppHandle, Emitter};

use crate::{
    capture::SharedBuffers,
    ipc::LiveTranscriptPayload,
    transcription::{
        load_transcriber, transcribe_live_utterance, LiveSegmenter, SegmenterConfig, Transcriber,
        TranscriptionModelSelection, Utterance,
    },
};

const LIVE_POLL_MS: u64 = 300;

pub(super) struct LiveTranscriptionConfig {
    pub(super) app: AppHandle,
    pub(super) thread_id: String,
    pub(super) buffers: Arc<Mutex<SharedBuffers>>,
    pub(super) model_selection: TranscriptionModelSelection,
    pub(super) mic_sample_rate: u32,
    pub(super) system_sample_rate: u32,
    pub(super) offset_ms: u64,
}

pub(super) struct LiveTranscription {
    should_stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl LiveTranscription {
    pub(super) fn stop(mut self) {
        self.should_stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub(super) fn spawn_live_transcription(config: LiveTranscriptionConfig) -> LiveTranscription {
    let should_stop = Arc::new(AtomicBool::new(false));
    let stop_flag = Arc::clone(&should_stop);
    let worker = thread::spawn(move || run_live_transcription(config, stop_flag));
    LiveTranscription {
        should_stop,
        worker: Some(worker),
    }
}

fn run_live_transcription(config: LiveTranscriptionConfig, should_stop: Arc<AtomicBool>) {
    if !config.model_selection.is_installed() {
        return;
    }
    let Ok(transcriber) = load_transcriber(&config.model_selection) else {
        return;
    };
    let mut mic = LiveChannel::new("mic", "You", config.mic_sample_rate);
    let mut system = LiveChannel::new("system", "Others", config.system_sample_rate);

    // Stop is checked first so a final open utterance is left to the
    // authoritative finalization pass rather than decoded here, which would
    // block the stop command for the length of that decode.
    while !should_stop.load(Ordering::Relaxed) {
        let (mic_start, mic_samples) = read_new(&config.buffers, true, &mut mic.cursor);
        mic.ingest(mic_start, &mic_samples, &*transcriber, &config);
        let (system_start, system_samples) = read_new(&config.buffers, false, &mut system.cursor);
        system.ingest(system_start, &system_samples, &*transcriber, &config);
        thread::sleep(Duration::from_millis(LIVE_POLL_MS));
    }
}

struct LiveChannel {
    source: &'static str,
    speaker: &'static str,
    cursor: u64,
    segmenter: LiveSegmenter,
}

impl LiveChannel {
    fn new(source: &'static str, speaker: &'static str, sample_rate: u32) -> Self {
        Self {
            source,
            speaker,
            cursor: 0,
            segmenter: LiveSegmenter::new(sample_rate, SegmenterConfig::live()),
        }
    }

    fn ingest(
        &mut self,
        start_index: u64,
        samples: &[f32],
        transcriber: &dyn Transcriber,
        config: &LiveTranscriptionConfig,
    ) {
        if samples.is_empty() {
            return;
        }
        let mut utterances = Vec::new();
        self.segmenter.push(start_index, samples, &mut utterances);
        self.emit(utterances, transcriber, config);
    }

    fn emit(
        &self,
        utterances: Vec<Utterance>,
        transcriber: &dyn Transcriber,
        config: &LiveTranscriptionConfig,
    ) {
        for utterance in utterances {
            let Ok(segments) = transcribe_live_utterance(
                transcriber,
                &utterance,
                self.source,
                self.speaker,
                config.offset_ms,
            ) else {
                continue;
            };
            for segment in segments {
                let _ = config.app.emit(
                    "transcript-update",
                    LiveTranscriptPayload {
                        thread_id: config.thread_id.clone(),
                        segment,
                    },
                );
            }
        }
    }
}

/// Returns the unread samples plus the absolute index of their first sample, so
/// the segmenter can detect a gap when the rolling buffer drops un-read audio.
fn read_new(buffers: &Mutex<SharedBuffers>, is_mic: bool, cursor: &mut u64) -> (u64, Vec<f32>) {
    let Ok(shared) = buffers.lock() else {
        return (*cursor, Vec::new());
    };
    let channel = if is_mic { &shared.mic } else { &shared.system };
    let start = (*cursor).max(channel.earliest_index());
    let end = channel.available_end_index();
    *cursor = end.max(*cursor);
    if end > start {
        (start, channel.window(start, end).unwrap_or_default())
    } else {
        (start, Vec::new())
    }
}
