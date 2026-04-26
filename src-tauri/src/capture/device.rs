use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait};
use tauri::AppHandle;

use super::{
    model::{ActiveAudioCapture, CaptureSource, PreparedAudioInput, SharedBuffers},
    samples::build_capture_stream,
    system_loopback::{prepare_system_loopback_device, SystemAudioCapture},
};
use crate::ensure_microphone_permission;

pub(super) fn prepare_device_audio_input(app: &AppHandle) -> Result<PreparedAudioInput, String> {
    let host = cpal::default_host();
    ensure_microphone_permission(app)?;

    let mic_device = host
        .default_input_device()
        .ok_or_else(|| "No default microphone input device is available".to_string())?;
    let system_device = prepare_system_loopback_device(&host)?;

    let mic_config = mic_device
        .default_input_config()
        .map_err(|err| format!("Failed to read microphone config: {err}"))?;
    let system_config = system_device
        .device
        .default_input_config()
        .map_err(|err| format!("Failed to read system loopback config: {err}"))?;

    let mic_sample_rate = mic_config.sample_rate();
    let system_sample_rate = system_config.sample_rate();
    let buffers = Arc::new(Mutex::new(SharedBuffers::new(
        mic_sample_rate,
        system_sample_rate,
    )));

    let mic_stream = build_capture_stream(
        mic_device,
        mic_config,
        Arc::clone(&buffers),
        CaptureSource::Mic,
    )?;
    let system_stream = build_capture_stream(
        system_device.device,
        system_config,
        Arc::clone(&buffers),
        CaptureSource::System,
    )?;
    let system_capture = SystemAudioCapture::new(system_stream, system_device.tap);

    Ok(PreparedAudioInput {
        mic_sample_rate,
        system_sample_rate,
        buffers,
        audio_capture: ActiveAudioCapture::Devices {
            _mic_stream: mic_stream,
            _system_capture: system_capture,
        },
    })
}
