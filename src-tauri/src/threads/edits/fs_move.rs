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

#[cfg(test)]
mod tests {
    use std::{env, fs, path::Path, time::UNIX_EPOCH};

    use super::{copy_dir_all, move_thread_dir};

    fn temp_root(name: &str) -> std::path::PathBuf {
        let stamp = UNIX_EPOCH.elapsed().unwrap().as_nanos();
        env::temp_dir().join(format!("just-notes-fsmove-{name}-{stamp}"))
    }

    #[test]
    fn copy_dir_all_reproduces_files_subdirs_and_symlinks() {
        let root = temp_root("copy");
        let source = root.join("source");
        let dest = root.join("dest");
        fs::create_dir_all(source.join("work")).unwrap();
        fs::write(source.join("thread.json"), "{}").unwrap();
        fs::write(source.join("work/audio.bin"), b"pcm").unwrap();
        std::os::unix::fs::symlink("thread.json", source.join("latest.json")).unwrap();

        copy_dir_all(&source, &dest).unwrap();

        assert_eq!(fs::read_to_string(dest.join("thread.json")).unwrap(), "{}");
        assert_eq!(
            fs::read(dest.join("work/audio.bin")).unwrap(),
            b"pcm".to_vec()
        );
        // The symlink is recreated as a link, not dereferenced into a copy.
        assert_eq!(
            fs::read_link(dest.join("latest.json")).unwrap(),
            Path::new("thread.json")
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn move_thread_dir_refuses_to_overwrite_an_existing_destination() {
        let root = temp_root("collision");
        let source = root.join("source");
        let dest = root.join("dest");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("thread.json"), "source").unwrap();
        fs::create_dir_all(&dest).unwrap();
        fs::write(dest.join("thread.json"), "dest").unwrap();

        assert!(move_thread_dir(&source, &dest).is_err());
        // Neither store was touched: both originals remain intact.
        assert_eq!(
            fs::read_to_string(source.join("thread.json")).unwrap(),
            "source"
        );
        assert_eq!(
            fs::read_to_string(dest.join("thread.json")).unwrap(),
            "dest"
        );

        let _ = fs::remove_dir_all(&root);
    }
}
