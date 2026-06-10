use tauri::{AppHandle, Builder, Manager, Wry};

mod app;
mod capture;
mod commands;
mod ipc;
mod platform;
mod recording;
mod settings;
mod threads;
mod transcription;
mod tray;

use app::AppPaths;
use recording::RecorderState;
use settings::SettingsState;
use threads::repository::reset_stale_recording_threads;
use transcription::FinalizeState;

pub fn run() {
    let paths = AppPaths::discover().expect("failed to locate Just Notes data directory");
    let initial_settings = settings::load_settings(&paths.data_dir);

    let builder = Builder::default()
        .manage(paths)
        .manage(RecorderState::default())
        .manage(SettingsState::new(initial_settings))
        .manage(FinalizeState::default())
        .setup(|app| {
            let paths = app.state::<AppPaths>();
            let settings = app.state::<SettingsState>().snapshot();
            reset_stale_recording_threads(&settings::effective_paths(&paths, &settings))?;
            tray::init_tray(app, stop_recording_from_tray)?;
            Ok(())
        });

    register_commands(builder)
        .run(tauri::generate_context!())
        .expect("error while running Just Notes");
}

#[cfg(any(debug_assertions, feature = "qa-fixtures"))]
fn register_commands(builder: Builder<Wry>) -> Builder<Wry> {
    builder.invoke_handler(tauri::generate_handler![
        commands::system::get_app_info,
        commands::system::get_transcription_status,
        commands::system::get_permissions_status,
        commands::system::reveal_in_finder,
        commands::system::copy_text_to_clipboard,
        commands::system::open_privacy_settings,
        commands::threads::list_threads,
        commands::threads::create_thread,
        commands::threads::get_thread,
        commands::threads::rename_thread,
        commands::threads::delete_thread,
        commands::threads::rename_speaker,
        commands::threads::update_segment_text,
        commands::threads::search_threads,
        commands::threads::export_thread_markdown,
        commands::settings::get_settings,
        commands::settings::update_settings,
        commands::settings::pick_folder,
        commands::recording::start_recording,
        commands::recording::start_fixture_recording,
        commands::recording::stop_recording,
        commands::recording::cancel_finalization
    ])
}

#[cfg(not(any(debug_assertions, feature = "qa-fixtures")))]
fn register_commands(builder: Builder<Wry>) -> Builder<Wry> {
    builder.invoke_handler(tauri::generate_handler![
        commands::system::get_app_info,
        commands::system::get_transcription_status,
        commands::system::get_permissions_status,
        commands::system::reveal_in_finder,
        commands::system::copy_text_to_clipboard,
        commands::system::open_privacy_settings,
        commands::threads::list_threads,
        commands::threads::create_thread,
        commands::threads::get_thread,
        commands::threads::rename_thread,
        commands::threads::delete_thread,
        commands::threads::rename_speaker,
        commands::threads::update_segment_text,
        commands::threads::search_threads,
        commands::threads::export_thread_markdown,
        commands::settings::get_settings,
        commands::settings::update_settings,
        commands::settings::pick_folder,
        commands::recording::start_recording,
        commands::recording::stop_recording,
        commands::recording::cancel_finalization
    ])
}

fn stop_recording_from_tray(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let paths = app.state::<AppPaths>().inner().clone();
        let recorder = app.state::<RecorderState>().inner().clone();
        let finalize = app.state::<FinalizeState>().inner().clone();
        let snapshot = app.state::<SettingsState>().snapshot();
        let effective = settings::effective_paths(&paths, &snapshot);
        if let Err(err) =
            recording::stop_recording(app.clone(), effective, recorder, snapshot, finalize)
        {
            eprintln!("tray stop failed: {err}");
        }
    });
}
