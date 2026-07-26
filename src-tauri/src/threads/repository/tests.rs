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

fn seed_thread_with_calendar(paths: &AppPaths, id: &str) {
    let dir = paths.thread_dir(id);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("thread.json"),
        format!(
            r#"{{"id":"{id}","title":"Pricing sync","createdAtMs":1,"updatedAtMs":1,"status":"idle","calendar":{{"eventId":"event-1:1000","calendarId":"calendar-1","attendees":["Alice","Bob"],"startAtMs":1000,"endAtMs":2000}}}}"#
        ),
    )
    .unwrap();
}

#[test]
fn listed_threads_carry_the_calendar_provenance_from_disk() {
    let paths = temp_paths("calendar-provenance-listed");
    seed_thread_with_calendar(&paths, "thread-1");

    let threads = list_threads(&paths).unwrap();

    let calendar = threads[0]
        .calendar
        .as_ref()
        .expect("a meeting thread surfaces its calendar provenance");
    assert_eq!(calendar.event_id, "event-1:1000");
    assert_eq!(calendar.attendees, vec!["Alice", "Bob"]);
    assert_eq!(calendar.start_at_ms, 1_000);

    let _ = fs::remove_dir_all(&paths.data_dir);
}

#[test]
fn a_thread_with_an_unrecognized_provenance_shape_still_lists() {
    let paths = temp_paths("calendar-provenance-partial");
    let dir = paths.thread_dir("thread-1");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("thread.json"),
        r#"{"id":"thread-1","title":"Pricing sync","createdAtMs":1,"updatedAtMs":1,"status":"idle","calendar":{"attendees":["Alice"]}}"#,
    )
    .unwrap();

    let threads = list_threads(&paths).unwrap();

    assert_eq!(
        threads.len(),
        1,
        "a partial provenance must not hide a thread"
    );
    let calendar = threads[0].calendar.as_ref().unwrap();
    assert_eq!(calendar.attendees, vec!["Alice"]);
    assert_eq!(calendar.event_id, "");
    assert_eq!(calendar.start_at_ms, 0);

    let _ = fs::remove_dir_all(&paths.data_dir);
}

#[test]
fn threads_stored_before_calendar_provenance_still_load() {
    let paths = temp_paths("calendar-provenance-absent");
    seed_thread_with_status(&paths, "thread-1", "idle");

    let threads = list_threads(&paths).unwrap();

    assert_eq!(threads.len(), 1);
    assert!(threads[0].calendar.is_none());

    let _ = fs::remove_dir_all(&paths.data_dir);
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
