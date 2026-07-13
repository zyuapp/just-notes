use std::path::Path;

use objc2::{rc::Retained, MainThreadMarker};
use objc2_app_kit::{NSModalResponseCancel, NSModalResponseOK, NSOpenPanel};
use objc2_foundation::{NSString, NSURL};
use tauri::AppHandle;

use super::run_on_main_thread_without_timeout;

pub(crate) struct FolderChoice {
    pub(crate) path: String,
    _scope: SecurityScopedFolder,
}

struct SecurityScopedFolder {
    url: Retained<NSURL>,
}

impl Drop for SecurityScopedFolder {
    fn drop(&mut self) {
        // SAFETY: Each stored URL is inserted only after a successful matching
        // startAccessingSecurityScopedResource call.
        unsafe { self.url.stopAccessingSecurityScopedResource() };
    }
}

pub(crate) fn choose_folder(
    app: &AppHandle,
    prompt: &str,
    show_hidden_files: bool,
    initial_directory: Option<&Path>,
    confirm_label: Option<&str>,
) -> Result<Option<FolderChoice>, String> {
    let prompt = prompt.to_string();
    let initial_directory = initial_directory.map(Path::to_path_buf);
    let confirm_label = confirm_label.map(str::to_string);
    run_on_main_thread_without_timeout(app, move || {
        choose_folder_on_main(
            &prompt,
            show_hidden_files,
            initial_directory.as_deref(),
            confirm_label.as_deref(),
        )
    })
}

fn choose_folder_on_main(
    prompt: &str,
    show_hidden_files: bool,
    initial_directory: Option<&Path>,
    confirm_label: Option<&str>,
) -> Result<Option<FolderChoice>, String> {
    let marker = MainThreadMarker::new()
        .ok_or_else(|| "The folder picker must run on the main thread".to_string())?;
    let panel = NSOpenPanel::openPanel(marker);
    panel.setCanChooseDirectories(true);
    panel.setCanChooseFiles(false);
    panel.setAllowsMultipleSelection(false);
    panel.setShowsHiddenFiles(show_hidden_files);
    panel.setMessage(Some(&NSString::from_str(prompt)));
    if let Some(label) = confirm_label {
        panel.setPrompt(Some(&NSString::from_str(label)));
    }
    if let Some(directory) = initial_directory {
        panel.setDirectoryURL(Some(&NSURL::fileURLWithPath(&NSString::from_str(
            &directory.display().to_string(),
        ))));
    }

    let response = panel.runModal();
    if response == NSModalResponseCancel {
        return Ok(None);
    }
    if response != NSModalResponseOK {
        return Err("The folder picker could not be displayed".to_string());
    }
    let url = panel
        .URL()
        .ok_or_else(|| "The folder picker did not return a folder".to_string())?;
    let path = url_path(&url)?;
    folder_choice(url, path).map(Some)
}

fn folder_choice(url: Retained<NSURL>, path: String) -> Result<FolderChoice, String> {
    // SAFETY: The URL comes from NSOpenPanel after explicit user selection.
    if !unsafe { url.startAccessingSecurityScopedResource() } {
        return Err("macOS did not grant access to the selected folder".to_string());
    }
    Ok(FolderChoice {
        path,
        _scope: SecurityScopedFolder { url },
    })
}

fn url_path(url: &NSURL) -> Result<String, String> {
    url.path()
        .map(|path| path.to_string())
        .filter(|path| !path.is_empty())
        .ok_or_else(|| "The selected folder has no filesystem path".to_string())
}
