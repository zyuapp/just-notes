use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use super::{push_f32_samples, push_mono_frames, CaptureSource, ChannelSelector, SharedBuffers};

fn mic_mono_after_chunks(chunks: &[Vec<f32>], channels: u16) -> Vec<f32> {
    let buffers = Arc::new(Mutex::new(SharedBuffers::new(48_000, 48_000)));
    let mut selector = ChannelSelector::new();
    for chunk in chunks {
        push_f32_samples(chunk, channels, &buffers, CaptureSource::Mic, &mut selector);
    }
    let shared = buffers.lock().expect("capture buffers lock");
    let end = shared.mic.available_end_index();
    shared.mic.window(0, end).unwrap_or_default()
}

fn mic_mono_after(interleaved: &[f32], channels: u16) -> Vec<f32> {
    mic_mono_after_chunks(&[interleaved.to_vec()], channels)
}

fn stereo_chunk(left: f32, right: f32, frames: usize) -> Vec<f32> {
    std::iter::repeat([left, right])
        .take(frames)
        .flatten()
        .collect()
}

fn rms(samples: &[f32]) -> f32 {
    (samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len() as f32).sqrt()
}

// The mic and system streams start independently, so the later one's sample
// index 0 must carry its distance from the first audio seen on either channel.
#[test]
fn a_late_starting_channel_records_its_skew_from_the_shared_origin() {
    let buffers = Arc::new(Mutex::new(SharedBuffers::new(48_000, 48_000)));
    push_mono_frames([0.1f32; 64].into_iter(), &buffers, CaptureSource::Mic);
    thread::sleep(Duration::from_millis(60));
    push_mono_frames([0.1f32; 64].into_iter(), &buffers, CaptureSource::System);
    push_mono_frames([0.1f32; 64].into_iter(), &buffers, CaptureSource::Mic);

    let shared = buffers.lock().expect("capture buffers lock");
    assert_eq!(shared.mic.start_offset_ms(), 0);
    let skew = shared.system.start_offset_ms();
    assert!(
        skew >= 50,
        "system start offset is {skew}ms after a 60ms delay"
    );
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
    let mono = mic_mono_after(&stereo_chunk(0.5, 0.0, 100), 2);
    let level = rms(&mono);
    assert!(
        level >= 0.45,
        "mono RMS is {level} for a 0.5-amplitude single live channel"
    );
}

#[test]
fn downmix_locks_onto_the_live_channel_regardless_of_position() {
    let mono = mic_mono_after(&stereo_chunk(0.0, 0.5, 100), 2);
    let level = rms(&mono);
    assert!(
        level >= 0.45,
        "mono RMS is {level} for voice on the second channel"
    );
}

// A single loud transient on the idle channel must not flip the selection
// away from the channel carrying sustained speech.
#[test]
fn downmix_does_not_hop_channels_on_a_transient() {
    let mut chunks = vec![stereo_chunk(0.5, 0.0, 100); 10];
    chunks.push(stereo_chunk(0.0, 0.9, 100));
    let mono = mic_mono_after_chunks(&chunks, 2);

    let transient_span = &mono[mono.len() - 100..];
    assert!(
        rms(transient_span) < 0.05,
        "selection hopped to the transient channel (span RMS {})",
        rms(transient_span)
    );
}
