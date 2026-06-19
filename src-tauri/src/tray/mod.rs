mod icon;

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager, Wry,
};

const TRAY_ID: &str = "just-notes-tray";

struct TrayMenuItems {
    status: MenuItem<Wry>,
    stop: MenuItem<Wry>,
}

pub(crate) fn init_tray(
    app: &tauri::App,
    on_stop: impl Fn(&AppHandle) + Send + Sync + 'static,
) -> tauri::Result<()> {
    let status = MenuItem::with_id(app, "tray-status", "Not recording", false, None::<&str>)?;
    let stop = MenuItem::with_id(app, "tray-stop", "Stop recording", false, None::<&str>)?;
    let open = MenuItem::with_id(app, "tray-open", "Open Just Notes", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "tray-quit", "Quit Just Notes", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &status,
            &PredefinedMenuItem::separator(app)?,
            &stop,
            &open,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon::template_icon())
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "tray-stop" => on_stop(app),
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

    app.manage(TrayMenuItems { status, stop });
    Ok(())
}

pub(crate) fn set_tray_recording(app: &AppHandle, recording: bool) {
    if let Some(items) = app.try_state::<TrayMenuItems>() {
        let _ = items.status.set_text(if recording {
            "Recording…"
        } else {
            "Not recording"
        });
        let _ = items.stop.set_enabled(recording);
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
