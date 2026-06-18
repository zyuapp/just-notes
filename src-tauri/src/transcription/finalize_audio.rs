use std::{
    fs::File,
    io::BufReader,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

use hound::{SampleFormat, WavReader, WavSpec};

use super::{resample_to_rate, rms, samples_to_ms, Transcriber};
use crate::threads::{RecordingAudioPaths, TranscriptSegment};

const FINALIZE_CHUNK_SECONDS: u64 = 600;
const FINALIZE_SILENT_CHUNK_RMS: f32 = 0.0005;

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

pub(super) fn transcribe_wav_channel(
    transcriber: &dyn Transcriber,
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
            for mut segment in transcriber.transcribe_segments(&samples_16k, "", source, speaker)? {
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
