use std::{fs, path::Path};

const IMPORT_DIRS: [&str; 4] = ["threads", "archived", "models", "engine"];
const STAGING_DIR: &str = ".legacy-import-staging";

/// Imports the previous unsandboxed data layout after the user grants access
/// to its hidden folder. Existing app data is never merged or overwritten.
pub(crate) fn import_legacy_data(
    source: &Path,
    custom_threads_source: Option<&Path>,
    destination: &Path,
) -> Result<(), String> {
    validate_source(source)?;
    validate_destination(destination)?;

    let staging = destination.join(STAGING_DIR);
    if staging.exists() {
        fs::remove_dir_all(&staging)
            .map_err(|err| format!("Failed to clear an incomplete legacy import: {err}"))?;
    }
    fs::create_dir(&staging)
        .map_err(|err| format!("Failed to prepare the legacy import: {err}"))?;

    let result = (|| {
        for name in IMPORT_DIRS {
            if name == "threads" {
                if let Some(custom_source) = custom_threads_source {
                    copy_tree(custom_source, &staging.join(name))?;
                    continue;
                }
            }
            let source_dir = source.join(name);
            if source_dir.exists() {
                copy_tree(&source_dir, &staging.join(name))?;
            }
        }
        install_staged_data(&staging, destination)
    })();

    let _ = fs::remove_dir_all(&staging);
    result
}

fn validate_source(source: &Path) -> Result<(), String> {
    if source.file_name().and_then(|name| name.to_str()) != Some(".just-notes") {
        return Err("Choose the existing .just-notes folder".to_string());
    }
    if !IMPORT_DIRS.iter().any(|name| source.join(name).exists()) {
        return Err("The selected folder does not contain Just Notes data".to_string());
    }
    Ok(())
}

fn validate_destination(destination: &Path) -> Result<(), String> {
    for name in IMPORT_DIRS {
        let path = destination.join(name);
        if path.exists() && directory_has_entries(&path)? {
            return Err(
                "Legacy data can only be imported before creating recordings in this version"
                    .to_string(),
            );
        }
    }
    Ok(())
}

fn directory_has_entries(path: &Path) -> Result<bool, String> {
    let mut entries =
        fs::read_dir(path).map_err(|err| format!("Failed to inspect {}: {err}", path.display()))?;
    Ok(entries.next().is_some())
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(source)
        .map_err(|err| format!("Failed to inspect {}: {err}", source.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "Legacy data contains an unsupported symbolic link at {}",
            source.display()
        ));
    }
    if metadata.is_file() {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("Failed to prepare {}: {err}", parent.display()))?;
        }
        fs::copy(source, destination)
            .map_err(|err| format!("Failed to import {}: {err}", source.display()))?;
        return Ok(());
    }
    if !metadata.is_dir() {
        return Err(format!("Unsupported legacy data at {}", source.display()));
    }
    fs::create_dir_all(destination)
        .map_err(|err| format!("Failed to prepare {}: {err}", destination.display()))?;
    for entry in
        fs::read_dir(source).map_err(|err| format!("Failed to read {}: {err}", source.display()))?
    {
        let entry = entry.map_err(|err| format!("Failed to read legacy data: {err}"))?;
        copy_tree(&entry.path(), &destination.join(entry.file_name()))?;
    }
    Ok(())
}

fn install_staged_data(staging: &Path, destination: &Path) -> Result<(), String> {
    let mut installed = Vec::new();
    for name in IMPORT_DIRS {
        let staged = staging.join(name);
        if !staged.exists() {
            continue;
        }
        let target = destination.join(name);
        if target.exists() {
            fs::remove_dir(&target)
                .map_err(|err| format!("Failed to replace {}: {err}", target.display()))?;
        }
        if let Err(err) = fs::rename(&staged, &target) {
            let rollback_errors = rollback_install(staging, destination, &installed, name);
            let rollback_detail = if rollback_errors.is_empty() {
                String::new()
            } else {
                format!(" Rollback was incomplete: {}", rollback_errors.join("; "))
            };
            return Err(format!(
                "Failed to install imported data: {err}.{rollback_detail}"
            ));
        }
        installed.push(name);
    }
    Ok(())
}

fn rollback_install(
    staging: &Path,
    destination: &Path,
    installed: &[&str],
    failed_name: &str,
) -> Vec<String> {
    let mut errors = Vec::new();
    for name in installed.iter().rev() {
        let installed_path = destination.join(name);
        if let Err(err) = fs::rename(&installed_path, staging.join(name)) {
            errors.push(format!(
                "could not restore {}: {err}",
                installed_path.display()
            ));
            continue;
        }
        if let Err(err) = fs::create_dir(&installed_path) {
            errors.push(format!(
                "could not recreate {}: {err}",
                installed_path.display()
            ));
        }
    }
    let failed_target = destination.join(failed_name);
    if let Err(err) = fs::create_dir(&failed_target) {
        errors.push(format!(
            "could not recreate {}: {err}",
            failed_target.display()
        ));
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::import_legacy_data;
    use std::{env, fs};

    fn roots(tag: &str) -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
        let root =
            env::temp_dir().join(format!("just-notes-migration-{tag}-{}", std::process::id()));
        let source = root.join(".just-notes");
        let destination = root.join("container");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&destination).unwrap();
        (root, source, destination)
    }

    #[test]
    fn imports_known_data_without_overwriting() {
        let (root, source, destination) = roots("success");
        fs::create_dir_all(source.join("threads/thread-1")).unwrap();
        fs::write(source.join("threads/thread-1/transcript.json"), "{}").unwrap();
        fs::create_dir_all(destination.join("threads")).unwrap();

        import_legacy_data(&source, None, &destination).unwrap();

        assert!(destination
            .join("threads/thread-1/transcript.json")
            .exists());
        assert!(!destination.join(".legacy-import-staging").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn refuses_to_merge_with_existing_recordings() {
        let (root, source, destination) = roots("conflict");
        fs::create_dir_all(source.join("threads/old")).unwrap();
        fs::create_dir_all(destination.join("threads/new")).unwrap();

        let error = import_legacy_data(&source, None, &destination).unwrap_err();

        assert!(error.contains("before creating recordings"));
        assert!(destination.join("threads/new").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn requires_the_legacy_folder_itself() {
        let (root, source, destination) = roots("wrong-folder");
        fs::create_dir_all(source.join("threads/old")).unwrap();

        let error = import_legacy_data(&source.join("threads"), None, &destination).unwrap_err();

        assert!(error.contains(".just-notes folder"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn imports_custom_legacy_transcripts_into_container() {
        let (root, source, destination) = roots("custom-threads");
        fs::create_dir_all(source.join("archived/old-archive")).unwrap();
        let custom = root.join("old-custom-notes");
        fs::create_dir_all(custom.join("thread-2")).unwrap();
        fs::write(custom.join("thread-2/transcript.json"), "{}").unwrap();

        import_legacy_data(&source, Some(&custom), &destination).unwrap();

        assert!(destination
            .join("threads/thread-2/transcript.json")
            .exists());
        assert!(destination.join("archived/old-archive").exists());
        let _ = fs::remove_dir_all(root);
    }
}
