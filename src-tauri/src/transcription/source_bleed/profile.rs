use std::path::Path;

use hound::{SampleFormat, WavReader};

const ENVELOPE_FRAME_MS: u64 = 25;
const MIN_CORRELATION_FRAMES: usize = 4;

#[derive(Debug, Clone, Copy)]
struct EnvelopeBin {
    square_sum: f64,
    samples: u64,
}

#[derive(Debug)]
pub(super) struct ChannelProfile {
    bins: Vec<EnvelopeBin>,
}

impl ChannelProfile {
    pub(super) fn from_wav(path: &Path) -> Result<Self, String> {
        let mut reader = WavReader::open(path)
            .map_err(|err| format!("Failed to read {} for bleed profile: {err}", path.display()))?;
        let spec = reader.spec();
        let frames_per_bin = frames_per_bin(spec.sample_rate);
        let channels = spec.channels.max(1) as usize;
        let mut builder = ChannelProfileBuilder::new(frames_per_bin);

        match spec.sample_format {
            SampleFormat::Float => {
                let samples = reader.samples::<f32>();
                add_samples_to_profile(samples, channels, &mut builder)?;
            }
            SampleFormat::Int => {
                let scale = (1i64 << (spec.bits_per_sample.saturating_sub(1))) as f32;
                let samples = reader
                    .samples::<i32>()
                    .map(|sample| sample.map(|value| value as f32 / scale));
                add_samples_to_profile(samples, channels, &mut builder)?;
            }
        }

        Ok(builder.finish())
    }

    pub(super) fn rms(&self, start_ms: u64, end_ms: u64) -> f32 {
        if end_ms <= start_ms {
            return 0.0;
        }
        let start_bin = (start_ms / ENVELOPE_FRAME_MS) as usize;
        let end_bin = end_ms.div_ceil(ENVELOPE_FRAME_MS) as usize;
        let mut square_sum = 0.0;
        let mut samples = 0u64;
        for bin in self.bins.get(start_bin..end_bin).unwrap_or_default() {
            square_sum += bin.square_sum;
            samples += bin.samples;
        }
        if samples == 0 {
            0.0
        } else {
            (square_sum / samples as f64).sqrt() as f32
        }
    }

    fn frame_value(&self, frame: isize) -> Option<f32> {
        if frame < 0 {
            return None;
        }
        match self.bins.get(frame as usize) {
            Some(bin) if bin.samples > 0 => {
                Some((bin.square_sum / bin.samples as f64).sqrt() as f32)
            }
            _ => None,
        }
    }

    #[cfg(test)]
    pub(super) fn from_envelope(values: &[f32]) -> Self {
        const SAMPLES_PER_FRAME: u64 = 400;
        Self {
            bins: values
                .iter()
                .map(|value| EnvelopeBin {
                    square_sum: (*value as f64 * *value as f64) * SAMPLES_PER_FRAME as f64,
                    samples: SAMPLES_PER_FRAME,
                })
                .collect(),
        }
    }
}

/// Highest normalized correlation between the mic and system energy envelopes
/// over the segment window, searching lags up to `max_lag_ms` in either
/// direction to absorb the acoustic echo delay and any offset between the two
/// recorded channels. Returns a value in `0.0..=1.0`.
pub(super) fn max_envelope_correlation(
    mic: &ChannelProfile,
    system: &ChannelProfile,
    start_ms: u64,
    end_ms: u64,
    max_lag_ms: u64,
) -> f32 {
    if end_ms <= start_ms {
        return 0.0;
    }
    let start_frame = (start_ms / ENVELOPE_FRAME_MS) as isize;
    let end_frame = end_ms.div_ceil(ENVELOPE_FRAME_MS) as isize;
    let lag_frames = (max_lag_ms / ENVELOPE_FRAME_MS) as isize;
    let mut best = 0.0f32;
    for lag in -lag_frames..=lag_frames {
        if let Some(correlation) = aligned_correlation(mic, system, start_frame, end_frame, lag) {
            if correlation > best {
                best = correlation;
            }
        }
    }
    best
}

/// Correlate the two envelopes over only the frames where both channels have
/// recorded audio at this lag. Returns `None` when too few frames overlap, so a
/// large lag that mostly runs off the end of a channel cannot manufacture a
/// match from missing data.
fn aligned_correlation(
    mic: &ChannelProfile,
    system: &ChannelProfile,
    start_frame: isize,
    end_frame: isize,
    lag: isize,
) -> Option<f32> {
    let mut mic_values = Vec::new();
    let mut system_values = Vec::new();
    for frame in start_frame..end_frame {
        if let (Some(mic_value), Some(system_value)) =
            (mic.frame_value(frame), system.frame_value(frame + lag))
        {
            mic_values.push(mic_value);
            system_values.push(system_value);
        }
    }
    if mic_values.len() < MIN_CORRELATION_FRAMES {
        return None;
    }
    Some(pearson(&mic_values, &system_values))
}

fn pearson(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len().min(b.len());
    if len == 0 {
        return 0.0;
    }
    let count = len as f32;
    let mean_a = a[..len].iter().sum::<f32>() / count;
    let mean_b = b[..len].iter().sum::<f32>() / count;
    let mut covariance = 0.0f32;
    let mut variance_a = 0.0f32;
    let mut variance_b = 0.0f32;
    for index in 0..len {
        let delta_a = a[index] - mean_a;
        let delta_b = b[index] - mean_b;
        covariance += delta_a * delta_b;
        variance_a += delta_a * delta_a;
        variance_b += delta_b * delta_b;
    }
    if variance_a <= f32::EPSILON || variance_b <= f32::EPSILON {
        return 0.0;
    }
    covariance / (variance_a * variance_b).sqrt()
}

struct ChannelProfileBuilder {
    bins: Vec<EnvelopeBin>,
    frames_per_bin: u64,
    current_frame: u64,
    square_sum: f64,
    samples: u64,
}

impl ChannelProfileBuilder {
    fn new(frames_per_bin: u64) -> Self {
        Self {
            bins: Vec::new(),
            frames_per_bin,
            current_frame: 0,
            square_sum: 0.0,
            samples: 0,
        }
    }

    fn add_frame(&mut self, sample: f32) {
        self.square_sum += sample as f64 * sample as f64;
        self.samples += 1;
        self.current_frame += 1;
        if self.current_frame >= self.frames_per_bin {
            self.flush();
        }
    }

    fn finish(mut self) -> ChannelProfile {
        self.flush();
        ChannelProfile { bins: self.bins }
    }

    fn flush(&mut self) {
        if self.samples == 0 {
            return;
        }
        self.bins.push(EnvelopeBin {
            square_sum: self.square_sum,
            samples: self.samples,
        });
        self.current_frame = 0;
        self.square_sum = 0.0;
        self.samples = 0;
    }
}

fn frames_per_bin(sample_rate: u32) -> u64 {
    ((sample_rate as u64 * ENVELOPE_FRAME_MS) / 1000).max(1)
}

fn add_samples_to_profile<T, E>(
    samples: impl Iterator<Item = Result<T, E>>,
    channels: usize,
    builder: &mut ChannelProfileBuilder,
) -> Result<(), String>
where
    T: Copy + Into<f32>,
    E: std::fmt::Display,
{
    let mut frame = Vec::with_capacity(channels);
    for sample in samples {
        frame.push(sample.map_err(|err| format!("Failed to decode bleed profile audio: {err}"))?);
        if frame.len() == channels {
            let mono = frame.iter().map(|value| (*value).into()).sum::<f32>() / channels as f32;
            builder.add_frame(mono);
            frame.clear();
        }
    }
    Ok(())
}
