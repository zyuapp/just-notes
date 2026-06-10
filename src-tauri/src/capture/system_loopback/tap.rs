use objc2::AnyThread;
use objc2_core_audio::{
    kAudioHardwareNoError, AudioHardwareCreateAggregateDevice, AudioHardwareCreateProcessTap,
    AudioHardwareDestroyAggregateDevice, AudioHardwareDestroyProcessTap, AudioObjectID,
    CATapDescription, CATapMuteBehavior,
};
use objc2_foundation::{NSArray, NSNumber, NSString};
use std::ptr::NonNull;

use super::aggregate::{
    attach_tap_to_aggregate_device, coreaudio_status, create_aggregate_device_description,
};
use crate::app::now_ms;

pub(in crate::capture) struct CoreAudioSystemTap {
    tap_id: AudioObjectID,
    aggregate_device_id: AudioObjectID,
}

impl CoreAudioSystemTap {
    pub(super) fn create(device_name: &str) -> Result<Self, String> {
        let tap_id = create_process_tap(device_name)?;
        let aggregate_device_id = match create_aggregate_device(device_name) {
            Ok(aggregate_device_id) => aggregate_device_id,
            Err(err) => {
                unsafe {
                    AudioHardwareDestroyProcessTap(tap_id);
                }
                return Err(err);
            }
        };

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

fn create_process_tap(device_name: &str) -> Result<AudioObjectID, String> {
    let empty_processes = NSArray::<NSNumber>::new();
    let description = unsafe {
        CATapDescription::initMonoGlobalTapButExcludeProcesses(
            CATapDescription::alloc(),
            &empty_processes,
        )
    };
    unsafe {
        description.setName(&NSString::from_str(device_name));
        description.setPrivate(true);
        description.setMuteBehavior(CATapMuteBehavior::Unmuted);
    }

    let mut tap_id = 0;
    let status = unsafe { AudioHardwareCreateProcessTap(Some(&description), &mut tap_id) };
    if status != kAudioHardwareNoError {
        return Err(format!(
            "Failed to create macOS system audio tap: {}. If system audio access was denied, \
             allow Just Notes under System Settings → Privacy & Security → Screen & System \
             Audio Recording, then try again.",
            coreaudio_status(status)
        ));
    }
    Ok(tap_id)
}

fn create_aggregate_device(device_name: &str) -> Result<AudioObjectID, String> {
    let aggregate_uid = format!("com.just-notes.system-audio.{}", now_ms()?);
    let aggregate_description = create_aggregate_device_description(device_name, &aggregate_uid)?;

    let mut aggregate_device_id = 0;
    let status = unsafe {
        AudioHardwareCreateAggregateDevice(
            aggregate_description.as_ref(),
            NonNull::from(&mut aggregate_device_id),
        )
    };
    if status != kAudioHardwareNoError {
        return Err(format!(
            "Failed to create macOS system audio aggregate device: {}",
            coreaudio_status(status)
        ));
    }

    Ok(aggregate_device_id)
}
