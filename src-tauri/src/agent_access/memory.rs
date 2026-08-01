use std::ffi::OsStr;

use super::{
    entry::EntryKind, filesystem::SafeDirectory, guide::MEMORY_NAME, paths::AgentAccessPaths,
};

const SHARED_DIR_NAME: &str = "agent-access";

pub(super) fn ensure(paths: &AgentAccessPaths) -> Result<(), String> {
    let data = SafeDirectory::open_absolute(paths.data_dir())?;
    let shared = data.create_child(SHARED_DIR_NAME)?;
    validate_directory(&shared)?;
    shared.create_empty_file(MEMORY_NAME)
}

pub(super) struct SharedMemoryRemoval {
    data: SafeDirectory,
    shared: Option<SafeDirectory>,
}

pub(super) fn prepare_removal(paths: &AgentAccessPaths) -> Result<SharedMemoryRemoval, String> {
    let data = SafeDirectory::open_absolute(paths.data_dir())?;
    let shared = data.open_child(SHARED_DIR_NAME)?;
    if let Some(shared) = &shared {
        validate_directory(shared)?;
    }
    Ok(SharedMemoryRemoval { data, shared })
}

impl SharedMemoryRemoval {
    pub(super) fn remove(self) -> Result<(), String> {
        let Some(shared) = self.shared else {
            return Ok(());
        };
        if shared.entry_kind(MEMORY_NAME)? == EntryKind::Regular {
            shared.unlink(MEMORY_NAME)?;
        }
        self.data.remove_child_dir(SHARED_DIR_NAME)
    }
}

fn validate_directory(shared: &SafeDirectory) -> Result<(), String> {
    for name in shared.list_names()? {
        if name != OsStr::new(MEMORY_NAME) {
            return Err(format!(
                "The shared-memory folder contains unexpected item {}",
                name.to_string_lossy()
            ));
        }
    }
    match shared.entry_kind(MEMORY_NAME)? {
        EntryKind::Missing | EntryKind::Regular => Ok(()),
        _ => Err("The shared MEMORY.md is not a regular file".to_string()),
    }
}
