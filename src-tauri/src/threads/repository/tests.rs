use std::{env, fs, time::UNIX_EPOCH};

use super::{list_threads, metadata_path, read_thread_metadata, reset_stale_recording_threads};
use crate::app::AppPaths;
use crate::threads::ThreadStatus;

fn temp_paths(name: &str) -> AppPaths {
    let stamp = UNIX_EPOCH.elapsed().unwrap().as_nanos();
    let data_dir = env::temp_dir().join(format!("just-notes-{name}-{stamp}"));
    AppPaths {
        threads_dir: data_dir.join("threads"),
        archived_dir: data_dir.join("archived"),
        data_dir,
    }
}

fn seed_thread_with_status(paths: &AppPaths, id: &str, status: &str) {
    let dir = paths.thread_dir(id);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("thread.json"),
        format!(
            r#"{{"id":"{id}","title":"Test {id}","createdAtMs":1,"updatedAtMs":1,"status":"{status}"}}"#
        ),
    )
    .unwrap();
}

#[test]
fn transcribing_metadata_from_an_earlier_version_still_loads() {
    let paths = temp_paths("stale-transcribing-loads");
    seed_thread_with_status(&paths, "thread-1", "transcribing");

    let threads = list_threads(&paths).unwrap();

    assert_eq!(threads.len(), 1);
    assert!(matches!(threads[0].status, ThreadStatus::Transcribing));

    let _ = fs::remove_dir_all(&paths.data_dir);
}

#[test]
fn startup_clears_a_stranded_transcribing_thread() {
    let paths = temp_paths("stale-transcribing-reset");
    seed_thread_with_status(&paths, "thread-1", "transcribing");

    reset_stale_recording_threads(&paths).unwrap();

    let metadata = read_thread_metadata(&metadata_path(&paths.thread_dir("thread-1"))).unwrap();
    assert!(matches!(metadata.status, ThreadStatus::Idle));

    let _ = fs::remove_dir_all(&paths.data_dir);
}
