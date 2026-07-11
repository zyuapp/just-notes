use std::sync::{Arc, Mutex};

use super::{push_f32_samples, CaptureSource, SharedBuffers};

fn mic_mono_after(interleaved: &[f32], channels: u16) -> Vec<f32> {
    let buffers = Arc::new(Mutex::new(SharedBuffers::new(48_000, 48_000)));
    push_f32_samples(interleaved, channels, &buffers, CaptureSource::Mic);
    let shared = buffers.lock().expect("capture buffers lock");
    let end = shared.mic.available_end_index();
    shared.mic.window(0, end).unwrap_or_default()
}

fn rms(samples: &[f32]) -> f32 {
    (samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len() as f32).sqrt()
}

#[test]
fn stereo_downmix_preserves_identical_channels() {
    let mono = mic_mono_after(&[0.5, 0.5, -0.5, -0.5], 2);
    assert_eq!(mono, vec![0.5, -0.5]);
}

// A stereo device carrying voice on one channel (single mic on a USB
// interface) should not lose half its level in the mono downmix — the loss
// stacks with the live speech gate and drops whole segments.
#[test]
fn stereo_downmix_keeps_single_live_channel_level() {
    let interleaved: Vec<f32> = std::iter::repeat([0.5, 0.0]).take(100).flatten().collect();
    let mono = mic_mono_after(&interleaved, 2);
    let level = rms(&mono);
    assert!(
        level >= 0.45,
        "mono RMS is {level} for a 0.5-amplitude single live channel"
    );
}
