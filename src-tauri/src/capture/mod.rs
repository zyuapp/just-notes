mod device;
#[cfg(any(debug_assertions, feature = "qa-fixtures"))]
mod fixture;
mod model;
mod permission;
mod samples;
mod system_loopback;

use cpal::traits::StreamTrait;
#[cfg(any(debug_assertions, feature = "qa-fixtures"))]
use std::sync::atomic::Ordering;
use tauri::AppHandle;

#[cfg(any(debug_assertions, feature = "qa-fixtures"))]
use fixture::prepare_fixture_audio_input;

pub(crate) use model::{ActiveAudioCapture, PreparedAudioInput, RecordingInputMode, SharedBuffers};

use device::prepare_device_audio_input;

pub(crate) fn prepare_audio_input(
    app: &AppHandle,
    input_mode: RecordingInputMode,
) -> Result<PreparedAudioInput, String> {
    match input_mode {
        RecordingInputMode::Devices => prepare_device_audio_input(app),
        #[cfg(any(debug_assertions, feature = "qa-fixtures"))]
        RecordingInputMode::Fixture {
            mic_path,
            system_path,
        } => prepare_fixture_audio_input(mic_path, system_path),
    }
}

pub(crate) fn start_audio_capture(audio_capture: &mut ActiveAudioCapture) -> Result<(), String> {
    match audio_capture {
        ActiveAudioCapture::Devices {
            _mic_stream,
            _system_capture,
        } => {
            _mic_stream
                .play()
                .map_err(|err| format!("Failed to start microphone stream: {err}"))?;
            _system_capture
                .stream()
                .play()
                .map_err(|err| format!("Failed to start system loopback stream: {err}"))
        }
        #[cfg(any(debug_assertions, feature = "qa-fixtures"))]
        ActiveAudioCapture::Fixture(_) => Ok(()),
    }
}

pub(crate) fn stop_audio_capture(audio_capture: ActiveAudioCapture) {
    match audio_capture {
        ActiveAudioCapture::Devices {
            _mic_stream,
            _system_capture,
        } => {
            drop(_mic_stream);
            drop(_system_capture);
        }
        #[cfg(any(debug_assertions, feature = "qa-fixtures"))]
        ActiveAudioCapture::Fixture(fixture) => {
            fixture.should_stop.store(true, Ordering::Relaxed);
            for worker in fixture.workers {
                let _ = worker.join();
            }
        }
    }
}
