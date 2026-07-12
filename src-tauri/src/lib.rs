use tauri::{AppHandle, Builder, Emitter, Manager, Wry};

mod app;
mod capture;
mod commands;
mod ipc;
mod meetings;
mod platform;
mod recording;
mod settings;
mod threads;
mod transcription;
mod tray;

use app::{AppPaths, StorageGate};
use meetings::MeetingSchedulerState;
use platform::FolderAccessState;
use recording::RecorderState;
use settings::SettingsState;
use threads::repository::reset_stale_recording_threads;
use transcription::{FinalizeState, ModelDownloadState};

type SetupResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub fn run() {
    let builder = Builder::default()
        .manage(RecorderState::default())
        .manage(FinalizeState::default())
        .manage(ModelDownloadState::default())
        .manage(StorageGate::default())
        .manage(MeetingSchedulerState::default())
        .setup(setup_app);

    register_commands(builder)
        .build(tauri::generate_context!())
        .expect("error while building Just Notes")
        .run(handle_run_event);
}

fn setup_app(app: &mut tauri::App<Wry>) -> SetupResult {
    let initial_settings = manage_persistent_state(app)?;
    let paths = app.state::<AppPaths>();
    reset_stale_recording_threads(&settings::effective_paths(&paths, &initial_settings))?;
    tray::init_tray(app, start_recording_from_tray, stop_recording_from_tray)?;
    let action_app = app.handle().clone();
    platform::notifications::initialize(meetings::notification_categories(), move |response| {
        meetings::handle_notification_action(action_app.clone(), response);
    });
    meetings::spawn_scheduler(app.handle().clone());
    // The minWidth/minHeight from tauri.conf.json is not enforced on
    // macOS; the layout needs at least this much room.
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_min_size(Some(tauri::LogicalSize::new(900.0, 620.0)));
    }
    Ok(())
}

fn manage_persistent_state(app: &mut tauri::App<Wry>) -> SetupResult<settings::AppSettings> {
    let paths = AppPaths::from_data_dir(app.path().app_data_dir()?);
    paths.ensure().map_err(std::io::Error::other)?;
    let access = FolderAccessState::default();
    let mut persisted = settings::load_persisted_settings(&paths.data_dir);
    if let Some(bookmark) = persisted.transcripts_bookmark() {
        match platform::restore_folder_access(app.handle(), bookmark) {
            Ok(restored) => {
                persisted.set_transcripts_folder(restored.path.clone(), restored.bookmark.clone());
                access.install(restored).map_err(std::io::Error::other)?;
            }
            Err(err) => {
                eprintln!("saved transcripts folder permission could not be restored: {err}");
                let mut initial_settings = persisted.settings().clone();
                initial_settings.transcripts_dir = None;
                app.manage(paths);
                app.manage(SettingsState::from_unavailable_folder(persisted));
                app.manage(access);
                return Ok(initial_settings);
            }
        }
        settings::save_persisted_settings(&paths.data_dir, &persisted)
            .map_err(std::io::Error::other)?;
    }
    let initial_settings = persisted.settings().clone();
    app.manage(paths);
    app.manage(SettingsState::from_persisted(persisted));
    app.manage(access);
    Ok(initial_settings)
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

macro_rules! command_handler {
    ($builder:expr $(, $extra:path)*) => {
        $builder.invoke_handler(tauri::generate_handler![
        commands::system::get_app_info,
        commands::transcription::get_transcription_status,
        commands::transcription::start_transcription_model_download,
        commands::transcription::cancel_transcription_model_download,
        commands::transcription::delete_transcription_model,
        commands::system::get_permissions_status,
        commands::system::reveal_in_finder,
        commands::system::copy_text_to_clipboard,
        commands::system::open_privacy_settings,
        commands::system::open_external_url,
        commands::system::open_legal_document,
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
        commands::settings::choose_transcripts_folder,
        commands::settings::import_legacy_data,
        commands::settings::use_default_transcripts_folder,
        commands::meetings::get_meeting_access_status,
        commands::meetings::request_meeting_access,
        commands::recording::start_recording,
        commands::recording::stop_recording,
        commands::recording::reprocess_thread,
        commands::recording::cancel_finalization,
        $($extra),*
        ])
    };
}

#[cfg(any(debug_assertions, feature = "qa-fixtures"))]
fn register_commands(builder: Builder<Wry>) -> Builder<Wry> {
    command_handler!(builder, commands::recording::start_fixture_recording)
}

#[cfg(not(any(debug_assertions, feature = "qa-fixtures")))]
fn register_commands(builder: Builder<Wry>) -> Builder<Wry> {
    command_handler!(builder)
}

// The tray starts a fresh thread: it has no window selection to record into.
// The frontend learns about it through the emitted `recording-started` event.
fn start_recording_from_tray(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let paths = app.state::<AppPaths>();
        let settings = app.state::<SettingsState>().inner().clone();
        let recorder = app.state::<RecorderState>().inner().clone();
        match recording::start_recording(
            app.clone(),
            paths.inner().clone(),
            recorder,
            settings,
            None,
        ) {
            Ok(payload) => {
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
