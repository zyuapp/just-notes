use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

use super::{format_io, ThreadMetadata, ThreadStatus};

pub(super) fn recording_directories(root: &Path) -> Result<Vec<PathBuf>, String> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut recordings = Vec::new();
    for entry in fs::read_dir(root).map_err(format_io("read legacy recordings"))? {
        let entry = entry.map_err(format_io("read legacy recording"))?;
        let file_type = entry
            .file_type()
            .map_err(format_io("inspect legacy recording"))?;
        if file_type.is_symlink() {
            return Err(format!(
                "Legacy data contains an unsupported symbolic link at {}",
                entry.path().display()
            ));
        }
        if file_type.is_dir() {
            recordings.push(entry.path());
        }
    }
    recordings.sort();
    Ok(recordings)
}

pub(super) fn validate_recording(recording: &Path) -> Result<String, String> {
    let folder_id = recording
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| format!("Invalid legacy recording folder: {}", recording.display()))?;
    let metadata_path = recording.join("thread.json");
    let metadata: ThreadMetadata = serde_json::from_slice(
        &fs::read(&metadata_path).map_err(format_io("read legacy recording metadata"))?,
    )
    .map_err(|error| format!("Invalid {}: {error}", metadata_path.display()))?;
    if metadata.id != folder_id {
        return Err(format!(
            "Legacy recording ID does not match its folder at {}",
            recording.display()
        ));
    }
    Ok(folder_id.to_string())
}

pub(super) fn trees_match(left: &Path, right: &Path) -> Result<bool, String> {
    Ok(tree_digest(left)? == tree_digest(right)?)
}

pub(super) fn tree_size(root: &Path) -> Result<u64, String> {
    tree_files(root)?
        .into_iter()
        .try_fold(0_u64, |total, path| {
            let size = fs::metadata(&path)
                .map_err(format_io("inspect recording file"))?
                .len();
            Ok(total.saturating_add(size))
        })
}

fn tree_digest(root: &Path) -> Result<Vec<u8>, String> {
    let mut files = tree_files(root)?;
    files.sort();
    let mut digest = Sha256::new();
    for path in files {
        let relative = path
            .strip_prefix(root)
            .map_err(|error| format!("Failed to inspect legacy recording: {error}"))?;
        update_frame(&mut digest, relative.as_os_str().as_encoded_bytes());
        if relative == Path::new("thread.json") {
            let mut metadata: ThreadMetadata = serde_json::from_slice(
                &fs::read(&path).map_err(format_io("read recording metadata"))?,
            )
            .map_err(|error| format!("Invalid {}: {error}", path.display()))?;
            metadata.id.clear();
            metadata.status = ThreadStatus::Idle;
            let normalized = serde_json::to_vec(&metadata)
                .map_err(|error| format!("Failed to normalize recording metadata: {error}"))?;
            update_frame(&mut digest, &normalized);
            continue;
        }
        digest.update(
            fs::metadata(&path)
                .map_err(format_io("inspect recording file"))?
                .len()
                .to_be_bytes(),
        );
        let mut file = fs::File::open(&path).map_err(format_io("open recording file"))?;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = file
                .read(&mut buffer)
                .map_err(format_io("hash recording file"))?;
            if read == 0 {
                break;
            }
            digest.update(&buffer[..read]);
        }
    }
    Ok(digest.finalize().to_vec())
}

fn update_frame(digest: &mut Sha256, bytes: &[u8]) {
    digest.update((bytes.len() as u64).to_be_bytes());
    digest.update(bytes);
}

fn tree_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    collect_tree_files(root, &mut files)?;
    Ok(files)
}

fn collect_tree_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(format_io("inspect recording data"))?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "Legacy data contains an unsupported symbolic link at {}",
            path.display()
        ));
    }
    if metadata.is_file() {
        files.push(path.to_path_buf());
        return Ok(());
    }
    if !metadata.is_dir() {
        return Err(format!("Unsupported legacy data at {}", path.display()));
    }
    for entry in fs::read_dir(path).map_err(format_io("read recording data"))? {
        collect_tree_files(
            &entry.map_err(format_io("read recording entry"))?.path(),
            files,
        )?;
    }
    Ok(())
}
