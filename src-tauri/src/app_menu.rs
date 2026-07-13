use tauri::{
    menu::{Menu, MenuBuilder, MenuItem, SubmenuBuilder},
    AppHandle, Emitter, Manager, Wry,
};

const IMPORT_LEGACY_MENU_ID: &str = "file-import-legacy";

pub(crate) fn build(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let app_menu = SubmenuBuilder::new(app, "Just Notes")
        .about(None)
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;
    let import = MenuItem::with_id(
        app,
        IMPORT_LEGACY_MENU_ID,
        "Import Previous Recordings…",
        true,
        None::<&str>,
    )?;
    let file_menu = SubmenuBuilder::new(app, "File")
        .item(&import)
        .separator()
        .close_window()
        .build()?;
    let edit_menu = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;
    let view_menu = SubmenuBuilder::new(app, "View").fullscreen().build()?;
    let window_menu = SubmenuBuilder::new(app, "Window")
        .minimize()
        .maximize()
        .build()?;
    MenuBuilder::new(app)
        .items(&[&app_menu, &file_menu, &edit_menu, &view_menu, &window_menu])
        .build()
}

pub(crate) fn handle_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    if event.id().as_ref() != IMPORT_LEGACY_MENU_ID {
        return;
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
    let _ = app.emit("legacy-import-requested", ());
}
