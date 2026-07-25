use std::{
    fs::File,
    io::BufWriter,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use hound::{SampleFormat, WavSpec, WavWriter};

use crate::{
    capture::{CaptureSource, SharedBuffers},
    threads::RecordingAudioPaths,
    transcription::samples_for_ms,
};

const AUDIO_SINK_POLL_MS: u64 = 250;

type ChannelWavWriter = WavWriter<BufWriter<File>>;
type SinkWorker = JoinHandle<Result<(), String>>;

pub(super) struct AudioSink {
    should_stop: Arc<AtomicBool>,
    worker: Option<SinkWorker>,
}

impl AudioSink {
    pub(super) fn stop(mut self) -> Result<(), String> {
        self.should_stop.store(true, Ordering::Relaxed);
        match self.worker.take() {
            Some(worker) => worker
                .join()
                .map_err(|_| "Audio file writer crashed".to_string())?,
            None => Ok(()),
        }
    }
}

pub(super) fn spawn_audio_sink(
    audio_paths: &RecordingAudioPaths,
    buffers: Arc<Mutex<SharedBuffers>>,
    mic_sample_rate: u32,
    system_sample_rate: u32,
) -> Result<AudioSink, String> {
    let mic = ChannelSink::create(audio_paths.mic_path(), mic_sample_rate, CaptureSource::Mic)?;
    let system = ChannelSink::create(
        audio_paths.system_path(),
        system_sample_rate,
        CaptureSource::System,
    )?;
    let should_stop = Arc::new(AtomicBool::new(false));
    let stop_flag = Arc::clone(&should_stop);
    let worker = thread::spawn(move || run_audio_sink(buffers, mic, system, stop_flag));

    Ok(AudioSink {
        should_stop,
        worker: Some(worker),
    })
}

struct ChannelSink {
    writer: ChannelWavWriter,
    cursor: u64,
    sample_rate: u32,
    channel: CaptureSource,
    lead_in_written: bool,
}

impl ChannelSink {
    fn create(path: &Path, sample_rate: u32, channel: CaptureSource) -> Result<Self, String> {
        let spec = WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let writer = WavWriter::create(path, spec)
            .map_err(|err| format!("Failed to create {}: {err}", path.display()))?;
        Ok(Self {
            writer,
            cursor: 0,
            sample_rate,
            channel,
            lead_in_written: false,
        })
    }

    fn drain(&mut self, buffers: &Arc<Mutex<SharedBuffers>>) -> Result<(), String> {
        let audio = buffers
            .lock()
            .map_err(|_| "Audio buffer lock was poisoned".to_string())?
            .take_new(self.channel, &mut self.cursor);

        if audio.samples.is_empty() {
            return Ok(());
        }
        self.write_lead_in(audio.start_offset_ms)?;
        for sample in audio.samples {
            let value = (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16;
            self.writer
                .write_sample(value)
                .map_err(|err| format!("Failed to write recording audio: {err}"))?;
        }
        Ok(())
    }

    /// Silence covering the gap between the shared capture origin and this
    /// channel's first sample, so a position in the file means the same instant
    /// as the matching transcript timestamp.
    fn write_lead_in(&mut self, start_offset_ms: u64) -> Result<(), String> {
        if self.lead_in_written {
            return Ok(());
        }
        self.lead_in_written = true;
        for _ in 0..samples_for_ms(self.sample_rate, start_offset_ms) {
            self.writer
                .write_sample(0i16)
                .map_err(|err| format!("Failed to write recording audio: {err}"))?;
        }
        Ok(())
    }

    fn finalize(self) -> Result<(), String> {
        self.writer
            .finalize()
            .map_err(|err| format!("Failed to finish the recording audio file: {err}"))
    }
}

fn run_audio_sink(
    buffers: Arc<Mutex<SharedBuffers>>,
    mut mic: ChannelSink,
    mut system: ChannelSink,
    should_stop: Arc<AtomicBool>,
) -> Result<(), String> {
    loop {
        let stopping = should_stop.load(Ordering::Relaxed);
        mic.drain(&buffers)?;
        system.drain(&buffers)?;
        if stopping {
            break;
        }
        thread::sleep(Duration::from_millis(AUDIO_SINK_POLL_MS));
    }

    mic.finalize()?;
    system.finalize()
}
