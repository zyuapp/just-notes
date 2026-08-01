use std::{path::Path, sync::mpsc, time::Duration};

use objc2::MainThreadMarker;
use objc2_app_kit::{NSAlert, NSAlertStyle, NSPasteboard, NSPasteboardTypeString, NSWorkspace};
use objc2_foundation::{NSArray, NSString, NSURL};
use tauri::AppHandle;

pub(crate) mod calendar;
pub(crate) mod notifications;

const APP_KIT_TIMEOUT: Duration = Duration::from_secs(120);

fn permission_request_result(permission: &str, error: Option<String>) -> Result<(), String> {
    match error {
        Some(error) => Err(format!("Failed to request {permission} access: {error}")),
        None => Ok(()),
    }
}

pub(crate) fn reveal_in_finder(app: &AppHandle, path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("Cannot reveal an empty path in Finder".to_string());
    }
    if !Path::new(path).exists() {
        return Err(format!(
            "Finder could not reveal {path} because it does not exist"
        ));
    }

    let path = path.to_string();
    run_on_main_thread(app, move || {
        let url = NSURL::fileURLWithPath(&NSString::from_str(&path));
        let urls = NSArray::from_retained_slice(&[url]);
        NSWorkspace::sharedWorkspace().activateFileViewerSelectingURLs(&urls);
        Ok(())
    })
}

pub(crate) fn show_blocking_error(title: &str, message: &str) -> Result<(), String> {
    let marker = MainThreadMarker::new()
        .ok_or_else(|| "A blocking macOS alert must run on the main thread".to_string())?;
    let alert = NSAlert::new(marker);
    alert.setAlertStyle(NSAlertStyle::Critical);
    alert.setMessageText(&NSString::from_str(title));
    alert.setInformativeText(&NSString::from_str(message));
    let _ = alert.runModal();
    Ok(())
}

pub(crate) fn copy_to_clipboard(app: &AppHandle, text: &str) -> Result<(), String> {
    let text = text.to_string();
    run_on_main_thread(app, move || {
        let pasteboard = NSPasteboard::generalPasteboard();
        pasteboard.clearContents();
        // SAFETY: NSPasteboardTypeString is a public, immutable AppKit
        // constant valid for the lifetime of the process.
        let string_type = unsafe { NSPasteboardTypeString };
        if pasteboard.setString_forType(&NSString::from_str(&text), string_type) {
            Ok(())
        } else {
            Err("The clipboard copy failed".to_string())
        }
    })
}

pub(crate) fn open_privacy_settings(app: &AppHandle, pane: &str) -> Result<(), String> {
    let url = privacy_settings_url(pane).to_string();
    run_on_main_thread(app, move || {
        let url = NSURL::URLWithString(&NSString::from_str(&url))
            .ok_or_else(|| "The System Settings URL is invalid".to_string())?;
        if NSWorkspace::sharedWorkspace().openURL(&url) {
            Ok(())
        } else {
            Err("System Settings could not be opened".to_string())
        }
    })
}

pub(crate) fn open_https_url(app: &AppHandle, url: &str) -> Result<(), String> {
    if !url.starts_with("https://") {
        return Err("Only HTTPS links can be opened".to_string());
    }
    open_workspace_url(app, url.to_string(), "The link")
}

pub(crate) fn open_file(app: &AppHandle, path: &Path) -> Result<(), String> {
    if !path.is_file() {
        return Err("The bundled legal document is missing".to_string());
    }
    let path = path.display().to_string();
    run_on_main_thread(app, move || {
        let url = NSURL::fileURLWithPath(&NSString::from_str(&path));
        if NSWorkspace::sharedWorkspace().openURL(&url) {
            Ok(())
        } else {
            Err("The legal document could not be opened".to_string())
        }
    })
}

fn open_workspace_url(app: &AppHandle, url: String, label: &str) -> Result<(), String> {
    let label = label.to_string();
    run_on_main_thread(app, move || {
        let url = NSURL::URLWithString(&NSString::from_str(&url))
            .ok_or_else(|| format!("{label} URL is invalid"))?;
        if NSWorkspace::sharedWorkspace().openURL(&url) {
            Ok(())
        } else {
            Err(format!("{label} could not be opened"))
        }
    })
}

fn privacy_settings_url(pane: &str) -> &'static str {
    match pane {
        "microphone" => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"
        }
        "system-audio" => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_AudioCapture"
        }
        "calendar" => "x-apple.systempreferences:com.apple.preference.security?Privacy_Calendars",
        "notifications" => "x-apple.systempreferences:com.apple.Notifications-Settings.extension",
        _ => "x-apple.systempreferences:com.apple.preference.security",
    }
}

fn run_on_main_thread<T, F>(app: &AppHandle, operation: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    // App setup runs on the main thread already. Executing inline avoids
    // dispatching back to the same queue and then blocking it on the receiver.
    if MainThreadMarker::new().is_some() {
        return operation();
    }

    let (sender, receiver) = mpsc::channel();
    app.run_on_main_thread(move || {
        let _ = sender.send(operation());
    })
    .map_err(|err| format!("Failed to schedule macOS integration on the main thread: {err}"))?;
    receiver
        .recv_timeout(APP_KIT_TIMEOUT)
        .map_err(|_| "Timed out waiting for macOS".to_string())?
}

#[cfg(test)]
mod tests {
    use super::{permission_request_result, privacy_settings_url};

    #[test]
    fn maps_known_privacy_panes() {
        assert!(privacy_settings_url("microphone").ends_with("Privacy_Microphone"));
        assert!(privacy_settings_url("system-audio").ends_with("Privacy_AudioCapture"));
        assert!(privacy_settings_url("calendar").ends_with("Privacy_Calendars"));
        assert!(privacy_settings_url("notifications").contains("Notifications-Settings"));
    }

    #[test]
    fn unknown_privacy_pane_opens_privacy_root() {
        assert_eq!(
            privacy_settings_url("unsupported"),
            "x-apple.systempreferences:com.apple.preference.security"
        );
    }

    #[test]
    fn completed_permission_decisions_are_not_errors() {
        assert_eq!(permission_request_result("Calendar", None), Ok(()));
        assert_eq!(permission_request_result("notification", None), Ok(()));
    }

    #[test]
    fn native_permission_errors_keep_the_permission_name() {
        assert_eq!(
            permission_request_result("Calendar", Some("EventKit failed".to_string())),
            Err("Failed to request Calendar access: EventKit failed".to_string())
        );
    }
}
