use std::thread;

use super::TranscriptionModelSelection;
use crate::threads::TranscriptSegment;

mod parakeet;

use parakeet::ParakeetTranscriber;

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
    Ok(Box::new(ParakeetTranscriber::load(&selection.model_path)?))
}

pub(super) fn default_thread_count() -> usize {
    thread::available_parallelism()
        .map(|count| count.get().clamp(2, 8))
        .unwrap_or(4)
}
