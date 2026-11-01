use std::{ffi::c_void, mem};

use objc2_app_kit::NSRunningApplication;
use objc2_foundation::NSString;

use super::RunningApp;

const MAX_ANCESTORS: usize = 8;

/// Sandboxed audio helpers (Chromium, Electron, Firefox) are not registered
/// with LaunchServices, so the host app is found through the parent chain.
pub(super) fn registered_ancestor(pid: i32) -> Option<RunningApp> {
    let mut current = pid;
    for _ in 0..MAX_ANCESTORS {
        if let Some(app) = running_app(current) {
            return Some(app);
        }
        current = parent_pid(current)?;
    }
    None
}

pub(crate) fn application_running(bundle_id: &str) -> bool {
    objc2::rc::autoreleasepool(|_| {
        NSRunningApplication::runningApplicationsWithBundleIdentifier(&NSString::from_str(
            bundle_id,
        ))
        .iter()
        .any(|app| !app.isTerminated())
    })
}

fn running_app(pid: i32) -> Option<RunningApp> {
    let app = NSRunningApplication::runningApplicationWithProcessIdentifier(pid)?;
    let bundle_id = app.bundleIdentifier()?.to_string();
    let name = app
        .localizedName()
        .map(|name| name.to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| bundle_id.clone());
    Some(RunningApp { bundle_id, name })
}

fn parent_pid(pid: i32) -> Option<i32> {
    let mut info: libc::proc_bsdinfo = unsafe { mem::zeroed() };
    let size = mem::size_of::<libc::proc_bsdinfo>() as i32;
    let read = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDTBSDINFO,
            0,
            (&mut info as *mut libc::proc_bsdinfo).cast::<c_void>(),
            size,
        )
    };
    if read != size {
        return None;
    }
    let parent = i32::try_from(info.pbi_ppid).ok()?;
    (parent > 1 && parent != pid).then_some(parent)
}
