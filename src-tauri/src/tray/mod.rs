mod icon;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager, Wry,
};

const TRAY_ID: &str = "just-notes-tray";

struct TrayState {
    record: MenuItem<Wry>,
    recording: Arc<AtomicBool>,
}

pub(crate) fn init_tray(
    app: &tauri::App,
    on_start: impl Fn(&AppHandle) + Send + Sync + 'static,
    on_stop: impl Fn(&AppHandle) + Send + Sync + 'static,
) -> tauri::Result<()> {
    let record = MenuItem::with_id(app, "tray-record", "Start recording", true, None::<&str>)?;
    let open = MenuItem::with_id(app, "tray-open", "Open Just Notes", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "tray-quit", "Quit Just Notes", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[&record, &PredefinedMenuItem::separator(app)?, &open, &quit],
    )?;

    let recording = Arc::new(AtomicBool::new(false));
    let recording_for_event = Arc::clone(&recording);
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon::template_icon())
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "tray-record" => {
                if recording_for_event.load(Ordering::SeqCst) {
                    on_stop(app);
                } else {
                    on_start(app);
                }
            }
            "tray-open" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "tray-quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    app.manage(TrayState { record, recording });
    Ok(())
}

pub(crate) fn set_tray_recording(app: &AppHandle, recording: bool) {
    if let Some(state) = app.try_state::<TrayState>() {
        state.recording.store(recording, Ordering::SeqCst);
        let _ = state.record.set_text(if recording {
            "Stop recording"
        } else {
            "Start recording"
        });
    }
    if !recording {
        if let Some(tray) = app.tray_by_id(TRAY_ID) {
            // set_title(None) leaves the previous title visible on macOS;
            // an empty string actually clears it.
            let _ = tray.set_title(Some(""));
        }
    }
}

pub(crate) fn set_tray_elapsed(app: &AppHandle, elapsed_ms: u64) {
    let total_seconds = elapsed_ms / 1000;
    let title = format!("{:02}:{:02}", total_seconds / 60, total_seconds % 60);
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_title(Some(title));
    }
}
