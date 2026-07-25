//! System audio capture authorization, which macOS governs through the
//! Screen & System Audio Recording privacy pane.
//!
//! Core Audio exposes no way to read the current status, so the only authority
//! is creating the same process tap a recording would and seeing whether macOS
//! allows it.

use std::sync::atomic::{AtomicBool, Ordering};

use super::tap::{probe_process_tap, TAP_PROBE_NAME};

/// Revoking this permission makes macOS ask the user to relaunch the app, so a
/// tap that once succeeded still describes this process. Remembering it keeps
/// the settings pane from creating a second tap while a recording holds one.
static TAP_SUCCEEDED: AtomicBool = AtomicBool::new(false);

pub(in crate::capture) fn remember_tap_succeeded() {
    TAP_SUCCEEDED.store(true, Ordering::Relaxed);
}

/// `authorized`, or `notGranted`. macOS answers the underlying question with a
/// bare success or failure, so a first-run app and a user who explicitly
/// declined are indistinguishable here; both report `notGranted`.
pub(crate) fn authorization_status() -> String {
    if TAP_SUCCEEDED.load(Ordering::Relaxed) || probe_process_tap(TAP_PROBE_NAME) {
        "authorized".to_string()
    } else {
        "notGranted".to_string()
    }
}
