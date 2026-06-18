use std::path::Path;

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use super::{default_thread_count, Transcriber};
use crate::{
    threads::TranscriptSegment,
    transcription::{clean_transcript_text, is_ignored_transcript_text},
};

const FINALIZE_BEAM_SIZE: i32 = 5;

pub(super) struct WhisperTranscriber {
    ctx: WhisperContext,
}

struct TranscriptionRequest<'a> {
    samples_16k: &'a [f32],
    prompt: &'a str,
    source: &'a str,
    speaker: &'a str,
}

impl WhisperTranscriber {
    pub(super) fn load(model_path: &Path) -> Result<Self, String> {
        let ctx = WhisperContext::new_with_params(model_path, whisper_context_parameters())
            .map_err(|err| {
                format!(
                    "Failed to load local transcription model {}: {err}",
                    model_path.display()
                )
            })?;
        Ok(Self { ctx })
    }

    fn transcribe_with_request(
        &self,
        request: TranscriptionRequest<'_>,
    ) -> Result<Vec<TranscriptSegment>, String> {
        let mut state = self
            .ctx
            .create_state()
            .map_err(|err| format!("Failed to create transcription state: {err}"))?;
        let mut params = full_params();
        params.set_language(Some("en"));
        params.set_n_threads(default_thread_count() as i32);
        params.set_no_context(true);
        params.set_single_segment(false);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        let prompt = request.prompt.trim();
        if !prompt.is_empty() {
            params.set_initial_prompt(prompt);
        }

        state
            .full(params, request.samples_16k)
            .map_err(|err| format!("Failed to transcribe {} audio: {err}", request.source))?;

        let mut segments = Vec::new();
        for segment in state.as_iter() {
            let text = clean_transcript_text(&segment.to_string());
            if text.is_empty() || is_ignored_transcript_text(&text) {
                continue;
            }
            let start_ms = (segment.start_timestamp().max(0) as u64) * 10;
            let end_ms = (segment
                .end_timestamp()
                .max(segment.start_timestamp())
                .max(0) as u64)
                * 10;
            segments.push(TranscriptSegment {
                speaker: request.speaker.to_string(),
                source: request.source.to_string(),
                start_ms,
                end_ms,
                text,
            });
        }
        Ok(segments)
    }
}

impl Transcriber for WhisperTranscriber {
    fn transcribe_segments(
        &self,
        samples_16k: &[f32],
        prompt: &str,
        source: &str,
        speaker: &str,
    ) -> Result<Vec<TranscriptSegment>, String> {
        self.transcribe_with_request(TranscriptionRequest {
            samples_16k,
            prompt,
            source,
            speaker,
        })
    }
}

fn whisper_context_parameters<'a>() -> WhisperContextParameters<'a> {
    let mut params = WhisperContextParameters::default();
    params.flash_attn(true);
    params
}

fn full_params() -> FullParams<'static, 'static> {
    FullParams::new(SamplingStrategy::BeamSearch {
        beam_size: FINALIZE_BEAM_SIZE,
        patience: -1.0,
    })
}

#[cfg(test)]
mod tests {
    use super::whisper_context_parameters;

    #[test]
    fn context_params_enable_flash_attention() {
        assert!(whisper_context_parameters().flash_attn);
    }
}
