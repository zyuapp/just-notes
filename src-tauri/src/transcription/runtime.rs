use std::{path::Path, thread};

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use super::{clean_transcript_text, is_ignored_transcript_text};
use crate::threads::TranscriptSegment;

const WHISPER_AUDIO_CONTEXT_SAMPLES_PER_TOKEN: usize = 320;

pub(crate) struct WhisperRuntime {
    ctx: WhisperContext,
}

#[derive(Clone, Copy)]
enum DecodeProfile {
    Live,
    Finalize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DecodeStrategy {
    Greedy { best_of: i32 },
    BeamSearch { beam_size: i32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DecodeConfig {
    strategy: DecodeStrategy,
    audio_ctx: Option<i32>,
}

struct TranscriptionRequest<'a> {
    profile: DecodeProfile,
    samples_16k: &'a [f32],
    prompt: &'a str,
    source: &'a str,
    speaker: &'a str,
}

impl WhisperRuntime {
    pub(crate) fn load(model_path: &Path) -> Result<Self, String> {
        let ctx = WhisperContext::new_with_params(model_path, whisper_context_parameters())
            .map_err(|err| {
                format!(
                    "Failed to load local transcription model {}: {err}",
                    model_path.display()
                )
            })?;
        Ok(Self { ctx })
    }

    pub(crate) fn transcribe_live(
        &self,
        samples_16k: &[f32],
        prompt: &str,
        source: &str,
        speaker: &str,
    ) -> Result<Vec<TranscriptSegment>, String> {
        self.transcribe_with_request(TranscriptionRequest {
            profile: DecodeProfile::Live,
            samples_16k,
            prompt,
            source,
            speaker,
        })
    }

    pub(crate) fn transcribe_finalize(
        &self,
        samples_16k: &[f32],
        prompt: &str,
        source: &str,
        speaker: &str,
    ) -> Result<Vec<TranscriptSegment>, String> {
        self.transcribe_with_request(TranscriptionRequest {
            profile: DecodeProfile::Finalize,
            samples_16k,
            prompt,
            source,
            speaker,
        })
    }

    fn transcribe_with_request(
        &self,
        request: TranscriptionRequest<'_>,
    ) -> Result<Vec<TranscriptSegment>, String> {
        let mut state = self
            .ctx
            .create_state()
            .map_err(|err| format!("Failed to create transcription state: {err}"))?;
        let config = decode_config(
            request.profile,
            request.samples_16k.len(),
            self.ctx.model_n_audio_ctx(),
        );
        let mut params = full_params(config);
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

fn whisper_context_parameters<'a>() -> WhisperContextParameters<'a> {
    let mut params = WhisperContextParameters::default();
    params.flash_attn(true);
    params
}

fn decode_config(profile: DecodeProfile, sample_count: usize, max_audio_ctx: i32) -> DecodeConfig {
    match profile {
        DecodeProfile::Live => DecodeConfig {
            strategy: DecodeStrategy::Greedy { best_of: 1 },
            audio_ctx: live_audio_ctx(sample_count, max_audio_ctx),
        },
        DecodeProfile::Finalize => DecodeConfig {
            strategy: DecodeStrategy::BeamSearch { beam_size: 5 },
            audio_ctx: None,
        },
    }
}

fn full_params(config: DecodeConfig) -> FullParams<'static, 'static> {
    let mut params = match config.strategy {
        DecodeStrategy::Greedy { best_of } => FullParams::new(SamplingStrategy::Greedy { best_of }),
        DecodeStrategy::BeamSearch { beam_size } => FullParams::new(SamplingStrategy::BeamSearch {
            beam_size,
            patience: -1.0,
        }),
    };
    if let Some(audio_ctx) = config.audio_ctx {
        params.set_audio_ctx(audio_ctx);
    }
    params
}

fn live_audio_ctx(sample_count: usize, max_audio_ctx: i32) -> Option<i32> {
    if sample_count == 0 || max_audio_ctx <= 0 {
        return None;
    }
    let tokens = sample_count.div_ceil(WHISPER_AUDIO_CONTEXT_SAMPLES_PER_TOKEN);
    Some((tokens as i32).clamp(1, max_audio_ctx))
}

fn default_thread_count() -> usize {
    thread::available_parallelism()
        .map(|count| count.get().clamp(2, 8))
        .unwrap_or(4)
}

#[cfg(test)]
mod tests {
    use super::{
        decode_config, live_audio_ctx, whisper_context_parameters, DecodeConfig, DecodeProfile,
        DecodeStrategy,
    };

    #[test]
    fn context_params_enable_flash_attention() {
        assert!(whisper_context_parameters().flash_attn);
    }

    #[test]
    fn live_decode_uses_greedy_sampling_and_audio_context() {
        assert_eq!(
            decode_config(DecodeProfile::Live, 16_000 * 2, 1_500),
            DecodeConfig {
                strategy: DecodeStrategy::Greedy { best_of: 1 },
                audio_ctx: Some(100),
            }
        );
    }

    #[test]
    fn finalization_decode_keeps_beam_search_quality_defaults() {
        assert_eq!(
            decode_config(DecodeProfile::Finalize, 16_000 * 12, 1_500),
            DecodeConfig {
                strategy: DecodeStrategy::BeamSearch { beam_size: 5 },
                audio_ctx: None,
            }
        );
    }

    #[test]
    fn live_audio_context_scales_with_window_length_and_clamps_to_model_limit() {
        assert_eq!(live_audio_ctx(16_000 * 2, 1_500), Some(100));
        assert_eq!(live_audio_ctx(16_000 * 12, 1_500), Some(600));
        assert_eq!(live_audio_ctx(16_000 * 30, 1_500), Some(1_500));
        assert_eq!(live_audio_ctx(16_000 * 45, 1_500), Some(1_500));
        assert_eq!(live_audio_ctx(16_000 * 12, 500), Some(500));
    }

    #[test]
    fn empty_live_audio_does_not_override_audio_context() {
        assert_eq!(live_audio_ctx(0, 1_500), None);
        assert_eq!(live_audio_ctx(16_000, 0), None);
    }
}
