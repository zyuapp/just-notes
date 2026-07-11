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
            move |data: &[f32], _| push_f32_samples(data, config.channels, &buffers, source),
            err_fn,
            None,
        ),
        SampleFormat::I16 => device.build_input_stream(
            &config,
            move |data: &[i16], _| push_i16_samples(data, config.channels, &buffers, source),
            err_fn,
            None,
        ),
        SampleFormat::U16 => device.build_input_stream(
            &config,
            move |data: &[u16], _| push_u16_samples(data, config.channels, &buffers, source),
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
) {
    let channels = channels.max(1) as usize;
    let channel = loudest_channel(samples, channels);
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
) {
    let converted: Vec<f32> = samples
        .iter()
        .map(|sample| *sample as f32 / i16::MAX as f32)
        .collect();
    push_f32_samples(&converted, channels, buffers, source);
}

fn push_u16_samples(
    samples: &[u16],
    channels: u16,
    buffers: &Arc<Mutex<SharedBuffers>>,
    source: CaptureSource,
) {
    let converted: Vec<f32> = samples
        .iter()
        .map(|sample| (*sample as f32 - 32768.0) / 32768.0)
        .collect();
    push_f32_samples(&converted, channels, buffers, source);
}

/// Index of the channel with the most energy in this callback chunk. Devices
/// can expose multiple channels with voice on only one (a mono mic on a
/// stereo interface); averaging would attenuate that speech by the channel
/// count, so the mono stream follows the loudest channel instead.
fn loudest_channel(samples: &[f32], channels: usize) -> usize {
    if channels == 1 {
        return 0;
    }
    let mut energy = vec![0.0f64; channels];
    for frame in samples.chunks(channels) {
        for (index, sample) in frame.iter().enumerate() {
            energy[index] += f64::from(sample * sample);
        }
    }
    energy
        .iter()
        .enumerate()
        .max_by(|left, right| left.1.total_cmp(right.1))
        .map(|(index, _)| index)
        .unwrap_or(0)
}

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
        match source {
            CaptureSource::Mic => shared.mic.push(&chunk, level),
            CaptureSource::System => shared.system.push(&chunk, level),
        }
    }
}
