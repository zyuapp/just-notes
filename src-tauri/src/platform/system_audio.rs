//! System audio capture authorization, which macOS governs through the
//! Screen & System Audio Recording privacy pane.

/// Granted, or not yet granted. macOS only exposes a boolean preflight for this
/// pane, so a first-run app and a user who explicitly declined are
/// indistinguishable here; both report `notGranted`.
pub(crate) fn authorization_status() -> String {
    if preflight() {
        "authorized".to_string()
    } else {
        "notGranted".to_string()
    }
}

#[cfg(target_os = "macos")]
fn preflight() -> bool {
    objc2_core_graphics::CGPreflightScreenCaptureAccess()
}

#[cfg(not(target_os = "macos"))]
fn preflight() -> bool {
    true
}
