use tauri::{AppHandle, Builder, Emitter, Manager, Wry};

mod app;
mod capture;
mod commands;
mod ipc;
mod meeting_surfaces;
mod meetings;
mod platform;
mod recording;
mod settings;
mod threads;
mod transcription;
mod tray;

use app::AppPaths;
use meetings::MeetingSchedulerState;
use recording::RecorderState;
use settings::SettingsState;
use threads::repository::reset_stale_recording_threads;
use transcription::{FinalizeState, ModelDownloadState};

pub fn run() {
    let paths = AppPaths::discover().expect("failed to locate Just Notes data directory");
    let initial_settings = settings::load_settings(&paths.data_dir);

    let builder = Builder::default()
        .manage(paths)
        .manage(RecorderState::default())
        .manage(SettingsState::new(initial_settings))
        .manage(FinalizeState::default())
        .manage(ModelDownloadState::default())
        .manage(MeetingSchedulerState::default())
        .setup(|app| {
            let paths = app.state::<AppPaths>();
            let settings = app.state::<SettingsState>().snapshot();
            reset_stale_recording_threads(&settings::effective_paths(&paths, &settings))?;
            tray::init_tray(
                app,
                start_recording_from_tray,
                stop_recording_from_tray,
                meeting_surfaces::start_native_meeting,
            )?;
            let action_app = app.handle().clone();
            platform::notifications::initialize(
                meetings::notification_categories(),
                move |response| {
                    meetings::handle_notification_action(
                        action_app.clone(),
                        response,
                        meeting_surfaces::start_native_meeting_now,
                        |app| {
                            meeting_surfaces::sync_current(app);
                        },
                    );
                },
            );
            meetings::spawn_scheduler(app.handle().clone(), |app| {
                meeting_surfaces::sync_current(app);
            });
            // The minWidth/minHeight from tauri.conf.json is not enforced on
            // macOS; the layout needs at least this much room.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_min_size(Some(tauri::LogicalSize::new(900.0, 620.0)));
            }
            Ok(())
        });

    register_commands(builder)
        .build(tauri::generate_context!())
        .expect("error while building Just Notes")
        .run(handle_run_event);
}

// A recording must be stopped (WAV headers finalized, duration persisted)
// before the process is allowed to exit, whether the exit comes from closing
// the window, the tray Quit item, or Cmd+Q.
fn handle_run_event(app: &AppHandle, event: tauri::RunEvent) {
    if let tauri::RunEvent::ExitRequested { api, .. } = event {
        let recorder = app.state::<RecorderState>().inner().clone();
        let finalize = app.state::<FinalizeState>().inner().clone();
        if recorder.is_active() || finalize.is_active() {
            api.prevent_exit();
            let app = app.clone();
            tauri::async_runtime::spawn_blocking(move || {
                if recorder.is_active() {
                    stop_active_recording(&app);
                }
                finalize.wait_for_idle();
                app.exit(0);
            });
        }
    }
}

#[cfg(any(debug_assertions, feature = "qa-fixtures"))]
fn register_commands(builder: Builder<Wry>) -> Builder<Wry> {
    builder.invoke_handler(tauri::generate_handler![
        commands::system::get_app_info,
        commands::transcription::get_transcription_status,
        commands::transcription::start_transcription_model_download,
        commands::transcription::cancel_transcription_model_download,
        commands::transcription::delete_transcription_model,
        commands::system::get_permissions_status,
        commands::system::reveal_in_finder,
        commands::system::copy_text_to_clipboard,
        commands::system::open_privacy_settings,
        commands::threads::list_threads,
        commands::threads::create_thread,
        commands::threads::get_thread,
        commands::threads::rename_thread,
        commands::threads::list_archived_threads,
        commands::threads::archive_thread,
        commands::threads::restore_thread,
        commands::threads::delete_thread,
        commands::threads::update_segment_text,
        commands::threads::search_threads,
        commands::threads::export_thread_markdown,
        commands::settings::get_settings,
        commands::settings::update_settings,
        commands::settings::pick_folder,
        commands::meetings::get_meeting_access_status,
        commands::meetings::request_meeting_access,
        commands::meetings::get_meeting_prompt,
        commands::meetings::start_meeting_recording,
        commands::meetings::dismiss_meeting_prompt,
        commands::recording::start_recording,
        commands::recording::start_fixture_recording,
        commands::recording::stop_recording,
        commands::recording::reprocess_thread,
        commands::recording::cancel_finalization
    ])
}

#[cfg(not(any(debug_assertions, feature = "qa-fixtures")))]
fn register_commands(builder: Builder<Wry>) -> Builder<Wry> {
    builder.invoke_handler(tauri::generate_handler![
        commands::system::get_app_info,
        commands::transcription::get_transcription_status,
        commands::transcription::start_transcription_model_download,
        commands::transcription::cancel_transcription_model_download,
        commands::transcription::delete_transcription_model,
        commands::system::get_permissions_status,
        commands::system::reveal_in_finder,
        commands::system::copy_text_to_clipboard,
        commands::system::open_privacy_settings,
        commands::threads::list_threads,
        commands::threads::create_thread,
        commands::threads::get_thread,
        commands::threads::rename_thread,
        commands::threads::list_archived_threads,
        commands::threads::archive_thread,
        commands::threads::restore_thread,
        commands::threads::delete_thread,
        commands::threads::update_segment_text,
        commands::threads::search_threads,
        commands::threads::export_thread_markdown,
        commands::settings::get_settings,
        commands::settings::update_settings,
        commands::settings::pick_folder,
        commands::meetings::get_meeting_access_status,
        commands::meetings::request_meeting_access,
        commands::meetings::get_meeting_prompt,
        commands::meetings::start_meeting_recording,
        commands::meetings::dismiss_meeting_prompt,
        commands::recording::start_recording,
        commands::recording::stop_recording,
        commands::recording::reprocess_thread,
        commands::recording::cancel_finalization
    ])
}

// The tray starts a fresh thread: it has no window selection to record into.
// The frontend learns about it through the emitted `recording-started` event.
fn start_recording_from_tray(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let paths = app.state::<AppPaths>();
        let settings = app.state::<SettingsState>().snapshot();
        let effective = settings::effective_paths(&paths, &settings);
        let recorder = app.state::<RecorderState>().inner().clone();
        match recording::start_recording(app.clone(), effective, recorder, settings, None) {
            Ok(started) => {
                let payload = commands::recording::to_payload(started);
                let _ = app.emit("recording-started", &payload);
            }
            Err(err) => eprintln!("recording start failed: {err}"),
        }
    });
}

fn stop_recording_from_tray(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || stop_active_recording(&app));
}

fn stop_active_recording(app: &AppHandle) {
    let recorder = app.state::<RecorderState>().inner().clone();
    if let Err(err) = recording::stop_recording(app.clone(), recorder) {
        eprintln!("recording stop failed: {err}");
    }
}
