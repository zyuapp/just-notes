use std::sync::{Arc, Mutex};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use objc2::{rc::Retained, runtime::Bool, MainThreadMarker};
use objc2_app_kit::{NSModalResponseCancel, NSModalResponseOK, NSOpenPanel};
use objc2_foundation::{
    NSData, NSString, NSURLBookmarkCreationOptions, NSURLBookmarkResolutionOptions, NSURL,
};
use tauri::AppHandle;

use super::run_on_main_thread;

pub(crate) struct FolderChoice {
    pub(crate) path: String,
    pub(crate) bookmark: String,
    scope: SecurityScopedFolder,
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

/// Owns the active Powerbox grant for the user-selected transcripts folder.
/// Clones share one scope so replacing the folder stops the previous grant once.
#[derive(Clone, Default)]
pub(crate) struct FolderAccessState(Arc<Mutex<Option<SecurityScopedFolder>>>);

impl FolderAccessState {
    pub(crate) fn install(&self, choice: FolderChoice) -> Result<(), String> {
        let mut active = self
            .0
            .lock()
            .map_err(|_| "The folder access state is unavailable".to_string())?;
        *active = Some(choice.scope);
        Ok(())
    }

    pub(crate) fn clear(&self) -> Result<(), String> {
        let mut active = self
            .0
            .lock()
            .map_err(|_| "The folder access state is unavailable".to_string())?;
        *active = None;
        Ok(())
    }
}

pub(crate) fn choose_folder(
    app: &AppHandle,
    prompt: &str,
    show_hidden_files: bool,
) -> Result<Option<FolderChoice>, String> {
    let prompt = prompt.to_string();
    run_on_main_thread(app, move || {
        choose_folder_on_main(&prompt, show_hidden_files)
    })
}

pub(crate) fn restore_folder_access(
    app: &AppHandle,
    bookmark: &str,
) -> Result<FolderChoice, String> {
    let bookmark = bookmark.to_string();
    run_on_main_thread(app, move || {
        let bytes = BASE64
            .decode(&bookmark)
            .map_err(|err| format!("The saved folder permission is invalid: {err}"))?;
        let data = NSData::from_vec(bytes);
        let mut stale = Bool::NO;
        // SAFETY: `stale` is a valid out pointer for the duration of the call.
        let url = unsafe {
            NSURL::URLByResolvingBookmarkData_options_relativeToURL_bookmarkDataIsStale_error(
                &data,
                NSURLBookmarkResolutionOptions::WithSecurityScope,
                None,
                &mut stale,
            )
        }
        .map_err(|err| format!("Failed to restore access to the transcripts folder: {err}"))?;
        let path = url_path(&url)?;
        let bookmark = if stale.as_bool() {
            create_bookmark(&url)?
        } else {
            bookmark
        };
        folder_choice(url, path, bookmark)
    })
}

fn choose_folder_on_main(
    prompt: &str,
    show_hidden_files: bool,
) -> Result<Option<FolderChoice>, String> {
    let marker = MainThreadMarker::new()
        .ok_or_else(|| "The folder picker must run on the main thread".to_string())?;
    let panel = NSOpenPanel::openPanel(marker);
    panel.setCanChooseDirectories(true);
    panel.setCanChooseFiles(false);
    panel.setAllowsMultipleSelection(false);
    panel.setShowsHiddenFiles(show_hidden_files);
    panel.setMessage(Some(&NSString::from_str(prompt)));

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
    let bookmark = create_bookmark(&url)?;
    folder_choice(url, path, bookmark).map(Some)
}

fn folder_choice(
    url: Retained<NSURL>,
    path: String,
    bookmark: String,
) -> Result<FolderChoice, String> {
    // SAFETY: The URL comes from NSOpenPanel or a bookmark resolved with
    // NSURLBookmarkResolutionWithSecurityScope.
    if !unsafe { url.startAccessingSecurityScopedResource() } {
        return Err("macOS did not grant access to the selected folder".to_string());
    }
    Ok(FolderChoice {
        path,
        bookmark,
        scope: SecurityScopedFolder { url },
    })
}

fn create_bookmark(url: &NSURL) -> Result<String, String> {
    let bookmark = url
        .bookmarkDataWithOptions_includingResourceValuesForKeys_relativeToURL_error(
            NSURLBookmarkCreationOptions::WithSecurityScope,
            None,
            None,
        )
        .map_err(|err| format!("Failed to save access to the transcripts folder: {err}"))?;
    Ok(BASE64.encode(bookmark.to_vec()))
}

fn url_path(url: &NSURL) -> Result<String, String> {
    url.path()
        .map(|path| path.to_string())
        .filter(|path| !path.is_empty())
        .ok_or_else(|| "The selected folder has no filesystem path".to_string())
}
