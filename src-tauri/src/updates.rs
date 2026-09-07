use std::sync::mpsc;

use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

use crate::platform;

pub(crate) const CHECK_FOR_UPDATES_MENU_ID: &str = "check-for-updates";

pub(crate) fn check_on_launch(app: &AppHandle) {
    if cfg!(debug_assertions) {
        return;
    }
    spawn_check(app.clone(), false);
}

pub(crate) fn check_requested(app: &AppHandle) {
    spawn_check(app.clone(), true);
}

fn spawn_check(app: AppHandle, user_requested: bool) {
    tauri::async_runtime::spawn(async move {
        if let Err(error) = check(&app, user_requested).await {
            eprintln!("update check failed: {error}");
            if user_requested {
                let _ = on_main_thread(&app, move || {
                    platform::show_notice("Update Check Failed", &error)
                })
                .await;
            }
        }
    });
}

async fn check(app: &AppHandle, user_requested: bool) -> Result<(), String> {
    let updater = app.updater().map_err(|error| error.to_string())?;
    let update = updater.check().await.map_err(|error| error.to_string())?;
    let Some(update) = update else {
        if user_requested {
            let version = app.package_info().version.to_string();
            on_main_thread(app, move || {
                platform::show_notice(
                    "You're up to date",
                    &format!("Just Notes {version} is the latest version."),
                )
            })
            .await??;
        }
        return Ok(());
    };

    let title = format!("Just Notes {} is available", update.version);
    let message = format!(
        "You have {}. The update is downloaded, installed, and relaunched automatically.",
        update.current_version
    );
    let confirmed = on_main_thread(app, move || {
        platform::ask_confirmation(&title, &message, "Install and Relaunch")
    })
    .await??;
    if !confirmed {
        return Ok(());
    }

    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|error| error.to_string())?;
    app.restart();
}

/// Runs a dialog on the main thread without a completion timeout; the answer
/// arrives whenever the user dismisses it.
async fn on_main_thread<T, F>(app: &AppHandle, operation: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let (sender, receiver) = mpsc::channel();
    app.run_on_main_thread(move || {
        let _ = sender.send(operation());
    })
    .map_err(|error| format!("Failed to reach the main thread: {error}"))?;
    tauri::async_runtime::spawn_blocking(move || receiver.recv())
        .await
        .map_err(|error| error.to_string())?
        .map_err(|_| "The update dialog was dropped before answering".to_string())
}
