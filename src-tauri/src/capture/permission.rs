use tauri::AppHandle;

#[cfg(target_os = "macos")]
use std::time::Duration;

#[cfg(target_os = "macos")]
pub(super) fn ensure_microphone_permission(app: &AppHandle) -> Result<(), String> {
    use std::sync::mpsc;

    let (sender, receiver) = mpsc::channel();

    app.run_on_main_thread(move || request_microphone_permission(sender))
        .map_err(|err| format!("Failed to request microphone permission on main thread: {err}"))?;

    receiver
        .recv_timeout(Duration::from_secs(120))
        .map_err(|_| "Timed out waiting for microphone permission".to_string())?
}

#[cfg(target_os = "macos")]
fn request_microphone_permission(sender: std::sync::mpsc::Sender<Result<(), String>>) {
    use block2::RcBlock;
    use objc2::runtime::Bool;
    use objc2_av_foundation::{AVAuthorizationStatus, AVCaptureDevice, AVMediaTypeAudio};

    let media_type = match unsafe { AVMediaTypeAudio } {
        Some(media_type) => media_type,
        None => {
            let _ = sender.send(Err("AVMediaTypeAudio is unavailable".to_string()));
            return;
        }
    };
    let status = unsafe { AVCaptureDevice::authorizationStatusForMediaType(media_type) };

    match status {
        AVAuthorizationStatus::Authorized => {
            let _ = sender.send(Ok(()));
        }
        AVAuthorizationStatus::Denied => {
            let _ = sender.send(Err(
                "Microphone access is denied in macOS Privacy & Security settings".to_string(),
            ));
        }
        AVAuthorizationStatus::Restricted => {
            let _ = sender.send(Err(
                "Microphone access is restricted by macOS policy".to_string()
            ));
        }
        AVAuthorizationStatus::NotDetermined => {
            let block = RcBlock::new(move |granted: Bool| {
                let result = if granted.as_bool() {
                    Ok(())
                } else {
                    Err("Microphone permission was not granted".to_string())
                };
                let _ = sender.send(result);
            });

            unsafe {
                AVCaptureDevice::requestAccessForMediaType_completionHandler(media_type, &block);
            }
            std::mem::forget(block);
        }
        other => {
            let _ = sender.send(Err(format!(
                "Unknown microphone authorization status: {other:?}"
            )));
        }
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn microphone_permission_status(app: &AppHandle) -> Result<String, String> {
    use std::sync::mpsc;

    let (sender, receiver) = mpsc::channel();
    app.run_on_main_thread(move || {
        let _ = sender.send(query_microphone_status());
    })
    .map_err(|err| format!("Failed to read microphone permission status: {err}"))?;

    receiver
        .recv_timeout(Duration::from_secs(5))
        .map_err(|_| "Timed out reading microphone permission status".to_string())
}

#[cfg(target_os = "macos")]
fn query_microphone_status() -> String {
    use objc2_av_foundation::{AVAuthorizationStatus, AVCaptureDevice, AVMediaTypeAudio};

    let Some(media_type) = (unsafe { AVMediaTypeAudio }) else {
        return "unknown".to_string();
    };
    match unsafe { AVCaptureDevice::authorizationStatusForMediaType(media_type) } {
        AVAuthorizationStatus::Authorized => "authorized",
        AVAuthorizationStatus::Denied => "denied",
        AVAuthorizationStatus::Restricted => "restricted",
        AVAuthorizationStatus::NotDetermined => "notDetermined",
        _ => "unknown",
    }
    .to_string()
}

#[cfg(not(target_os = "macos"))]
pub(super) fn ensure_microphone_permission(_app: &AppHandle) -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn microphone_permission_status(_app: &AppHandle) -> Result<String, String> {
    Ok("authorized".to_string())
}
