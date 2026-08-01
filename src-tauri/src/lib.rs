use tauri::{AppHandle, Builder, Emitter, Manager, WebviewWindowBuilder, Wry};

mod agent_access;
mod app;
mod app_menu;
mod capture;
mod commands;
mod ipc;
mod meeting_surfaces;
mod meetings;
mod platform;
mod recording;
mod recording_payload;
mod settings;
mod threads;
mod transcription;
mod tray;

use agent_access::{AgentAccessPaths, GuideLifecycle};
use app::{AppPaths, DataRootMigration, DataRootMigrationError};
use meetings::MeetingSchedulerState;
use recording::RecorderState;
use settings::SettingsState;
use threads::{readiness::migrate_retrieval_readiness, repository::reset_stale_recording_threads};
use transcription::ModelDownloadState;

type SetupResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub fn run() {
    let builder = Builder::default()
        .manage(RecorderState::default())
        .manage(ModelDownloadState::default())
        .manage(MeetingSchedulerState::default())
        .menu(app_menu::build)
        .setup(setup_app);

    register_commands(builder)
        .build(tauri::generate_context!())
        .expect("error while building Just Notes")
        .run(handle_run_event);
}

fn setup_app(app: &mut tauri::App<Wry>) -> SetupResult {
    match manage_persistent_state(app)? {
        PersistentStateSetup::Ready => {}
        PersistentStateSetup::MigrationConflict(error) => {
            platform::show_blocking_error(
                "Just Notes could not open your notes",
                &error.to_string(),
            )
            .map_err(std::io::Error::other)?;
            app.handle().exit(1);
            return Ok(());
        }
    }
    let paths = app.state::<AppPaths>();
    paths.cleanup_abandoned_import_staging()?;
    migrate_retrieval_readiness(&paths)?;
    reset_stale_recording_threads(&paths)?;
    build_main_window(app)?;
    tray::init_tray(
        app,
        start_recording_from_tray,
        stop_recording_from_tray,
        meeting_surfaces::start_native_meeting,
    )?;
    let action_app = app.handle().clone();
    platform::notifications::initialize(meetings::notification_categories(), move |response| {
        meetings::handle_notification_action(
            action_app.clone(),
            response,
            meeting_surfaces::start_native_meeting_now,
            |app| {
                meeting_surfaces::sync_current(app);
            },
        );
    });
    meetings::spawn_scheduler(
        app.handle().clone(),
        meeting_surfaces::start_native_meeting_now,
        |app| {
            meeting_surfaces::sync_current(app);
        },
    );
    Ok(())
}

fn build_main_window(app: &tauri::App<Wry>) -> SetupResult {
    let config = app
        .config()
        .app
        .windows
        .first()
        .ok_or_else(|| std::io::Error::other("Main window configuration is missing"))?;
    let window = WebviewWindowBuilder::from_config(app.handle(), config)?.build()?;
    // The configured minimum is not enforced on macOS.
    window.set_min_size(Some(tauri::LogicalSize::new(900.0, 620.0)))?;
    Ok(())
}

enum PersistentStateSetup {
    Ready,
    MigrationConflict(DataRootMigrationError),
}

fn manage_persistent_state(app: &mut tauri::App<Wry>) -> SetupResult<PersistentStateSetup> {
    let home_dir = app.path().home_dir()?;
    let migration = DataRootMigration::for_home(app.path().app_data_dir()?, &home_dir);
    let data_dir = match migration.resolve() {
        Ok(data_dir) => data_dir,
        Err(error) if error.is_conflict() => {
            return Ok(PersistentStateSetup::MigrationConflict(error))
        }
        Err(error) => return Err(error.into()),
    };
    let paths = AppPaths::from_data_dir(data_dir);
    paths.ensure().map_err(std::io::Error::other)?;
    let settings = settings::load_settings(&paths.data_dir);
    let agent_access_paths =
        AgentAccessPaths::from_roots(&paths.data_dir, &home_dir).map_err(std::io::Error::other)?;
    app.manage(paths);
    app.manage(SettingsState::new_state(settings));
    app.manage(GuideLifecycle::new(agent_access_paths));
    Ok(PersistentStateSetup::Ready)
}

// A recording must be stopped (WAV headers finalized, duration persisted)
// before the process is allowed to exit, whether the exit comes from closing
// the window, the tray Quit item, or Cmd+Q.
fn handle_run_event(app: &AppHandle, event: tauri::RunEvent) {
    if let tauri::RunEvent::ExitRequested { api, .. } = event {
        let recorder = app.state::<RecorderState>().inner().clone();
        if recorder.is_active() {
            api.prevent_exit();
            let app = app.clone();
            tauri::async_runtime::spawn_blocking(move || {
                stop_active_recording(&app);
                app.exit(0);
            });
        }
    }
}

macro_rules! command_handler {
    ($builder:expr $(, $extra:path)*) => {
        $builder.invoke_handler(tauri::generate_handler![
        commands::agent_access::get_agent_guide_statuses,
        commands::agent_access::install_agent_guides,
        commands::agent_access::remove_agent_guide,
        commands::agent_access::reveal_agent_guide,
        commands::system::get_app_info,
        commands::transcription::get_transcription_status,
        commands::transcription::start_transcription_model_download,
        commands::transcription::cancel_transcription_model_download,
        commands::transcription::delete_transcription_model,
        commands::system::get_permissions_status,
        commands::threads::get_storage_usage,
        commands::threads::delete_reclaimable_raw_audio,
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
        commands::threads::search_threads,
        commands::threads::export_thread_markdown,
        commands::settings::get_settings,
        commands::settings::update_settings,
        commands::meetings::get_meeting_access_status,
        commands::meetings::request_meeting_calendar_access,
        commands::meetings::request_meeting_notification_access,
        commands::meetings::get_meeting_prompt,
        commands::meetings::start_meeting_recording,
        commands::meetings::dismiss_meeting_prompt,
        commands::recording::start_recording,
        commands::recording::stop_recording,
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
            Ok(started) => {
                let payload = recording_payload::from_started(started);
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
