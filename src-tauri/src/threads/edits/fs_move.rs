use std::{fs, path::Path};

// Move a thread directory between the live and archived stores. Within one
// filesystem this is an atomic rename. When the stores sit on different volumes
// (a custom transcripts folder on another drive) rename can't cross the
// boundary, so fall back to an all-or-nothing copy that never leaves the thread
// in both stores — the live-corpus invariant depends on that.
pub(super) fn move_thread_dir(source: &Path, dest: &Path) -> Result<(), String> {
    if dest.exists() {
        return Err(format!("{} already exists", dest.display()));
    }
    let parent = dest
        .parent()
        .ok_or_else(|| format!("{} has no parent directory", dest.display()))?;
    fs::create_dir_all(parent)
        .map_err(|err| format!("Failed to create {}: {err}", parent.display()))?;

    if same_filesystem(source, parent)? {
        return fs::rename(source, dest).map_err(|err| {
            format!(
                "Failed to move {} to {}: {err}",
                source.display(),
                dest.display()
            )
        });
    }

    copy_dir_all(source, dest).inspect_err(|_| {
        let _ = fs::remove_dir_all(dest);
    })?;
    if let Err(err) = fs::remove_dir_all(source) {
        let _ = fs::remove_dir_all(dest);
        return Err(format!(
            "Failed to move {} to {}: source could not be removed: {err}",
            source.display(),
            dest.display()
        ));
    }
    Ok(())
}

fn same_filesystem(source: &Path, dest_parent: &Path) -> Result<bool, String> {
    use std::os::unix::fs::MetadataExt;
    let source_dev = fs::metadata(source)
        .map_err(|err| format!("Failed to inspect {}: {err}", source.display()))?
        .dev();
    let dest_dev = fs::metadata(dest_parent)
        .map_err(|err| format!("Failed to inspect {}: {err}", dest_parent.display()))?
        .dev();
    Ok(source_dev == dest_dev)
}

fn copy_dir_all(source: &Path, dest: &Path) -> Result<(), String> {
    fs::create_dir_all(dest)
        .map_err(|err| format!("Failed to create {}: {err}", dest.display()))?;
    for entry in
        fs::read_dir(source).map_err(|err| format!("Failed to read {}: {err}", source.display()))?
    {
        let entry = entry.map_err(|err| format!("Failed to read {}: {err}", source.display()))?;
        let file_type = entry
            .file_type()
            .map_err(|err| format!("Failed to inspect {}: {err}", entry.path().display()))?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        if file_type.is_symlink() {
            let target = fs::read_link(&from)
                .map_err(|err| format!("Failed to read link {}: {err}", from.display()))?;
            std::os::unix::fs::symlink(&target, &to)
                .map_err(|err| format!("Failed to recreate link {}: {err}", to.display()))?;
        } else if file_type.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            fs::copy(&from, &to).map(|_| ()).map_err(|err| {
                format!(
                    "Failed to copy {} to {}: {err}",
                    from.display(),
                    to.display()
                )
            })?;
        }
    }
    Ok(())
}
