use std::{
    path::PathBuf,
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
    threads::{commit::append_thread_segments, TranscriptSegment},
    transcription::{
        load_transcriber, transcribe_live_utterance, LiveSegmenter, SegmenterConfig, Transcriber,
        TranscriptionModelSelection, Utterance,
    },
};

const LIVE_POLL_MS: u64 = 300;

pub(super) struct LiveTranscriptionConfig {
    pub(super) app: AppHandle,
    pub(super) thread_id: String,
    pub(super) thread_dir: PathBuf,
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
    let mut mic = LiveChannel::new("mic", "You", true, config.mic_sample_rate);
    let mut system = LiveChannel::new("system", "Others", false, config.system_sample_rate);

    while !should_stop.load(Ordering::Relaxed) {
        publish(&config, mic.drain(&config, &*transcriber));
        publish(&config, system.drain(&config, &*transcriber));
        thread::sleep(Duration::from_millis(LIVE_POLL_MS));
    }

    // No finalization pass runs on stop, so flush the audio since the last poll
    // plus each channel's open utterance here; otherwise the last thing said
    // before stop would be missing from the transcript.
    publish(&config, mic.drain_and_flush(&config, &*transcriber));
    publish(&config, system.drain_and_flush(&config, &*transcriber));
}

// Emits each new segment to the UI and appends it to the thread's transcript
// JSONL, so the transcript is durable during recording and authoritative on stop.
fn publish(config: &LiveTranscriptionConfig, segments: Vec<TranscriptSegment>) {
    if segments.is_empty() {
        return;
    }
    for segment in &segments {
        let _ = config.app.emit(
            "transcript-update",
            LiveTranscriptPayload {
                thread_id: config.thread_id.clone(),
                segment: segment.clone(),
            },
        );
    }
    let _ = append_thread_segments(&config.thread_dir, &segments);
}

struct LiveChannel {
    source: &'static str,
    speaker: &'static str,
    is_mic: bool,
    cursor: u64,
    segmenter: LiveSegmenter,
}

impl LiveChannel {
    fn new(source: &'static str, speaker: &'static str, is_mic: bool, sample_rate: u32) -> Self {
        Self {
            source,
            speaker,
            is_mic,
            cursor: 0,
            segmenter: LiveSegmenter::new(sample_rate, SegmenterConfig::live()),
        }
    }

    // Transcribes utterances that closed in the audio captured since the last call.
    fn drain(
        &mut self,
        config: &LiveTranscriptionConfig,
        transcriber: &dyn Transcriber,
    ) -> Vec<TranscriptSegment> {
        let (start_index, samples) = read_new(&config.buffers, self.is_mic, &mut self.cursor);
        if samples.is_empty() {
            return Vec::new();
        }
        let mut utterances = Vec::new();
        self.segmenter.push(start_index, &samples, &mut utterances);
        self.transcribe(utterances, transcriber, config.offset_ms)
    }

    // Drains, then closes the open utterance so the channel's tail is captured.
    fn drain_and_flush(
        &mut self,
        config: &LiveTranscriptionConfig,
        transcriber: &dyn Transcriber,
    ) -> Vec<TranscriptSegment> {
        let mut segments = self.drain(config, transcriber);
        let mut utterances = Vec::new();
        self.segmenter.flush(&mut utterances);
        segments.extend(self.transcribe(utterances, transcriber, config.offset_ms));
        segments
    }

    fn transcribe(
        &self,
        utterances: Vec<Utterance>,
        transcriber: &dyn Transcriber,
        offset_ms: u64,
    ) -> Vec<TranscriptSegment> {
        let mut segments = Vec::new();
        for utterance in utterances {
            if let Ok(mut produced) = transcribe_live_utterance(
                transcriber,
                &utterance,
                self.source,
                self.speaker,
                offset_ms,
            ) {
                segments.append(&mut produced);
            }
        }
        segments
    }
}

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
