mod aggregate;
mod tap;

use std::{thread, time::Duration};

use cpal::{
    traits::{DeviceTrait, HostTrait},
    Device, Stream,
};

use tap::CoreAudioSystemTap;

const SYSTEM_CAPTURE_DEVICE_NAME: &str = "Just Notes System Audio";

pub(super) struct PreparedSystemLoopbackDevice {
    pub(super) device: Device,
    pub(super) tap: CoreAudioSystemTap,
}

pub(crate) struct SystemAudioCapture {
    stream: Option<Stream>,
    tap: Option<CoreAudioSystemTap>,
}

impl SystemAudioCapture {
    pub(super) fn new(stream: Stream, tap: CoreAudioSystemTap) -> Self {
        Self {
            stream: Some(stream),
            tap: Some(tap),
        }
    }

    pub(super) fn stream(&self) -> &Stream {
        self.stream
            .as_ref()
            .expect("system capture stream should exist until drop")
    }
}

impl Drop for SystemAudioCapture {
    fn drop(&mut self) {
        self.stream.take();
        self.tap.take();
    }
}

pub(super) fn prepare_system_loopback_device(
    host: &cpal::Host,
) -> Result<PreparedSystemLoopbackDevice, String> {
    let tap = CoreAudioSystemTap::create(SYSTEM_CAPTURE_DEVICE_NAME)?;

    for _ in 0..20 {
        if let Some(device) = find_system_loopback_device(host)? {
            return Ok(PreparedSystemLoopbackDevice { device, tap });
        }
        thread::sleep(Duration::from_millis(100));
    }

    Err(format!(
        "Created macOS system audio tap, but CPAL did not expose the {SYSTEM_CAPTURE_DEVICE_NAME} input device"
    ))
}

fn find_system_loopback_device(host: &cpal::Host) -> Result<Option<Device>, String> {
    let devices = host
        .input_devices()
        .map_err(|err| format!("Failed to enumerate audio input devices: {err}"))?;
    for device in devices {
        let Ok(description) = device.description() else {
            continue;
        };
        if description.name() == SYSTEM_CAPTURE_DEVICE_NAME {
            return Ok(Some(device));
        }
    }
    Ok(None)
}
