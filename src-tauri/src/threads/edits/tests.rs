use std::{env, fs, time::UNIX_EPOCH};

use super::{archive_thread, delete_thread, rename_thread, restore_thread, update_segment_text};
use crate::app::AppPaths;
use crate::threads::repository::{list_archived_threads, list_threads};

fn temp_paths(name: &str) -> AppPaths {
    let stamp = UNIX_EPOCH.elapsed().unwrap().as_nanos();
    let data_dir = env::temp_dir().join(format!("just-notes-{name}-{stamp}"));
    AppPaths {
        threads_dir: data_dir.join("threads"),
        archived_dir: data_dir.join("archived"),
        data_dir,
    }
}

fn seed_thread(paths: &AppPaths, id: &str) {
    let dir = paths.thread_dir(id);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("thread.json"),
        format!(
            r#"{{"id":"{id}","title":"Test {id}","createdAtMs":1,"updatedAtMs":1,"status":"idle"}}"#
        ),
    )
    .unwrap();
    fs::write(dir.join("transcript.md"), "# Test\n").unwrap();
}

#[test]
fn archive_restore_delete_roundtrip_keeps_stores_separate() {
    let paths = temp_paths("archive-roundtrip");
    seed_thread(&paths, "thread-1");

    archive_thread(&paths, "thread-1").unwrap();
    assert!(!paths.thread_dir("thread-1").exists());
    assert!(paths.archived_thread_dir("thread-1").exists());
    assert!(list_threads(&paths).unwrap().is_empty());
    assert_eq!(list_archived_threads(&paths).unwrap().len(), 1);

    restore_thread(&paths, "thread-1").unwrap();
    assert!(paths.thread_dir("thread-1").exists());
    assert!(list_archived_threads(&paths).unwrap().is_empty());
    assert_eq!(list_threads(&paths).unwrap().len(), 1);

    let _ = fs::remove_dir_all(&paths.data_dir);
}

#[test]
fn permanent_delete_is_scoped_to_the_archive() {
    let paths = temp_paths("archive-delete-scope");
    seed_thread(&paths, "thread-1");

    // A live thread cannot be permanently deleted; only archiving (a reversible
    // move) acts on the live store.
    assert!(delete_thread(&paths, "thread-1").is_err());
    assert!(paths.thread_dir("thread-1").exists());

    archive_thread(&paths, "thread-1").unwrap();
    delete_thread(&paths, "thread-1").unwrap();
    assert!(!paths.archived_thread_dir("thread-1").exists());
    assert!(list_threads(&paths).unwrap().is_empty());
    assert!(list_archived_threads(&paths).unwrap().is_empty());

    let _ = fs::remove_dir_all(&paths.data_dir);
}

#[test]
fn title_rename_preserves_activity_timestamp() {
    let paths = temp_paths("rename-title-timestamp");
    seed_thread(&paths, "thread-1");

    let detail = rename_thread(&paths, "thread-1", "Renamed thread").unwrap();

    assert_eq!(detail.summary.title, "Renamed thread");
    assert_eq!(detail.summary.updated_at_ms, 1);
    assert!(
        fs::read_to_string(paths.thread_dir("thread-1").join("transcript.md"))
            .unwrap()
            .starts_with("# Renamed thread\n")
    );

    let _ = fs::remove_dir_all(&paths.data_dir);
}

#[test]
fn segment_edit_advances_activity_timestamp() {
    let paths = temp_paths("segment-edit-timestamp");
    seed_thread(&paths, "thread-1");
    fs::write(
        paths.thread_dir("thread-1").join("transcript.jsonl"),
        r#"{"speaker":"Speaker 0","source":"microphone","startMs":0,"endMs":1000,"text":"Original"}"#,
    )
    .unwrap();

    let detail = update_segment_text(&paths, "thread-1", 0, "Updated").unwrap();

    assert!(detail.summary.updated_at_ms > 1);
    assert_eq!(detail.segments[0].text, "Updated");

    let _ = fs::remove_dir_all(&paths.data_dir);
}
