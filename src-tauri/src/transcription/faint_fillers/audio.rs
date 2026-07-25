use std::path::Path;

use hound::{SampleFormat, WavReader};

use crate::transcription::{rms, samples_for_ms};

/// Filler-only text below this frame level is much more likely to be a decode
/// of mic noise than an intentional backchannel. Quiet substantive speech is
/// never subject to this threshold.
pub(crate) const CONFIDENT_SPEECH_RMS: f32 = 0.02;

const FRAME_MS: u64 = 20;
pub(super) const MIN_CONFIDENT_RUN_MS: u64 = 60;
/// Live capture is written as signed 16-bit PCM before stop-time polish. Allow
/// one quantization step so an exact-threshold live frame stays confirmed
/// after persistence.
const RMS_COMPARISON_EPSILON: f32 = 1.0 / 32_768.0;
/// Token timestamps can lag a short word's physical onset slightly.
pub(super) const SEGMENT_CONTEXT_MS: u64 = 40;

/// Compact 20 ms RMS envelope shared by decode-time samples and stop-time
/// WAVs. Keeping both paths on the same frame grid prevents a filler that met
/// the live 60 ms rule from being removed later by a coarser profile.
pub(super) struct SpeechProfile {
    frame_rms: Vec<f32>,
}

impl SpeechProfile {
    fn from_samples(samples: &[f32], sample_rate: u32) -> Self {
        if sample_rate == 0 {
            return Self {
                frame_rms: Vec::new(),
            };
        }
        let frame_len = samples_for_ms(sample_rate, FRAME_MS).max(1);
        Self {
            frame_rms: samples.chunks_exact(frame_len).map(rms).collect(),
        }
    }

    pub(super) fn from_wav(path: &Path) -> Result<Self, String> {
        let mut reader = WavReader::open(path).map_err(|err| {
            format!(
                "Failed to read {} for filler speech profile: {err}",
                path.display()
            )
        })?;
        let spec = reader.spec();
        let frame_samples = samples_for_ms(spec.sample_rate, FRAME_MS).max(1);
        let channels = spec.channels.max(1) as usize;
        let mut builder = SpeechProfileBuilder::new(frame_samples);

        match spec.sample_format {
            SampleFormat::Float => {
                add_samples_to_profile(reader.samples::<f32>(), channels, 1.0, &mut builder)?;
            }
            SampleFormat::Int => {
                let scale = (1i64 << spec.bits_per_sample.saturating_sub(1)) as f64;
                add_samples_to_profile(reader.samples::<i32>(), channels, scale, &mut builder)?;
            }
        }
        Ok(builder.finish())
    }

    /// `None` means the requested segment is not fully covered by recorded
    /// audio. Callers must preserve transcript content when evidence is
    /// unavailable rather than treating a failed or truncated WAV as silence.
    pub(super) fn confident_speech_evidence(&self, start_ms: u64, end_ms: u64) -> Option<bool> {
        if end_ms <= start_ms {
            return None;
        }
        let duration_ms = self.frame_rms.len() as u64 * FRAME_MS;
        if self.frame_rms.is_empty() || end_ms > duration_ms {
            return None;
        }
        let start_ms = start_ms.saturating_sub(SEGMENT_CONTEXT_MS).min(duration_ms);
        let end_ms = end_ms.saturating_add(SEGMENT_CONTEXT_MS).min(duration_ms);
        if end_ms <= start_ms {
            return None;
        }
        let first_frame = (start_ms / FRAME_MS) as usize;
        let last_frame = end_ms.div_ceil(FRAME_MS) as usize;
        Some(has_sustained_confident_frames(
            self.frame_rms
                .get(first_frame..last_frame)
                .unwrap_or_default(),
        ))
    }

    #[cfg(test)]
    pub(super) fn from_frame_levels(frame_rms: &[f32]) -> Self {
        Self {
            frame_rms: frame_rms.to_vec(),
        }
    }
}

pub(super) fn segment_has_confident_speech(
    samples: &[f32],
    sample_rate: u32,
    start_ms: u64,
    end_ms: u64,
) -> bool {
    if sample_rate == 0 || end_ms <= start_ms {
        return false;
    }
    SpeechProfile::from_samples(samples, sample_rate)
        .confident_speech_evidence(start_ms, end_ms)
        .unwrap_or(false)
}

fn has_sustained_confident_frames(frame_rms: &[f32]) -> bool {
    let required_frames = MIN_CONFIDENT_RUN_MS.div_ceil(FRAME_MS) as usize;
    let mut run = 0;
    for level in frame_rms {
        if *level + RMS_COMPARISON_EPSILON >= CONFIDENT_SPEECH_RMS {
            run += 1;
            if run >= required_frames {
                return true;
            }
        } else {
            run = 0;
        }
    }
    false
}

struct SpeechProfileBuilder {
    frame_samples: usize,
    samples: usize,
    square_sum: f64,
    frame_rms: Vec<f32>,
}

impl SpeechProfileBuilder {
    fn new(frame_samples: usize) -> Self {
        Self {
            frame_samples,
            samples: 0,
            square_sum: 0.0,
            frame_rms: Vec::new(),
        }
    }

    fn add_sample(&mut self, sample: f32) {
        self.square_sum += f64::from(sample) * f64::from(sample);
        self.samples += 1;
        if self.samples == self.frame_samples {
            self.frame_rms
                .push((self.square_sum / self.samples as f64).sqrt() as f32);
            self.samples = 0;
            self.square_sum = 0.0;
        }
    }

    fn finish(self) -> SpeechProfile {
        // Match `chunks_exact` in the live path: a final partial 20 ms frame
        // cannot count toward the sustained-duration requirement.
        SpeechProfile {
            frame_rms: self.frame_rms,
        }
    }
}

fn add_samples_to_profile<T, E>(
    samples: impl Iterator<Item = Result<T, E>>,
    channels: usize,
    scale: f64,
    builder: &mut SpeechProfileBuilder,
) -> Result<(), String>
where
    T: Copy + Into<f64>,
    E: std::fmt::Display,
{
    let mut frame = Vec::with_capacity(channels);
    for sample in samples {
        frame.push(sample.map_err(|err| format!("Failed to decode filler speech profile: {err}"))?);
        if frame.len() == channels {
            let mono = frame
                .iter()
                .map(|value| Into::<f64>::into(*value) / scale)
                .sum::<f64>()
                / channels as f64;
            builder.add_sample(mono as f32);
            frame.clear();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
