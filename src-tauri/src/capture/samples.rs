use std::sync::{Arc, Mutex};

use cpal::{
    traits::DeviceTrait, Device, SampleFormat, Stream, StreamConfig, SupportedStreamConfig,
};

use super::model::{CaptureSource, SharedBuffers};

#[cfg(test)]
mod tests;

pub(super) fn build_capture_stream(
    device: Device,
    supported_config: SupportedStreamConfig,
    buffers: Arc<Mutex<SharedBuffers>>,
    source: CaptureSource,
) -> Result<Stream, String> {
    let sample_format = supported_config.sample_format();
    let config: StreamConfig = supported_config.into();

    let err_source = match source {
        CaptureSource::Mic => "microphone",
        CaptureSource::System => "system loopback",
    };
    let err_fn = move |err| eprintln!("{err_source} stream error: {err}");

    match sample_format {
        SampleFormat::F32 => device.build_input_stream(
            &config,
            {
                let mut selector = ChannelSelector::new();
                move |data: &[f32], _| {
                    push_f32_samples(data, config.channels, &buffers, source, &mut selector)
                }
            },
            err_fn,
            None,
        ),
        SampleFormat::I16 => device.build_input_stream(
            &config,
            {
                let mut selector = ChannelSelector::new();
                move |data: &[i16], _| {
                    push_i16_samples(data, config.channels, &buffers, source, &mut selector)
                }
            },
            err_fn,
            None,
        ),
        SampleFormat::U16 => device.build_input_stream(
            &config,
            {
                let mut selector = ChannelSelector::new();
                move |data: &[u16], _| {
                    push_u16_samples(data, config.channels, &buffers, source, &mut selector)
                }
            },
            err_fn,
            None,
        ),
        other => {
            return Err(format!(
                "Unsupported {err_source} sample format: {other:?}. Expected f32, i16, or u16."
            ))
        }
    }
    .map_err(|err| format!("Failed to build {err_source} input stream: {err}"))
}

fn push_f32_samples(
    samples: &[f32],
    channels: u16,
    buffers: &Arc<Mutex<SharedBuffers>>,
    source: CaptureSource,
    selector: &mut ChannelSelector,
) {
    let channels = channels.max(1) as usize;
    let channel = selector.pick(samples, channels);
    push_mono_frames(
        samples
            .chunks(channels)
            .map(move |frame| frame[channel.min(frame.len() - 1)]),
        buffers,
        source,
    );
}

fn push_i16_samples(
    samples: &[i16],
    channels: u16,
    buffers: &Arc<Mutex<SharedBuffers>>,
    source: CaptureSource,
    selector: &mut ChannelSelector,
) {
    let converted: Vec<f32> = samples
        .iter()
        .map(|sample| *sample as f32 / i16::MAX as f32)
        .collect();
    push_f32_samples(&converted, channels, buffers, source, selector);
}

fn push_u16_samples(
    samples: &[u16],
    channels: u16,
    buffers: &Arc<Mutex<SharedBuffers>>,
    source: CaptureSource,
    selector: &mut ChannelSelector,
) {
    let converted: Vec<f32> = samples
        .iter()
        .map(|sample| (*sample as f32 - 32768.0) / 32768.0)
        .collect();
    push_f32_samples(&converted, channels, buffers, source, selector);
}

/// Follows the loudest channel of a multi-channel stream. Devices can expose
/// several channels with voice on only one (a mono mic on a stereo
/// interface); averaging would attenuate that speech by the channel count.
/// Energy is smoothed across callbacks and switching requires sustained
/// dominance, so the mono stream does not hop channels on transient noise.
pub(super) struct ChannelSelector {
    chosen: usize,
    smoothed: Vec<f64>,
}

const CHANNEL_SWITCH_RATIO: f64 = 2.0;

impl ChannelSelector {
    pub(super) fn new() -> Self {
        Self {
            chosen: 0,
            smoothed: Vec::new(),
        }
    }

    fn pick(&mut self, samples: &[f32], channels: usize) -> usize {
        if channels == 1 {
            self.chosen = 0;
            return 0;
        }
        self.smoothed.resize(channels, 0.0);
        let frames = (samples.len() / channels).max(1) as f64;
        let mut energy = vec![0.0f64; channels];
        for frame in samples.chunks(channels) {
            for (index, sample) in frame.iter().enumerate() {
                energy[index] += f64::from(sample * sample);
            }
        }
        for (smoothed, total) in self.smoothed.iter_mut().zip(energy) {
            *smoothed = *smoothed * 0.9 + (total / frames) * 0.1;
        }

        self.chosen = self.chosen.min(channels - 1);
        if let Some((best, best_energy)) = self
            .smoothed
            .iter()
            .enumerate()
            .max_by(|left, right| left.1.total_cmp(right.1))
        {
            if best != self.chosen
                && *best_energy > self.smoothed[self.chosen] * CHANNEL_SWITCH_RATIO
            {
                self.chosen = best;
            }
        }
        self.chosen
    }
}

#[cfg(any(debug_assertions, feature = "qa-fixtures"))]
pub(super) fn average_f32(frame: &[f32]) -> f32 {
    frame.iter().copied().sum::<f32>() / frame.len() as f32
}

pub(super) fn push_mono_frames<I>(
    frames: I,
    buffers: &Arc<Mutex<SharedBuffers>>,
    source: CaptureSource,
) where
    I: Iterator<Item = f32>,
{
    let mut chunk = Vec::new();
    let mut square_sum = 0.0;

    for sample in frames {
        let clamped = sample.clamp(-1.0, 1.0);
        square_sum += clamped * clamped;
        chunk.push(clamped);
    }

    if chunk.is_empty() {
        return;
    }

    let rms = (square_sum / chunk.len() as f32).sqrt();
    let level = (rms * 4.0).min(1.0);

    if let Ok(mut shared) = buffers.lock() {
        shared.push(source, &chunk, level);
    }
}
