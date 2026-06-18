use std::thread;

use super::TranscriptionModelSelection;
use crate::threads::TranscriptSegment;

mod parakeet;
mod whisper;

use parakeet::ParakeetTranscriber;
use whisper::WhisperTranscriber;

pub(crate) trait Transcriber {
    fn transcribe_segments(
        &self,
        samples_16k: &[f32],
        prompt: &str,
        source: &str,
        speaker: &str,
    ) -> Result<Vec<TranscriptSegment>, String>;
}

pub(crate) type BoxedTranscriber = Box<dyn Transcriber>;

pub(crate) fn load_transcriber(
    selection: &TranscriptionModelSelection,
) -> Result<BoxedTranscriber, String> {
    match selection.provider {
        super::TranscriptionProvider::Parakeet => {
            Ok(Box::new(ParakeetTranscriber::load(&selection.model_path)?))
        }
        super::TranscriptionProvider::Whisper => {
            Ok(Box::new(WhisperTranscriber::load(&selection.model_path)?))
        }
    }
}

pub(super) fn samples_to_ms(samples: u64, sample_rate: u32) -> u64 {
    samples.saturating_mul(1000) / u64::from(sample_rate.max(1))
}

pub(super) fn default_thread_count() -> usize {
    thread::available_parallelism()
        .map(|count| count.get().clamp(2, 8))
        .unwrap_or(4)
}
