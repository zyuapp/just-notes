use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use super::{
    model::{
        ActiveAudioCapture, CaptureSource, FixtureAudioCapture, PreparedAudioInput, SharedBuffers,
    },
    samples::{average_f32, push_mono_frames},
};
use crate::transcription::samples_to_ms;

const FIXTURE_CHUNK_MS: u64 = 50;

struct FixtureTrack {
    sample_rate: u32,
    samples: Vec<f32>,
}

pub(super) fn prepare_fixture_audio_input(
    mic_path: PathBuf,
    system_path: PathBuf,
) -> Result<PreparedAudioInput, String> {
    let mic_track = read_fixture_track(&mic_path)?;
    let system_track = read_fixture_track(&system_path)?;
    let buffers = Arc::new(Mutex::new(SharedBuffers::new(
        mic_track.sample_rate,
        system_track.sample_rate,
    )));
    let should_stop = Arc::new(AtomicBool::new(false));
    let workers = vec![
        spawn_fixture_audio_worker(
            mic_track,
            Arc::clone(&buffers),
            CaptureSource::Mic,
            Arc::clone(&should_stop),
        ),
        spawn_fixture_audio_worker(
            system_track,
            Arc::clone(&buffers),
            CaptureSource::System,
            Arc::clone(&should_stop),
        ),
    ];

    Ok(PreparedAudioInput {
        mic_sample_rate: read_wav_sample_rate(&mic_path)?,
        system_sample_rate: read_wav_sample_rate(&system_path)?,
        buffers,
        audio_capture: ActiveAudioCapture::Fixture(FixtureAudioCapture {
            should_stop,
            workers,
        }),
    })
}

fn read_wav_sample_rate(path: &Path) -> Result<u32, String> {
    let reader = hound::WavReader::open(path)
        .map_err(|err| format!("Failed to open fixture audio {}: {err}", path.display()))?;
    Ok(reader.spec().sample_rate)
}

fn read_fixture_track(path: &Path) -> Result<FixtureTrack, String> {
    let mut reader = hound::WavReader::open(path).map_err(|err| {
        format!(
            "Failed to open fixture audio {}. Create qa-mic.wav and qa-system.wav in ~/.just-notes/fixtures: {err}",
            path.display()
        )
    })?;
    let spec = reader.spec();
    if spec.channels == 0 {
        return Err(format!("Fixture audio {} has no channels", path.display()));
    }

    let channels = spec.channels as usize;
    let raw_samples = read_fixture_samples(&mut reader, spec, path)?;
    let samples = raw_samples
        .chunks(channels)
        .map(average_f32)
        .collect::<Vec<_>>();

    Ok(FixtureTrack {
        sample_rate: spec.sample_rate,
        samples,
    })
}

fn read_fixture_samples<R: std::io::Read>(
    reader: &mut hound::WavReader<R>,
    spec: hound::WavSpec,
    path: &Path,
) -> Result<Vec<f32>, String> {
    let raw_samples = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .map(|sample| {
                sample.map(|value| value.clamp(-1.0, 1.0)).map_err(|err| {
                    format!("Failed to read fixture audio {}: {err}", path.display())
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
        hound::SampleFormat::Int if spec.bits_per_sample <= 16 => reader
            .samples::<i16>()
            .map(|sample| {
                sample
                    .map(|value| value as f32 / i16::MAX as f32)
                    .map_err(|err| {
                        format!("Failed to read fixture audio {}: {err}", path.display())
                    })
            })
            .collect::<Result<Vec<_>, _>>()?,
        hound::SampleFormat::Int => {
            let denom = ((1_i64 << (spec.bits_per_sample.saturating_sub(1) as u32)) - 1) as f32;
            reader
                .samples::<i32>()
                .map(|sample| {
                    sample
                        .map(|value| (value as f32 / denom).clamp(-1.0, 1.0))
                        .map_err(|err| {
                            format!("Failed to read fixture audio {}: {err}", path.display())
                        })
                })
                .collect::<Result<Vec<_>, _>>()?
        }
    };
    Ok(raw_samples)
}

fn spawn_fixture_audio_worker(
    track: FixtureTrack,
    buffers: Arc<Mutex<SharedBuffers>>,
    source: CaptureSource,
    should_stop: Arc<AtomicBool>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let chunk_samples = ((track.sample_rate as u64 * FIXTURE_CHUNK_MS) / 1000).max(1) as usize;
        for chunk in track.samples.chunks(chunk_samples) {
            if should_stop.load(Ordering::Relaxed) {
                break;
            }
            push_mono_frames(chunk.iter().copied(), &buffers, source);
            let chunk_ms = samples_to_ms(chunk.len() as u64, track.sample_rate);
            thread::sleep(Duration::from_millis(chunk_ms.max(1)));
        }
    })
}
