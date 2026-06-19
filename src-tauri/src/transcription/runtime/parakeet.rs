use std::path::Path;

use sherpa_onnx::{OfflineRecognizer, OfflineRecognizerConfig, OfflineTransducerModelConfig};

use super::{default_thread_count, Transcriber};
use crate::{
    threads::TranscriptSegment,
    transcription::{parakeet_model_files, samples_to_ms},
};

mod segments;

use segments::{parakeet_result_segments, SegmentIdentity};

pub(super) struct ParakeetTranscriber {
    recognizer: OfflineRecognizer,
}

impl ParakeetTranscriber {
    pub(super) fn load(model_dir: &Path) -> Result<Self, String> {
        let [encoder, decoder, joiner, tokens] = parakeet_model_files(model_dir);
        for path in [&encoder, &decoder, &joiner, &tokens] {
            if !path.is_file() {
                return Err(format!("Missing Parakeet model file {}", path.display()));
            }
        }

        let mut config = OfflineRecognizerConfig::default();
        config.model_config.transducer = OfflineTransducerModelConfig {
            encoder: Some(encoder.display().to_string()),
            decoder: Some(decoder.display().to_string()),
            joiner: Some(joiner.display().to_string()),
        };
        config.model_config.tokens = Some(tokens.display().to_string());
        config.model_config.model_type = Some("nemo_transducer".to_string());
        config.model_config.provider = Some("cpu".to_string());
        config.model_config.num_threads = default_thread_count() as i32;
        let recognizer = OfflineRecognizer::create(&config)
            .ok_or_else(|| format!("Failed to load Parakeet model {}", model_dir.display()))?;
        Ok(Self { recognizer })
    }
}

impl Transcriber for ParakeetTranscriber {
    fn transcribe_segments(
        &self,
        samples_16k: &[f32],
        _prompt: &str,
        source: &str,
        speaker: &str,
    ) -> Result<Vec<TranscriptSegment>, String> {
        let stream = self.recognizer.create_stream();
        stream.accept_waveform(16_000, samples_16k);
        self.recognizer.decode(&stream);
        Ok(stream
            .get_result()
            .map(|result| {
                parakeet_result_segments(
                    &result,
                    samples_to_ms(samples_16k.len() as u64, 16_000),
                    SegmentIdentity { source, speaker },
                )
            })
            .unwrap_or_default())
    }
}
