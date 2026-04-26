use std::{
    ffi::{c_void, CStr},
    mem,
    ptr::{self, NonNull},
    thread,
    time::Duration,
};

use cpal::{
    traits::{DeviceTrait, HostTrait},
    Device, Stream,
};
use objc2::AnyThread;
use objc2_core_audio::{
    kAudioAggregateDeviceIsPrivateKey, kAudioAggregateDeviceNameKey,
    kAudioAggregateDevicePropertyTapList, kAudioAggregateDeviceUIDKey, kAudioHardwareNoError,
    kAudioObjectPropertyElementMain, kAudioObjectPropertyScopeGlobal, kAudioTapPropertyUID,
    AudioHardwareCreateAggregateDevice, AudioHardwareCreateProcessTap,
    AudioHardwareDestroyAggregateDevice, AudioHardwareDestroyProcessTap,
    AudioObjectGetPropertyData, AudioObjectID, AudioObjectPropertyAddress,
    AudioObjectSetPropertyData, CATapDescription, CATapMuteBehavior,
};
use objc2_core_foundation::{CFArray, CFBoolean, CFDictionary, CFString, CFType};
use objc2_foundation::{NSArray, NSNumber, NSString};

use crate::app::now_ms;

type RetainedAudioDictionary = objc2_core_foundation::CFRetained<CFDictionary<CFType, CFType>>;

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
    let tap = CoreAudioSystemTap::create()?;

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

pub(super) struct CoreAudioSystemTap {
    tap_id: AudioObjectID,
    aggregate_device_id: AudioObjectID,
}

impl CoreAudioSystemTap {
    fn create() -> Result<Self, String> {
        let empty_processes = NSArray::<NSNumber>::new();
        let description = unsafe {
            CATapDescription::initMonoGlobalTapButExcludeProcesses(
                CATapDescription::alloc(),
                &empty_processes,
            )
        };
        unsafe {
            description.setName(&NSString::from_str(SYSTEM_CAPTURE_DEVICE_NAME));
            description.setPrivate(true);
            description.setMuteBehavior(CATapMuteBehavior::Unmuted);
        }

        let mut tap_id = 0;
        let status = unsafe { AudioHardwareCreateProcessTap(Some(&description), &mut tap_id) };
        if status != kAudioHardwareNoError {
            return Err(format!(
                "Failed to create macOS system audio tap: {}",
                coreaudio_status(status)
            ));
        }

        let aggregate_uid = format!("com.just-notes.system-audio.{}", now_ms()?);
        let aggregate_description =
            create_aggregate_device_description(SYSTEM_CAPTURE_DEVICE_NAME, &aggregate_uid)?;

        let mut aggregate_device_id = 0;
        let status = unsafe {
            AudioHardwareCreateAggregateDevice(
                aggregate_description.as_ref(),
                NonNull::from(&mut aggregate_device_id),
            )
        };
        if status != kAudioHardwareNoError {
            unsafe {
                AudioHardwareDestroyProcessTap(tap_id);
            }
            return Err(format!(
                "Failed to create macOS system audio aggregate device: {}",
                coreaudio_status(status)
            ));
        }

        if let Err(err) = attach_tap_to_aggregate_device(tap_id, aggregate_device_id) {
            unsafe {
                AudioHardwareDestroyAggregateDevice(aggregate_device_id);
                AudioHardwareDestroyProcessTap(tap_id);
            }
            return Err(err);
        }

        Ok(Self {
            tap_id,
            aggregate_device_id,
        })
    }
}

impl Drop for CoreAudioSystemTap {
    fn drop(&mut self) {
        unsafe {
            AudioHardwareDestroyAggregateDevice(self.aggregate_device_id);
            AudioHardwareDestroyProcessTap(self.tap_id);
        }
    }
}

fn create_aggregate_device_description(
    device_name: &str,
    aggregate_uid: &str,
) -> Result<RetainedAudioDictionary, String> {
    let name_key = cf_audio_key(kAudioAggregateDeviceNameKey)?;
    let uid_key = cf_audio_key(kAudioAggregateDeviceUIDKey)?;
    let private_key = cf_audio_key(kAudioAggregateDeviceIsPrivateKey)?;
    let name = CFString::from_str(device_name);
    let uid = CFString::from_str(aggregate_uid);
    let private = CFBoolean::new(true);

    Ok(CFDictionary::<CFType, CFType>::from_slices(
        &[name_key.as_ref(), uid_key.as_ref(), private_key.as_ref()],
        &[name.as_ref(), uid.as_ref(), private.as_ref()],
    ))
}

fn attach_tap_to_aggregate_device(
    tap_id: AudioObjectID,
    aggregate_device_id: AudioObjectID,
) -> Result<(), String> {
    let mut tap_uid_ref: *const CFString = ptr::null();
    let mut property_size = mem::size_of::<*const CFString>() as u32;
    let mut property_address = audio_property_address(kAudioTapPropertyUID);
    let status = unsafe {
        AudioObjectGetPropertyData(
            tap_id,
            NonNull::from(&mut property_address),
            0,
            ptr::null(),
            NonNull::from(&mut property_size),
            NonNull::new((&mut tap_uid_ref as *mut *const CFString).cast::<c_void>())
                .expect("tap UID output pointer cannot be null"),
        )
    };
    if status != kAudioHardwareNoError {
        return Err(format!(
            "Failed to read macOS system audio tap UID: {}",
            coreaudio_status(status)
        ));
    }

    let tap_uid = unsafe {
        tap_uid_ref
            .as_ref()
            .ok_or_else(|| "CoreAudio returned an empty system audio tap UID".to_string())?
    };
    let tap_list = CFArray::<CFString>::from_objects(&[tap_uid]);
    let mut tap_list_ref: *const CFArray<CFString> = tap_list.as_ref();
    let mut property_address = audio_property_address(kAudioAggregateDevicePropertyTapList);
    let property_size = mem::size_of::<*const CFArray<CFString>>() as u32;
    let status = unsafe {
        AudioObjectSetPropertyData(
            aggregate_device_id,
            NonNull::from(&mut property_address),
            0,
            ptr::null(),
            property_size,
            NonNull::new((&mut tap_list_ref as *mut *const CFArray<CFString>).cast::<c_void>())
                .expect("tap list pointer cannot be null"),
        )
    };
    if status != kAudioHardwareNoError {
        return Err(format!(
            "Failed to attach macOS system audio tap to aggregate device: {}",
            coreaudio_status(status)
        ));
    }

    Ok(())
}

fn audio_property_address(selector: u32) -> AudioObjectPropertyAddress {
    AudioObjectPropertyAddress {
        mSelector: selector,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain,
    }
}

fn cf_audio_key(key: &CStr) -> Result<objc2_core_foundation::CFRetained<CFString>, String> {
    let key = key
        .to_str()
        .map_err(|err| format!("Invalid CoreAudio dictionary key: {err}"))?;
    Ok(CFString::from_str(key))
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

fn coreaudio_status(status: i32) -> String {
    let bytes = status.to_be_bytes();
    if bytes
        .iter()
        .all(|byte| byte.is_ascii_graphic() || *byte == b' ')
    {
        format!("{status} ('{}')", String::from_utf8_lossy(&bytes))
    } else {
        status.to_string()
    }
}
