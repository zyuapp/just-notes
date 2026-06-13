use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
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
}

impl SharedBuffers {
    pub(super) fn new(mic_sample_rate: u32, system_sample_rate: u32) -> Self {
        Self {
            mic: RollingChannel::new(mic_sample_rate),
            system: RollingChannel::new(system_sample_rate),
        }
    }
}

pub(crate) struct RollingChannel {
    samples: VecDeque<f32>,
    base_index: u64,
    sample_rate: u32,
    max_samples: usize,
    pub(crate) level: f32,
}

impl RollingChannel {
    fn new(sample_rate: u32) -> Self {
        let max_samples = ((sample_rate as u64 * MAX_ROLLING_BUFFER_MS) / 1000).max(1) as usize;
        Self {
            samples: VecDeque::with_capacity(max_samples.min(sample_rate as usize * 10)),
            base_index: 0,
            sample_rate,
            max_samples,
            level: 0.0,
        }
    }

    pub(super) fn push(&mut self, chunk: &[f32], level: f32) {
        self.samples.extend(chunk.iter().copied());
        let overflow = self.samples.len().saturating_sub(self.max_samples);
        if overflow > 0 {
            self.samples.drain(0..overflow);
            self.base_index += overflow as u64;
        }
        self.level = smooth_level(self.level, level);
    }

    pub(crate) fn available_end_index(&self) -> u64 {
        self.base_index + self.samples.len() as u64
    }

    pub(crate) fn earliest_index(&self) -> u64 {
        self.base_index
    }

    pub(crate) fn sample_rate(&self) -> u32 {
        self.sample_rate
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
