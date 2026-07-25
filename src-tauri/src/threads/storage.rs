//! Disk footprint of the thread library, and reclaiming the raw audio a
//! finished recording no longer needs.

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::app::AppPaths;

use super::{
    repository::{metadata_path, read_thread_metadata, thread_dirs_in},
    RecordingAudioPaths,
};

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub(crate) struct StorageUsage {
    pub(crate) total_bytes: u64,
    pub(crate) raw_audio_bytes: u64,
    /// Raw audio held by threads that are neither recording nor transcribing,
    /// so it can be deleted without disturbing work in progress.
    pub(crate) reclaimable_bytes: u64,
    pub(crate) thread_count: u64,
}

pub(crate) fn usage(paths: &AppPaths) -> Result<StorageUsage, String> {
    let mut usage = StorageUsage::default();
    for dir in thread_dirs(paths)? {
        usage.thread_count += 1;
        usage.total_bytes += directory_size(&dir, 0);
        let raw_audio = raw_audio_size(&dir);
        usage.raw_audio_bytes += raw_audio;
        if is_reclaimable(&dir) {
            usage.reclaimable_bytes += raw_audio;
        }
    }
    Ok(usage)
}

/// Deletes `mic.wav`/`system.wav` for every idle thread and reports the bytes
/// freed. Those threads can no longer be re-transcribed afterwards.
pub(crate) fn delete_reclaimable_raw_audio(paths: &AppPaths) -> Result<u64, String> {
    let mut freed = 0;
    for dir in thread_dirs(paths)? {
        if !is_reclaimable(&dir) {
            continue;
        }
        let size = raw_audio_size(&dir);
        RecordingAudioPaths::for_thread_dir(&dir).remove_files()?;
        freed += size;
    }
    Ok(freed)
}

fn thread_dirs(paths: &AppPaths) -> Result<Vec<PathBuf>, String> {
    paths.ensure()?;
    let mut dirs = thread_dirs_in(&paths.threads_dir)?;
    dirs.extend(thread_dirs_in(&paths.archived_dir)?);
    Ok(dirs)
}

/// A thread mid-recording or mid-transcription still needs its WAVs. Unreadable
/// metadata is treated as busy so a torn write never costs the user audio.
fn is_reclaimable(thread_dir: &Path) -> bool {
    read_thread_metadata(&metadata_path(thread_dir))
        .map(|metadata| !metadata.status.is_busy())
        .unwrap_or(false)
}

fn raw_audio_size(thread_dir: &Path) -> u64 {
    let audio = RecordingAudioPaths::for_thread_dir(thread_dir);
    file_size(audio.mic_path()) + file_size(audio.system_path())
}

// The transcripts folder is user-visible and users are invited to open it, so a
// symlink pointing back up the tree is reachable. `is_dir` follows symlinks, so
// the walk is depth-bounded rather than trusting the shape of the directory.
const MAX_WALK_DEPTH: u32 = 8;

fn directory_size(dir: &Path, depth: u32) -> u64 {
    if depth >= MAX_WALK_DEPTH {
        return 0;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| {
            let path = entry.path();
            if entry.file_type().is_ok_and(|kind| kind.is_symlink()) {
                0
            } else if path.is_dir() {
                directory_size(&path, depth + 1)
            } else {
                file_size(&path)
            }
        })
        .sum()
}

fn file_size(path: &Path) -> u64 {
    fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::threads::model::ThreadStatus;

    fn temp_paths(name: &str) -> AppPaths {
        let dir =
            std::env::temp_dir().join(format!("just-notes-storage-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let paths = AppPaths::from_data_dir(dir);
        paths.ensure().unwrap();
        paths
    }

    fn write_thread(dir: &Path, id: &str, status: ThreadStatus, mic: &[u8], system: &[u8]) {
        fs::create_dir_all(dir).unwrap();
        let metadata = format!(
            r#"{{"id":"{id}","title":"t","createdAtMs":0,"updatedAtMs":0,"status":"{}","durationMs":0}}"#,
            match status {
                ThreadStatus::Idle => "idle",
                ThreadStatus::Recording => "recording",
                ThreadStatus::Transcribing => "transcribing",
            }
        );
        fs::write(dir.join("thread.json"), metadata).unwrap();
        fs::write(dir.join("mic.wav"), mic).unwrap();
        fs::write(dir.join("system.wav"), system).unwrap();
    }

    #[test]
    fn usage_counts_threads_audio_and_archived_copies() {
        let paths = temp_paths("usage");
        write_thread(
            &paths.thread_dir("a"),
            "a",
            ThreadStatus::Idle,
            &[0; 100],
            &[0; 50],
        );
        write_thread(
            &paths.archived_thread_dir("b"),
            "b",
            ThreadStatus::Idle,
            &[0; 10],
            &[0; 10],
        );

        let usage = usage(&paths).unwrap();

        assert_eq!(usage.thread_count, 2);
        assert_eq!(usage.raw_audio_bytes, 170);
        assert_eq!(usage.reclaimable_bytes, 170);
        assert!(usage.total_bytes >= usage.raw_audio_bytes);
        let _ = fs::remove_dir_all(&paths.data_dir);
    }

    #[test]
    fn unreadable_metadata_keeps_the_raw_audio() {
        let paths = temp_paths("torn");
        let dir = paths.thread_dir("a");
        write_thread(&dir, "a", ThreadStatus::Idle, &[0; 100], &[]);
        // A thread.json caught mid-write must not be read as "idle".
        fs::write(dir.join("thread.json"), "{\"id\":\"a\",\"tit").unwrap();

        assert_eq!(usage(&paths).unwrap().reclaimable_bytes, 0);
        assert_eq!(delete_reclaimable_raw_audio(&paths).unwrap(), 0);
        assert!(dir.join("mic.wav").exists());
        let _ = fs::remove_dir_all(&paths.data_dir);
    }

    #[test]
    fn a_symlink_loop_does_not_hang_the_walk() {
        let paths = temp_paths("symlink");
        let dir = paths.thread_dir("a");
        write_thread(&dir, "a", ThreadStatus::Idle, &[0; 100], &[]);
        std::os::unix::fs::symlink(&paths.threads_dir, dir.join("loop")).unwrap();

        let usage = usage(&paths).unwrap();

        assert_eq!(usage.thread_count, 1);
        assert_eq!(usage.raw_audio_bytes, 100);
        let _ = fs::remove_dir_all(&paths.data_dir);
    }

    #[test]
    fn busy_threads_keep_their_raw_audio() {
        let paths = temp_paths("busy");
        write_thread(
            &paths.thread_dir("a"),
            "a",
            ThreadStatus::Idle,
            &[0; 100],
            &[],
        );
        write_thread(
            &paths.thread_dir("b"),
            "b",
            ThreadStatus::Recording,
            &[0; 400],
            &[],
        );

        let usage = usage(&paths).unwrap();
        assert_eq!(usage.raw_audio_bytes, 500);
        assert_eq!(usage.reclaimable_bytes, 100);

        let freed = delete_reclaimable_raw_audio(&paths).unwrap();

        assert_eq!(freed, 100);
        assert!(!paths.thread_dir("a").join("mic.wav").exists());
        assert!(paths.thread_dir("b").join("mic.wav").exists());
        let _ = fs::remove_dir_all(&paths.data_dir);
    }
}
