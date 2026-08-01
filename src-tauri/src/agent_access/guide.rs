use std::{
    ffi::OsStr,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

#[cfg(test)]
use std::cell::Cell;

use super::{
    entry::{EntryKind, RenameChildOutcome},
    filesystem::SafeDirectory,
    model::InstallManifest,
    paths::GUIDE_DIR_NAME,
    AgentId,
};

pub(super) const GUIDE_NAME: &str = "SKILL.md";
pub(super) const MEMORY_NAME: &str = "MEMORY.md";
pub(super) const MANIFEST_NAME: &str = ".just-notes-install.json";
pub(super) const GUIDE_CONTENT: &str = include_str!("../../../assets/SKILL.md");

static STAGING_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static REMOVAL_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
thread_local! {
    static RESTORE_FAULT: Cell<u8> = const { Cell::new(0) };
}

pub(super) fn validate_managed_guide(
    guide: &SafeDirectory,
    agent: AgentId,
    memory_file: &Path,
) -> Result<(), String> {
    let manifest_bytes = guide
        .read_regular(MANIFEST_NAME)?
        .ok_or_else(|| "The Just Notes ownership manifest is missing".to_string())?;
    let manifest: InstallManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|_| "The Just Notes ownership manifest is invalid".to_string())?;
    if manifest != InstallManifest::expected(agent) {
        return Err("The guide folder is owned by a different installation".to_string());
    }

    for name in guide.list_names()? {
        if name != OsStr::new(GUIDE_NAME)
            && name != OsStr::new(MEMORY_NAME)
            && name != OsStr::new(MANIFEST_NAME)
        {
            return Err(format!(
                "The guide folder contains unexpected item {}",
                name.to_string_lossy()
            ));
        }
    }
    match guide.entry_kind(GUIDE_NAME)? {
        EntryKind::Missing | EntryKind::Regular => {}
        _ => return Err("SKILL.md is not a regular file".to_string()),
    }
    match guide.entry_kind(MEMORY_NAME)? {
        EntryKind::Missing => {}
        EntryKind::Symlink if guide.read_symlink(MEMORY_NAME)?.as_deref() == Some(memory_file) => {}
        _ => return Err("MEMORY.md is not the Just Notes shared-memory link".to_string()),
    }
    Ok(())
}

pub(super) fn update_managed_guide(
    guide: &SafeDirectory,
    agent: AgentId,
    memory_file: &Path,
) -> Result<(), String> {
    validate_managed_guide(guide, agent, memory_file)?;
    guide.write_atomic(GUIDE_NAME, "just-notes-guide", GUIDE_CONTENT.as_bytes())?;
    if guide.entry_kind(MEMORY_NAME)? == EntryKind::Missing {
        guide.create_symlink(MEMORY_NAME, memory_file)?;
    }
    write_manifest(guide, agent)
}

pub(super) fn install_new_guide(
    parent: &SafeDirectory,
    agent: AgentId,
    memory_file: &Path,
) -> Result<(), String> {
    if parent.entry_kind(GUIDE_DIR_NAME)? != EntryKind::Missing {
        return Err("The guide destination appeared during installation".to_string());
    }
    let sequence = STAGING_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let staging_name = format!(".just-notes-install-{}-{sequence}", std::process::id());
    let staging = parent.create_child_exclusive(&staging_name)?;
    let setup_result = (|| {
        staging.write_atomic(GUIDE_NAME, "just-notes-guide", GUIDE_CONTENT.as_bytes())?;
        staging.create_symlink(MEMORY_NAME, memory_file)?;
        write_manifest(&staging, agent)
    })();
    if let Err(error) = setup_result {
        cleanup_staging(parent, &staging, &staging_name);
        return Err(error);
    }
    match parent.rename_child(&staging_name, GUIDE_DIR_NAME) {
        Ok(RenameChildOutcome::Synced) => Ok(()),
        Ok(RenameChildOutcome::RenamedButUnsynced(error)) => Err(error),
        Err(error) => {
            cleanup_staging(parent, &staging, &staging_name);
            Err(error)
        }
    }
}

pub(super) fn remove_managed_guide(
    parent: &SafeDirectory,
    guide: &SafeDirectory,
    agent: AgentId,
    memory_file: &Path,
) -> Result<(), String> {
    validate_managed_guide(guide, agent, memory_file)?;
    let sequence = REMOVAL_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let quarantine_name = format!(".just-notes-remove-{}-{sequence}", std::process::id());
    let _quarantine_outcome = parent.rename_child(GUIDE_DIR_NAME, &quarantine_name)?;
    let result = (|| {
        for name in [GUIDE_NAME, MEMORY_NAME, MANIFEST_NAME] {
            if guide.entry_kind(name)? != EntryKind::Missing {
                guide.unlink(name)?;
            }
        }
        inject_removal_failure()?;
        parent.remove_child_dir(&quarantine_name)
    })();
    if let Err(error) = result {
        let manifest_error = if guide.entry_kind(MANIFEST_NAME)? == EntryKind::Missing {
            write_manifest(guide, agent).err()
        } else {
            None
        };
        let rename_error = match parent.rename_child(&quarantine_name, GUIDE_DIR_NAME) {
            Ok(RenameChildOutcome::Synced) => None,
            Ok(RenameChildOutcome::RenamedButUnsynced(error)) | Err(error) => Some(error),
        };
        return Err(recovery_error(error, manifest_error, rename_error));
    }
    Ok(())
}

fn write_manifest(guide: &SafeDirectory, agent: AgentId) -> Result<(), String> {
    #[cfg(test)]
    if RESTORE_FAULT.with(|fault| fault.replace(0) == 2) {
        return Err("Injected manifest restoration failure".to_string());
    }
    let mut json = serde_json::to_vec_pretty(&InstallManifest::expected(agent))
        .map_err(|error| format!("Failed to encode the Agent Access manifest: {error}"))?;
    json.push(b'\n');
    guide.write_atomic(MANIFEST_NAME, "just-notes-manifest", &json)
}

fn recovery_error(original: String, manifest: Option<String>, rename: Option<String>) -> String {
    let mut details = Vec::new();
    if let Some(error) = manifest {
        details.push(format!("manifest recovery failed: {error}"));
    }
    if let Some(error) = rename {
        details.push(format!("guide-name recovery failed: {error}"));
    }
    if details.is_empty() {
        original
    } else {
        format!("{original}; {}", details.join("; "))
    }
}

#[cfg(test)]
pub(super) fn fail_manifest_restore_during_next_removal() {
    RESTORE_FAULT.with(|fault| fault.set(1));
}

#[cfg(test)]
fn inject_removal_failure() -> Result<(), String> {
    let should_fail = RESTORE_FAULT.with(|fault| {
        if fault.get() != 1 {
            return false;
        }
        fault.set(2);
        true
    });
    if should_fail {
        Err("Injected removal failure".to_string())
    } else {
        Ok(())
    }
}

#[cfg(not(test))]
fn inject_removal_failure() -> Result<(), String> {
    Ok(())
}

fn cleanup_staging(parent: &SafeDirectory, staging: &SafeDirectory, staging_name: &str) {
    for name in [GUIDE_NAME, MEMORY_NAME, MANIFEST_NAME] {
        if staging
            .entry_kind(name)
            .is_ok_and(|kind| kind != EntryKind::Missing)
        {
            let _ = staging.unlink(name);
        }
    }
    let _ = parent.remove_child_dir(staging_name);
}
