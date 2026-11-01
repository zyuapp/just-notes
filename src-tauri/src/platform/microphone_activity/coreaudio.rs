use std::{
    ffi::c_void,
    mem,
    ptr::{self, NonNull},
};

use objc2_core_audio::{
    kAudioHardwareNoError, kAudioHardwarePropertyProcessObjectList,
    kAudioObjectPropertyElementMain, kAudioObjectPropertyScopeGlobal, kAudioObjectSystemObject,
    kAudioProcessPropertyBundleID, kAudioProcessPropertyIsRunningInput, kAudioProcessPropertyPID,
    AudioObjectAddPropertyListener, AudioObjectGetPropertyData, AudioObjectGetPropertyDataSize,
    AudioObjectID, AudioObjectPropertyAddress, AudioObjectPropertySelector,
    AudioObjectRemovePropertyListener,
};
use objc2_core_foundation::{CFRetained, CFString};

pub(super) fn process_objects() -> Result<Vec<AudioObjectID>, String> {
    let mut address = property_address(kAudioHardwarePropertyProcessObjectList);
    let mut size = 0u32;
    let status = unsafe {
        AudioObjectGetPropertyDataSize(
            kAudioObjectSystemObject as AudioObjectID,
            NonNull::from(&mut address),
            0,
            ptr::null(),
            NonNull::from(&mut size),
        )
    };
    if status != kAudioHardwareNoError {
        return Err(format!("CoreAudio process list size failed: {status}"));
    }
    let count = size as usize / mem::size_of::<AudioObjectID>();
    let mut objects = vec![0 as AudioObjectID; count];
    if count == 0 {
        return Ok(objects);
    }
    let status = unsafe {
        AudioObjectGetPropertyData(
            kAudioObjectSystemObject as AudioObjectID,
            NonNull::from(&mut address),
            0,
            ptr::null(),
            NonNull::from(&mut size),
            NonNull::new(objects.as_mut_ptr().cast::<c_void>()).expect("vector pointer"),
        )
    };
    if status != kAudioHardwareNoError {
        return Err(format!("CoreAudio process list failed: {status}"));
    }
    objects.truncate(size as usize / mem::size_of::<AudioObjectID>());
    Ok(objects)
}

pub(super) fn is_running_input(object: AudioObjectID) -> bool {
    scalar::<u32>(object, kAudioProcessPropertyIsRunningInput).is_some_and(|running| running != 0)
}

pub(super) fn pid(object: AudioObjectID) -> Option<i32> {
    scalar::<i32>(object, kAudioProcessPropertyPID).filter(|pid| *pid > 0)
}

pub(super) fn bundle_id(object: AudioObjectID) -> Option<String> {
    let mut address = property_address(kAudioProcessPropertyBundleID);
    let mut value: *const CFString = ptr::null();
    let mut size = mem::size_of::<*const CFString>() as u32;
    let status = unsafe {
        AudioObjectGetPropertyData(
            object,
            NonNull::from(&mut address),
            0,
            ptr::null(),
            NonNull::from(&mut size),
            NonNull::from(&mut value).cast::<c_void>(),
        )
    };
    let value = NonNull::new(value.cast_mut())?;
    // The property follows the Copy rule, so the caller owns the string.
    let value = unsafe { CFRetained::from_raw(value) };
    if status != kAudioHardwareNoError {
        return None;
    }
    Some(value.to_string()).filter(|bundle_id| !bundle_id.is_empty())
}

pub(super) fn listen_to_process_list(enabled: bool) {
    set_listener(
        kAudioObjectSystemObject as AudioObjectID,
        kAudioHardwarePropertyProcessObjectList,
        enabled,
    );
}

pub(super) fn listen_to_input_state(object: AudioObjectID, enabled: bool) {
    set_listener(object, kAudioProcessPropertyIsRunningInput, enabled);
}

fn set_listener(object: AudioObjectID, selector: AudioObjectPropertySelector, enabled: bool) {
    let mut address = property_address(selector);
    // A process object can vanish before its listener is removed; CoreAudio
    // drops listeners with the object, so a failed removal needs no handling.
    let status = unsafe {
        if enabled {
            AudioObjectAddPropertyListener(
                object,
                NonNull::from(&mut address),
                Some(on_property_changed),
                ptr::null_mut(),
            )
        } else {
            AudioObjectRemovePropertyListener(
                object,
                NonNull::from(&mut address),
                Some(on_property_changed),
                ptr::null_mut(),
            )
        }
    };
    if enabled && status != kAudioHardwareNoError {
        eprintln!("microphone activity listener on {object} failed: {status}");
    }
}

unsafe extern "C-unwind" fn on_property_changed(
    _object: AudioObjectID,
    _count: u32,
    _addresses: NonNull<AudioObjectPropertyAddress>,
    _client_data: *mut c_void,
) -> i32 {
    super::notify_change();
    0
}

fn scalar<T: Copy + Default>(
    object: AudioObjectID,
    selector: AudioObjectPropertySelector,
) -> Option<T> {
    let mut address = property_address(selector);
    let mut value = T::default();
    let mut size = mem::size_of::<T>() as u32;
    let status = unsafe {
        AudioObjectGetPropertyData(
            object,
            NonNull::from(&mut address),
            0,
            ptr::null(),
            NonNull::from(&mut size),
            NonNull::from(&mut value).cast::<c_void>(),
        )
    };
    (status == kAudioHardwareNoError && size as usize == mem::size_of::<T>()).then_some(value)
}

fn property_address(selector: AudioObjectPropertySelector) -> AudioObjectPropertyAddress {
    AudioObjectPropertyAddress {
        mSelector: selector,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain,
    }
}
