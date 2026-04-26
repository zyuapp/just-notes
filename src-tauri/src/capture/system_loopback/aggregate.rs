use std::{
    ffi::{c_void, CStr},
    mem,
    ptr::{self, NonNull},
};

use objc2_core_audio::{
    kAudioAggregateDeviceIsPrivateKey, kAudioAggregateDeviceNameKey,
    kAudioAggregateDevicePropertyTapList, kAudioAggregateDeviceUIDKey, kAudioHardwareNoError,
    kAudioObjectPropertyElementMain, kAudioObjectPropertyScopeGlobal, kAudioTapPropertyUID,
    AudioObjectGetPropertyData, AudioObjectID, AudioObjectPropertyAddress,
    AudioObjectSetPropertyData,
};
use objc2_core_foundation::{CFArray, CFBoolean, CFDictionary, CFString, CFType};

type RetainedAudioDictionary = objc2_core_foundation::CFRetained<CFDictionary<CFType, CFType>>;

pub(super) fn create_aggregate_device_description(
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

pub(super) fn attach_tap_to_aggregate_device(
    tap_id: AudioObjectID,
    aggregate_device_id: AudioObjectID,
) -> Result<(), String> {
    let tap_uid_ref = read_tap_uid(tap_id)?;
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

fn read_tap_uid(tap_id: AudioObjectID) -> Result<*const CFString, String> {
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

    Ok(tap_uid_ref)
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

pub(super) fn coreaudio_status(status: i32) -> String {
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
