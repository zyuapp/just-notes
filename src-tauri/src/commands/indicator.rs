use tauri::{AppHandle, Manager};

use crate::indicator;

#[tauri::command]
pub(crate) fn set_indicator_width(app: AppHandle, visible_width: f64) {
    indicator::set_indicator_visible_width(&app, visible_width);
}

#[tauri::command]
pub(crate) fn open_main_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}
