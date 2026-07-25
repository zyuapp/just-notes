use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Instant,
};

#[cfg(any(debug_assertions, feature = "qa-fixtures"))]
use std::{path::PathBuf, sync::atomic::AtomicBool, thread::JoinHandle};

use cpal::Stream;

use super::system_loopback::SystemAudioCapture;

const MAX_ROLLING_BUFFER_MS: u64 = 120_000;

pub(crate) enum ActiveAudioCapture {
    Devices {
        _mic_stream: Stream,
        _system_capture: SystemAudioCapture,
    },
    #[cfg(any(debug_assertions, feature = "qa-fixtures"))]
    Fixture(FixtureAudioCapture),
}

#[cfg(any(debug_assertions, feature = "qa-fixtures"))]
pub(crate) struct FixtureAudioCapture {
    pub(super) should_stop: Arc<AtomicBool>,
    pub(super) workers: Vec<JoinHandle<()>>,
}

pub(crate) enum RecordingInputMode {
    Devices,
    #[cfg(any(debug_assertions, feature = "qa-fixtures"))]
    Fixture {
        mic_path: PathBuf,
        system_path: PathBuf,
    },
}

pub(crate) struct PreparedAudioInput {
    pub(crate) mic_sample_rate: u32,
    pub(crate) system_sample_rate: u32,
    pub(crate) buffers: Arc<Mutex<SharedBuffers>>,
    pub(crate) audio_capture: ActiveAudioCapture,
}

pub(crate) struct SharedBuffers {
    pub(crate) mic: RollingChannel,
    pub(crate) system: RollingChannel,
    origin: Option<Instant>,
}

impl SharedBuffers {
    pub(super) fn new(mic_sample_rate: u32, system_sample_rate: u32) -> Self {
        Self {
            mic: RollingChannel::new(mic_sample_rate),
            system: RollingChannel::new(system_sample_rate),
            origin: None,
        }
    }

    /// Appends captured audio, anchoring both channels to the first sample that
    /// arrives on either of them. The two input streams start independently, so
    /// without a shared origin each channel's sample index 0 would sit at a
    /// different instant and their timelines would not be comparable.
    pub(super) fn push(&mut self, source: CaptureSource, chunk: &[f32], level: f32) {
        let origin = *self.origin.get_or_insert_with(Instant::now);
        match source {
            CaptureSource::Mic => self.mic.push(chunk, level, origin),
            CaptureSource::System => self.system.push(chunk, level, origin),
        }
    }
}

pub(crate) struct RollingChannel {
    samples: VecDeque<f32>,
    base_index: u64,
    max_samples: usize,
    start_offset_ms: Option<u64>,
    pub(crate) level: f32,
}

impl RollingChannel {
    fn new(sample_rate: u32) -> Self {
        let max_samples = ((sample_rate as u64 * MAX_ROLLING_BUFFER_MS) / 1000).max(1) as usize;
        Self {
            samples: VecDeque::with_capacity(max_samples.min(sample_rate as usize * 10)),
            base_index: 0,
            max_samples,
            start_offset_ms: None,
            level: 0.0,
        }
    }

    fn push(&mut self, chunk: &[f32], level: f32, origin: Instant) {
        self.start_offset_ms
            .get_or_insert_with(|| origin.elapsed().as_millis() as u64);
        self.samples.extend(chunk.iter().copied());
        let overflow = self.samples.len().saturating_sub(self.max_samples);
        if overflow > 0 {
            self.samples.drain(0..overflow);
            self.base_index += overflow as u64;
        }
        self.level = smooth_level(self.level, level);
    }

    /// Where sample index 0 sits on the shared capture timeline. Resolution is
    /// one input callback, so this measures stream start-up skew, not the
    /// device latency inside a single callback.
    pub(crate) fn start_offset_ms(&self) -> u64 {
        self.start_offset_ms.unwrap_or(0)
    }

    pub(crate) fn available_end_index(&self) -> u64 {
        self.base_index + self.samples.len() as u64
    }

    pub(crate) fn earliest_index(&self) -> u64 {
        self.base_index
    }

    pub(crate) fn window(&self, start_index: u64, end_index: u64) -> Option<Vec<f32>> {
        if start_index < self.base_index || end_index > self.available_end_index() {
            return None;
        }
        let start = (start_index - self.base_index) as usize;
        let end = (end_index - self.base_index) as usize;
        if end <= start {
            return None;
        }
        Some(self.samples.range(start..end).copied().collect())
    }
}

#[derive(Clone, Copy)]
pub(super) enum CaptureSource {
    Mic,
    System,
}

fn smooth_level(current: f32, next: f32) -> f32 {
    (current * 0.72) + (next * 0.28)
}
