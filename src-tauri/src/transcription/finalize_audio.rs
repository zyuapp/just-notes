use std::{
    fs::File,
    io::BufReader,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

use hound::{SampleFormat, WavReader, WavSpec};

use super::{
    transcribe_live_utterance, wav_duration_ms, ChannelRole, LiveSegmenter, SegmenterConfig,
    Transcriber, Utterance,
};
use crate::threads::{RecordingAudioPaths, TranscriptSegment};

// Audio is read in bounded blocks and fed through the shared segmenter so the
// recognizer only ever decodes one short speech utterance at a time, never the
// whole channel at once.
const FINALIZE_READ_SECONDS: usize = 30;

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

    /// Longest measured duration across the saved channels, or None if neither
    /// can be read. A missing channel counts as zero; a corrupt one is skipped.
    pub(crate) fn measured_duration_ms(&self) -> Option<u64> {
        [self.paths.mic_path(), self.paths.system_path()]
            .into_iter()
            .filter_map(|path| wav_duration_ms(path).ok())
            .max()
    }
}

pub(super) fn transcribe_wav_channel(
    transcriber: &dyn Transcriber,
    path: &Path,
    role: ChannelRole,
    cancel: &AtomicBool,
    config: SegmenterConfig,
) -> Result<Vec<TranscriptSegment>, String> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let mut reader =
        WavReader::open(path).map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
    let spec = reader.spec();
    let block_frames = spec.sample_rate as usize * FINALIZE_READ_SECONDS;
    let mut segmenter = LiveSegmenter::new(spec.sample_rate, config);
    let mut utterances = Vec::new();
    let mut segments = Vec::new();
    let mut next_index = 0u64;

    loop {
        if cancel.load(Ordering::Relaxed) {
            return Ok(segments);
        }
        let block = read_mono_chunk(&mut reader, spec, block_frames)?;
        if block.is_empty() {
            break;
        }
        segmenter.push(next_index, &block, &mut utterances);
        next_index += block.len() as u64;
        drain_utterances(transcriber, role, &mut utterances, &mut segments)?;
    }
    segmenter.flush(&mut utterances);
    drain_utterances(transcriber, role, &mut utterances, &mut segments)?;
    Ok(segments)
}

fn drain_utterances(
    transcriber: &dyn Transcriber,
    role: ChannelRole,
    utterances: &mut Vec<Utterance>,
    segments: &mut Vec<TranscriptSegment>,
) -> Result<(), String> {
    for utterance in utterances.drain(..) {
        segments.extend(transcribe_live_utterance(transcriber, &utterance, role, 0)?);
    }
    Ok(())
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
