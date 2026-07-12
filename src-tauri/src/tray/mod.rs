mod icon;

use std::sync::{Arc, Mutex};

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager, Wry,
};

const TRAY_ID: &str = "just-notes-tray";

struct TrayState {
    record: MenuItem<Wry>,
    meeting: MenuItem<Wry>,
    view: Arc<Mutex<TrayView>>,
}

#[derive(Clone)]
pub(crate) struct TrayMeetingPrompt {
    pub(crate) request_id: String,
    pub(crate) title: String,
}

#[derive(Default)]
struct TrayView {
    recording: bool,
    prompt: Option<TrayMeetingPrompt>,
}

struct TrayMenu {
    menu: Menu<Wry>,
    record: MenuItem<Wry>,
    meeting: MenuItem<Wry>,
}

fn build_menu(app: &tauri::App) -> tauri::Result<TrayMenu> {
    let record = MenuItem::with_id(app, "tray-record", "Start recording", true, None::<&str>)?;
    let meeting = MenuItem::with_id(
        app,
        "tray-meeting-record",
        "No meeting starting soon",
        false,
        None::<&str>,
    )?;
    let open = MenuItem::with_id(app, "tray-open", "Open Just Notes", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "tray-quit", "Quit Just Notes", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &meeting,
            &record,
            &PredefinedMenuItem::separator(app)?,
            &open,
            &quit,
        ],
    )?;
    Ok(TrayMenu {
        menu,
        record,
        meeting,
    })
}

pub(crate) fn init_tray(
    app: &tauri::App,
    on_start: impl Fn(&AppHandle) + Send + Sync + 'static,
    on_stop: impl Fn(&AppHandle) + Send + Sync + 'static,
    on_start_meeting: impl Fn(&AppHandle, String) + Send + Sync + 'static,
) -> tauri::Result<()> {
    let TrayMenu {
        menu,
        record,
        meeting,
    } = build_menu(app)?;

    let view = Arc::new(Mutex::new(TrayView::default()));
    let view_for_event = Arc::clone(&view);
    let meeting_for_event = meeting.clone();
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon::template_icon())
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "tray-record" => {
                let recording = view_for_event.lock().is_ok_and(|view| view.recording);
                if recording {
                    on_stop(app);
                } else {
                    on_start(app);
                }
            }
            "tray-meeting-record" => {
                let request_id = view_for_event.lock().ok().and_then(|mut view| {
                    let request_id = view.prompt.take().map(|prompt| prompt.request_id);
                    render_meeting(&meeting_for_event, &view);
                    request_id
                });
                if let Some(request_id) = request_id {
                    on_start_meeting(app, request_id);
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

    app.manage(TrayState {
        record,
        meeting,
        view,
    });
    Ok(())
}

pub(crate) fn set_tray_recording(app: &AppHandle, recording: bool) {
    if let Some(state) = app.try_state::<TrayState>() {
        if let Ok(mut view) = state.view.lock() {
            view.recording = recording;
            let _ = state.record.set_text(if recording {
                "Stop recording"
            } else {
                "Start recording"
            });
            render_meeting(&state.meeting, &view);
        }
    }
    if !recording {
        if let Some(tray) = app.tray_by_id(TRAY_ID) {
            // set_title(None) leaves the previous title visible on macOS;
            // an empty string actually clears it.
            let _ = tray.set_title(Some(""));
        }
    }
}

pub(crate) fn set_meeting_prompt(app: &AppHandle, prompt: Option<TrayMeetingPrompt>) {
    let Some(state) = app.try_state::<TrayState>() else {
        return;
    };
    if let Ok(mut view) = state.view.lock() {
        view.prompt = prompt;
        render_meeting(&state.meeting, &view);
    };
}

fn render_meeting(item: &MenuItem<Wry>, view: &TrayView) {
    let label = view
        .prompt
        .as_ref()
        .map(|prompt| format!("Record “{}”", prompt.title))
        .unwrap_or_else(|| "No meeting starting soon".to_string());
    let _ = item.set_text(label);
    let _ = item.set_enabled(!view.recording && view.prompt.is_some());
}

pub(crate) fn set_tray_elapsed(app: &AppHandle, elapsed_ms: u64) {
    let total_seconds = elapsed_ms / 1000;
    let title = format!("{:02}:{:02}", total_seconds / 60, total_seconds % 60);
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_title(Some(title));
    }
}
