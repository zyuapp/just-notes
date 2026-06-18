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
    time::Duration,
};

use hound::{SampleFormat, WavReader, WavSpec};
use tauri::{AppHandle, Emitter};

use super::{
    resample_to_rate, rms, samples_to_ms, suppress_cross_channel_bleed,
    suppress_system_dominated_mic_segments, TranscriptionPaths, WhisperRuntime,
};
use crate::{
    ipc::FinalizationStatusPayload,
    threads::{
        repository::{render_thread_markdown, set_thread_status, touch_thread},
        transcript_store::write_transcript_jsonl,
        RecordingAudioPaths, ThreadStatus, TranscriptSegment,
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

    pub(crate) fn is_active(&self) -> bool {
        self.0.lock().map(|jobs| !jobs.is_empty()).unwrap_or(false)
    }

    pub(crate) fn wait_for_idle(&self) {
        while self.is_active() {
            thread::sleep(Duration::from_millis(100));
        }
    }
}

pub(crate) struct FinalizationConfig {
    pub(crate) app: AppHandle,
    pub(crate) state: FinalizeState,
    pub(crate) thread_id: String,
    pub(crate) thread_dir: PathBuf,
    pub(crate) audio_artifacts: FinalizationAudioArtifacts,
    pub(crate) paths: TranscriptionPaths,
    pub(crate) markdown_copy: bool,
}

#[derive(Clone)]
pub(crate) struct FinalizationAudioArtifacts {
    paths: RecordingAudioPaths,
    retention: FinalizationAudioRetention,
}

#[derive(Clone, Copy)]
enum FinalizationAudioRetention {
    Keep,
    DeleteWhenDone,
}

impl FinalizationAudioArtifacts {
    pub(crate) fn for_thread_dir(thread_dir: &Path, save_raw_audio: bool) -> Self {
        Self {
            paths: RecordingAudioPaths::for_thread_dir(thread_dir),
            retention: if save_raw_audio {
                FinalizationAudioRetention::Keep
            } else {
                FinalizationAudioRetention::DeleteWhenDone
            },
        }
    }

    pub(crate) fn paths(&self) -> &RecordingAudioPaths {
        &self.paths
    }

    pub(crate) fn cleanup_if_transient(&self) -> Result<(), String> {
        match self.retention {
            FinalizationAudioRetention::Keep => Ok(()),
            FinalizationAudioRetention::DeleteWhenDone => self.paths.remove_files(),
        }
    }
}

pub(crate) enum FinalizationStart {
    Started,
    AlreadyRunning,
}

enum FinalizationOutcome {
    Completed,
    Cancelled,
    Empty,
}

pub(crate) fn spawn_finalization(config: FinalizationConfig) -> Result<FinalizationStart, String> {
    if !config.paths.model_path.is_file() {
        return Err("Local transcription model is not installed".to_string());
    }
    if !config.audio_artifacts.paths().has_any() {
        return Err("No recording audio was captured for final transcription".to_string());
    }
    let Some(cancel) = config.state.begin(&config.thread_id) else {
        return Ok(FinalizationStart::AlreadyRunning);
    };
    if set_thread_status(&config.thread_dir, ThreadStatus::Transcribing).is_err() {
        config.state.finish(&config.thread_id);
        return Err("Failed to mark thread as transcribing".to_string());
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
            run_finalization(
                &config,
                &cancel,
                config.audio_artifacts.paths().mic_path(),
                config.audio_artifacts.paths().system_path(),
            )
        }))
        .unwrap_or_else(|_| Err("Transcript finalization crashed".to_string()));
        let outcome = match outcome {
            Ok(FinalizationOutcome::Completed) => config
                .audio_artifacts
                .cleanup_if_transient()
                .map(|()| FinalizationOutcome::Completed),
            other => other,
        };
        let _ = set_thread_status(&config.thread_dir, ThreadStatus::Idle);
        config.state.finish(&config.thread_id);
        match outcome {
            Ok(FinalizationOutcome::Completed) => emit_finalization_status(
                &config.app,
                &config.thread_id,
                "done",
                "The polished transcript is ready",
            ),
            Ok(FinalizationOutcome::Cancelled | FinalizationOutcome::Empty) => {
                emit_finalization_status(
                    &config.app,
                    &config.thread_id,
                    "cancelled",
                    "No final transcript was produced",
                )
            }
            Err(err) => emit_finalization_status(&config.app, &config.thread_id, "failed", &err),
        }
    });
    Ok(FinalizationStart::Started)
}

fn run_finalization(
    config: &FinalizationConfig,
    cancel: &AtomicBool,
    mic_path: &Path,
    system_path: &Path,
) -> Result<FinalizationOutcome, String> {
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
        return Ok(FinalizationOutcome::Cancelled);
    }

    segments.sort_by(|left, right| {
        left.start_ms
            .cmp(&right.start_ms)
            .then_with(|| left.end_ms.cmp(&right.end_ms))
            .then_with(|| left.source.cmp(&right.source))
    });
    let segments = suppress_cross_channel_bleed(segments);
    let segments = suppress_system_dominated_mic_segments(segments, mic_path, system_path)?;
    if segments.is_empty() {
        return Ok(FinalizationOutcome::Empty);
    }

    write_transcript_jsonl(&config.thread_dir.join("transcript.jsonl"), &segments)?;
    touch_thread(&config.thread_dir)?;
    if config.markdown_copy {
        render_thread_markdown(&config.thread_dir)?;
    }
    Ok(FinalizationOutcome::Completed)
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
            for mut segment in whisper.transcribe_finalize(&samples_16k, "", source, speaker)? {
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

pub(crate) fn emit_finalization_failure(app: &AppHandle, thread_id: &str, message: &str) {
    emit_finalization_status(app, thread_id, "failed", message);
}
