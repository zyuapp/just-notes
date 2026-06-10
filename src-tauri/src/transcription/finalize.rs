use std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
};

use hound::{SampleFormat, WavReader, WavSpec};
use tauri::{AppHandle, Emitter};

use super::{
    resample_to_rate, rms, samples_to_ms, suppress_cross_channel_bleed, TranscriptionPaths,
    WhisperRuntime,
};
use crate::{
    ipc::FinalizationStatusPayload,
    threads::{
        repository::{render_thread_markdown, set_thread_status, touch_thread},
        transcript_store::write_transcript_jsonl,
        ThreadStatus, TranscriptSegment,
    },
};

const FINALIZE_CHUNK_SECONDS: u64 = 600;
const FINALIZE_SILENT_CHUNK_RMS: f32 = 0.0005;

type FinalizeJobs = Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>;

#[derive(Clone, Default)]
pub(crate) struct FinalizeState(FinalizeJobs);

impl FinalizeState {
    fn begin(&self, thread_id: &str) -> Option<Arc<AtomicBool>> {
        let mut jobs = self.0.lock().ok()?;
        if jobs.contains_key(thread_id) {
            return None;
        }
        let cancel = Arc::new(AtomicBool::new(false));
        jobs.insert(thread_id.to_string(), Arc::clone(&cancel));
        Some(cancel)
    }

    fn finish(&self, thread_id: &str) {
        if let Ok(mut jobs) = self.0.lock() {
            jobs.remove(thread_id);
        }
    }

    pub(crate) fn cancel(&self, thread_id: &str) -> bool {
        let Ok(jobs) = self.0.lock() else {
            return false;
        };
        match jobs.get(thread_id) {
            Some(flag) => {
                flag.store(true, Ordering::Relaxed);
                true
            }
            None => false,
        }
    }
}

pub(crate) struct FinalizationConfig {
    pub(crate) app: AppHandle,
    pub(crate) state: FinalizeState,
    pub(crate) thread_id: String,
    pub(crate) thread_dir: PathBuf,
    pub(crate) paths: TranscriptionPaths,
    pub(crate) markdown_copy: bool,
}

pub(crate) fn spawn_finalization(config: FinalizationConfig) -> bool {
    let mic_path = config.thread_dir.join("mic.wav");
    let system_path = config.thread_dir.join("system.wav");
    if !config.paths.model_path.is_file() || (!mic_path.is_file() && !system_path.is_file()) {
        return false;
    }
    let Some(cancel) = config.state.begin(&config.thread_id) else {
        return false;
    };
    if set_thread_status(&config.thread_dir, ThreadStatus::Transcribing).is_err() {
        config.state.finish(&config.thread_id);
        return false;
    }

    emit_finalization_status(
        &config.app,
        &config.thread_id,
        "running",
        "Improving the transcript from the saved recording",
    );
    thread::spawn(move || {
        // catch_unwind keeps a whisper/decoder panic from leaking the
        // Transcribing status and the registry entry for this thread.
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run_finalization(&config, &cancel, &mic_path, &system_path)
        }))
        .unwrap_or_else(|_| Err("Transcript finalization crashed".to_string()));
        let _ = set_thread_status(&config.thread_dir, ThreadStatus::Idle);
        config.state.finish(&config.thread_id);
        match outcome {
            Ok(true) => emit_finalization_status(
                &config.app,
                &config.thread_id,
                "done",
                "The polished transcript is ready",
            ),
            Ok(false) => emit_finalization_status(
                &config.app,
                &config.thread_id,
                "cancelled",
                "Kept the live transcript",
            ),
            Err(err) => emit_finalization_status(&config.app, &config.thread_id, "failed", &err),
        }
    });
    true
}

fn run_finalization(
    config: &FinalizationConfig,
    cancel: &AtomicBool,
    mic_path: &Path,
    system_path: &Path,
) -> Result<bool, String> {
    let whisper = WhisperRuntime::load(&config.paths.model_path)?;
    let mut segments = transcribe_wav_channel(&whisper, mic_path, "mic", "You", cancel)?;
    segments.extend(transcribe_wav_channel(
        &whisper,
        system_path,
        "system",
        "Others",
        cancel,
    )?);
    if cancel.load(Ordering::Relaxed) {
        return Ok(false);
    }

    segments.sort_by(|left, right| {
        left.start_ms
            .cmp(&right.start_ms)
            .then_with(|| left.end_ms.cmp(&right.end_ms))
            .then_with(|| left.source.cmp(&right.source))
    });
    let segments = suppress_cross_channel_bleed(segments);
    if segments.is_empty() {
        return Ok(false);
    }

    write_transcript_jsonl(&config.thread_dir.join("transcript.jsonl"), &segments)?;
    touch_thread(&config.thread_dir)?;
    if config.markdown_copy {
        render_thread_markdown(&config.thread_dir)?;
    }
    Ok(true)
}

fn transcribe_wav_channel(
    whisper: &WhisperRuntime,
    path: &Path,
    source: &str,
    speaker: &str,
    cancel: &AtomicBool,
) -> Result<Vec<TranscriptSegment>, String> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let mut reader =
        WavReader::open(path).map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
    let spec = reader.spec();
    let chunk_frames = (spec.sample_rate as u64 * FINALIZE_CHUNK_SECONDS) as usize;
    let mut offset_ms = 0u64;
    let mut segments = Vec::new();

    loop {
        if cancel.load(Ordering::Relaxed) {
            return Ok(segments);
        }
        let chunk = read_mono_chunk(&mut reader, spec, chunk_frames)?;
        if chunk.is_empty() {
            break;
        }
        let chunk_ms = samples_to_ms(chunk.len() as u64, spec.sample_rate);
        if rms(&chunk) >= FINALIZE_SILENT_CHUNK_RMS {
            let samples_16k = resample_to_rate(&chunk, spec.sample_rate, 16_000);
            for mut segment in whisper.transcribe(&samples_16k, "", source, speaker)? {
                segment.start_ms += offset_ms;
                segment.end_ms += offset_ms;
                segments.push(segment);
            }
        }
        offset_ms += chunk_ms;
    }
    Ok(segments)
}

fn read_mono_chunk(
    reader: &mut WavReader<BufReader<File>>,
    spec: WavSpec,
    max_frames: usize,
) -> Result<Vec<f32>, String> {
    let channels = spec.channels.max(1) as usize;
    let max_samples = max_frames.saturating_mul(channels);
    let raw: Vec<f32> = match spec.sample_format {
        SampleFormat::Float => reader
            .samples::<f32>()
            .take(max_samples)
            .collect::<Result<_, _>>()
            .map_err(|err| format!("Failed to decode recording audio: {err}"))?,
        SampleFormat::Int => {
            let scale = (1i64 << (spec.bits_per_sample.saturating_sub(1))) as f32;
            reader
                .samples::<i32>()
                .take(max_samples)
                .map(|sample| sample.map(|value| value as f32 / scale))
                .collect::<Result<_, _>>()
                .map_err(|err| format!("Failed to decode recording audio: {err}"))?
        }
    };

    Ok(raw
        .chunks(channels)
        .map(|frame| frame.iter().copied().sum::<f32>() / frame.len() as f32)
        .collect())
}

fn emit_finalization_status(app: &AppHandle, thread_id: &str, state: &str, message: &str) {
    let _ = app.emit(
        "finalization-status",
        FinalizationStatusPayload {
            thread_id: thread_id.to_string(),
            state: state.to_string(),
            message: message.to_string(),
        },
    );
}
