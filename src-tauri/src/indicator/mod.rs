use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use tauri::{AppHandle, Emitter, LogicalPosition, Manager, WebviewUrl, WebviewWindowBuilder};

const INDICATOR_LABEL: &str = "indicator";
const MAIN_LABEL: &str = "main";
// The window is wider than the pill ever renders; the surplus hangs past the
// right screen edge, so revealing or hiding controls only moves the window
// instead of resizing it.
const WINDOW_WIDTH: f64 = 160.0;
const WINDOW_HEIGHT: f64 = 26.0;
const INITIAL_VISIBLE_WIDTH: f64 = 56.0;
const HOVER_POLL_INTERVAL: Duration = Duration::from_millis(80);

// The pill is visible while recording, and also while the main window exists
// but is not focused, so a new recording can be started from it. The webview
// is told which mode to render through the `indicator-state` event and the
// `get_indicator_state` command.
struct IndicatorState {
    recording: bool,
    main_focused: bool,
    hover_watch: Option<Arc<AtomicBool>>,
}

impl Default for IndicatorState {
    fn default() -> Self {
        Self {
            recording: false,
            main_focused: true,
            hover_watch: None,
        }
    }
}

pub(crate) fn set_indicator_recording(app: &AppHandle, recording: bool) {
    update(app, |state| state.recording = recording);
}

pub(crate) fn set_main_window_focused(app: &AppHandle, focused: bool) {
    update(app, |state| state.main_focused = focused);
}

pub(crate) fn indicator_is_recording(app: &AppHandle) -> bool {
    with_state(app, |state| state.recording)
}

pub(crate) fn close_indicator(app: &AppHandle) {
    stop_hover_watch(app);
    if let Some(window) = app.get_webview_window(INDICATOR_LABEL) {
        let _ = window.close();
    }
}

// `visible_width` is the logical width of the pill content; the window slides
// so exactly that many pixels stay on screen, right edge flush with the
// monitor's work area.
pub(crate) fn set_indicator_visible_width(app: &AppHandle, visible_width: f64) {
    let Some(window) = app.get_webview_window(INDICATOR_LABEL) else {
        return;
    };
    let Some((right_edge, center_y)) = screen_anchor(app) else {
        return;
    };
    let width = visible_width.clamp(0.0, WINDOW_WIDTH);
    let _ = window.set_position(LogicalPosition::new(
        right_edge - width,
        center_y - WINDOW_HEIGHT / 2.0,
    ));
}

fn update(app: &AppHandle, mutate: impl FnOnce(&mut IndicatorState)) {
    let (recording, visible) = with_state(app, |state| {
        mutate(state);
        (state.recording, state.recording || !state.main_focused)
    });
    // Without a main window the app is shutting down; never (re)create the
    // pill in that state or it would keep the process alive.
    if !visible || app.get_webview_window(MAIN_LABEL).is_none() {
        stop_hover_watch(app);
        if let Some(window) = app.get_webview_window(INDICATOR_LABEL) {
            let _ = window.hide();
        }
        return;
    }
    if let Err(err) = show_indicator(app) {
        eprintln!("recording indicator failed to open: {err}");
        return;
    }
    let _ = app.emit("indicator-state", recording);
    start_hover_watch(app);
}

// macOS only delivers webview hover events while the application is active,
// and the pill exists almost exclusively while it is not. Hover is therefore
// derived from the global cursor position and pushed to the webview as the
// `indicator-hover` event.
fn start_hover_watch(app: &AppHandle) {
    let flag = with_state(app, |state| {
        if let Some(flag) = &state.hover_watch {
            if flag.load(Ordering::Relaxed) {
                return None;
            }
        }
        let flag = Arc::new(AtomicBool::new(true));
        state.hover_watch = Some(Arc::clone(&flag));
        Some(flag)
    });
    let Some(flag) = flag else {
        return;
    };
    let app = app.clone();
    thread::spawn(move || {
        let mut last_hovered = None;
        while flag.load(Ordering::Relaxed) {
            let Some(window) = app.get_webview_window(INDICATOR_LABEL) else {
                break;
            };
            let hovered = cursor_over_window(&app, &window).unwrap_or(false);
            if last_hovered != Some(hovered) {
                last_hovered = Some(hovered);
                let _ = app.emit("indicator-hover", hovered);
            }
            thread::sleep(HOVER_POLL_INTERVAL);
        }
    });
}

fn stop_hover_watch(app: &AppHandle) {
    if let Some(flag) = with_state(app, |state| state.hover_watch.take()) {
        flag.store(false, Ordering::Relaxed);
    }
}

fn cursor_over_window(app: &AppHandle, window: &tauri::WebviewWindow) -> Option<bool> {
    let cursor = app.cursor_position().ok()?;
    let position = window.outer_position().ok()?;
    let size = window.outer_size().ok()?;
    Some(
        cursor.x >= position.x as f64
            && cursor.x < position.x as f64 + size.width as f64
            && cursor.y >= position.y as f64
            && cursor.y < position.y as f64 + size.height as f64,
    )
}

fn with_state<R>(app: &AppHandle, access: impl FnOnce(&mut IndicatorState) -> R) -> R {
    if app.try_state::<Mutex<IndicatorState>>().is_none() {
        app.manage(Mutex::new(IndicatorState::default()));
    }
    let state = app.state::<Mutex<IndicatorState>>();
    let mut guard = state.lock().expect("indicator state lock poisoned");
    access(&mut guard)
}

fn show_indicator(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(INDICATOR_LABEL) {
        window.show()?;
        return Ok(());
    }
    let mut builder = WebviewWindowBuilder::new(
        app,
        INDICATOR_LABEL,
        WebviewUrl::App("indicator.html".into()),
    )
    .title("Recording")
    .inner_size(WINDOW_WIDTH, WINDOW_HEIGHT)
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .resizable(false)
    .always_on_top(true)
    .visible_on_all_workspaces(true)
    .accept_first_mouse(true)
    .focusable(false);
    if let Some((right_edge, center_y)) = screen_anchor(app) {
        builder = builder.position(
            right_edge - INITIAL_VISIBLE_WIDTH,
            center_y - WINDOW_HEIGHT / 2.0,
        );
    }
    builder.build()?;
    Ok(())
}

fn screen_anchor(app: &AppHandle) -> Option<(f64, f64)> {
    let monitor = app
        .get_webview_window(MAIN_LABEL)
        .and_then(|window| window.current_monitor().ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten())?;
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let right_edge = (area.position.x as f64 + area.size.width as f64) / scale;
    let center_y = (area.position.y as f64 + area.size.height as f64 / 2.0) / scale;
    Some((right_edge, center_y))
}
