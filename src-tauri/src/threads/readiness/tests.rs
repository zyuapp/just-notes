use std::{
    env, fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use super::{
    is_thread_retrievable, mark_recording_aborted, mark_recording_finished, mark_recording_started,
    migrate_retrieval_readiness, storage::ThreadDirectory,
};
use crate::{
    app::AppPaths,
    threads::{
        repository::{metadata_path, read_thread_metadata},
        RetrievalReadiness,
    },
};

fn temp_paths(name: &str) -> AppPaths {
    let stamp = UNIX_EPOCH.elapsed().unwrap().as_nanos();
    let data_dir = env::temp_dir().join(format!("just-notes-readiness-{name}-{stamp}"));
    AppPaths {
        threads_dir: data_dir.join("threads"),
        archived_dir: data_dir.join("archived"),
        data_dir,
    }
}

#[test]
fn aborted_resume_restores_readiness_from_the_unchanged_transcript() {
    let paths = temp_paths("aborted-resume");
    let thread_dir = seed_legacy_thread(&paths.threads_dir, "thread-1", "idle");
    write_valid_transcript(&thread_dir);
    migrate_retrieval_readiness(&paths).unwrap();

    mark_recording_started(&thread_dir).unwrap();
    mark_recording_aborted(&thread_dir).unwrap();

    assert!(is_thread_retrievable(&thread_dir).unwrap());
    let _ = fs::remove_dir_all(&paths.data_dir);
}

fn seed_legacy_thread(root: &Path, id: &str, status: &str) -> PathBuf {
    let thread_dir = root.join(id);
    fs::create_dir_all(&thread_dir).unwrap();
    fs::write(
        thread_dir.join("thread.json"),
        format!(
            r#"{{"id":"{id}","title":"Test","createdAtMs":1,"updatedAtMs":1,"status":"{status}"}}"#
        ),
    )
    .unwrap();
    thread_dir
}

fn write_valid_transcript(thread_dir: &Path) {
    fs::write(
        thread_dir.join("transcript.jsonl"),
        r#"{"speaker":"You","source":"mic","startMs":0,"endMs":1000,"text":"Hello"}
"#,
    )
    .unwrap();
}

#[test]
fn migration_marks_only_valid_idle_active_threads_ready() {
    let paths = temp_paths("migration");
    let ready = seed_legacy_thread(&paths.threads_dir, "ready", "idle");
    write_valid_transcript(&ready);
    let busy = seed_legacy_thread(&paths.threads_dir, "busy", "recording");
    write_valid_transcript(&busy);
    let corrupt = seed_legacy_thread(&paths.threads_dir, "corrupt", "idle");
    fs::write(corrupt.join("transcript.jsonl"), "not json\n").unwrap();
    let archived = seed_legacy_thread(&paths.archived_dir, "archived", "idle");
    write_valid_transcript(&archived);

    migrate_retrieval_readiness(&paths).unwrap();

    assert_eq!(readiness(&ready), RetrievalReadiness::Ready);
    assert_eq!(readiness(&busy), RetrievalReadiness::Unavailable);
    assert_eq!(readiness(&corrupt), RetrievalReadiness::Unavailable);
    assert_eq!(readiness(&archived), RetrievalReadiness::Unknown);
    let _ = fs::remove_dir_all(&paths.data_dir);
}

#[test]
fn recording_start_clears_ready_and_only_a_valid_finish_restores_it() {
    let paths = temp_paths("lifecycle");
    let thread_dir = seed_legacy_thread(&paths.threads_dir, "thread-1", "idle");
    write_valid_transcript(&thread_dir);
    migrate_retrieval_readiness(&paths).unwrap();
    assert!(is_thread_retrievable(&thread_dir).unwrap());

    mark_recording_started(&thread_dir).unwrap();
    assert_eq!(readiness(&thread_dir), RetrievalReadiness::Unavailable);
    assert!(!is_thread_retrievable(&thread_dir).unwrap());

    assert!(mark_recording_finished(&thread_dir, 2_000, false).unwrap());
    assert_eq!(readiness(&thread_dir), RetrievalReadiness::Ready);
    assert_eq!(metadata(&thread_dir).duration_ms, 2_000);
    let _ = fs::remove_dir_all(&paths.data_dir);
}

#[test]
fn empty_corrupt_and_symlinked_transcripts_are_not_retrievable() {
    let paths = temp_paths("invalid");
    let empty = seed_legacy_thread(&paths.threads_dir, "empty", "idle");
    fs::write(empty.join("transcript.jsonl"), "").unwrap();
    migrate_retrieval_readiness(&paths).unwrap();
    assert_eq!(readiness(&empty), RetrievalReadiness::Unavailable);

    let corrupt = seed_legacy_thread(&paths.threads_dir, "corrupt", "idle");
    fs::write(corrupt.join("transcript.jsonl"), "not json\n").unwrap();
    migrate_retrieval_readiness(&paths).unwrap();
    assert_eq!(readiness(&corrupt), RetrievalReadiness::Unavailable);

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let linked = seed_legacy_thread(&paths.threads_dir, "linked", "idle");
        let target = paths.data_dir.join("transcript-target.jsonl");
        write_valid_transcript(&paths.data_dir);
        fs::rename(paths.data_dir.join("transcript.jsonl"), &target).unwrap();
        symlink(target, linked.join("transcript.jsonl")).unwrap();
        migrate_retrieval_readiness(&paths).unwrap();
        assert_eq!(readiness(&linked), RetrievalReadiness::Unavailable);
    }
    let _ = fs::remove_dir_all(&paths.data_dir);
}

#[cfg(unix)]
#[test]
fn migration_does_not_follow_or_replace_symlinked_metadata() {
    use std::os::unix::fs::symlink;

    let paths = temp_paths("linked-metadata");
    let thread_dir = paths.threads_dir.join("linked-metadata");
    fs::create_dir_all(&thread_dir).unwrap();
    let metadata_target = paths.data_dir.join("metadata-target.json");
    fs::write(
        &metadata_target,
        r#"{"id":"linked-metadata","title":"Test","createdAtMs":1,"updatedAtMs":1,"status":"idle"}"#,
    )
    .unwrap();
    symlink(&metadata_target, thread_dir.join("thread.json")).unwrap();
    write_valid_transcript(&thread_dir);

    migrate_retrieval_readiness(&paths).unwrap();

    assert!(fs::symlink_metadata(thread_dir.join("thread.json"))
        .unwrap()
        .file_type()
        .is_symlink());
    assert!(!fs::read_to_string(metadata_target)
        .unwrap()
        .contains("retrievalReadiness"));
    let _ = fs::remove_dir_all(&paths.data_dir);
}

#[cfg(unix)]
#[test]
fn held_directory_descriptor_cannot_be_redirected_by_replacing_its_path() {
    use std::os::unix::fs::symlink;

    let paths = temp_paths("directory-swap");
    let thread_dir = seed_legacy_thread(&paths.threads_dir, "thread-1", "idle");
    write_valid_transcript(&thread_dir);
    let held = ThreadDirectory::required(&thread_dir).unwrap();
    let moved_dir = paths.data_dir.join("held-thread");
    fs::rename(&thread_dir, &moved_dir).unwrap();
    let external_dir = paths.data_dir.join("external-thread");
    let external_metadata = seed_legacy_thread(&external_dir, "target", "idle");
    symlink(&external_metadata, &thread_dir).unwrap();

    held.update_metadata(true, |metadata| {
        metadata.retrieval_readiness = RetrievalReadiness::Ready;
    })
    .unwrap();

    assert_eq!(readiness(&moved_dir), RetrievalReadiness::Ready);
    assert_eq!(readiness(&external_metadata), RetrievalReadiness::Unknown);
    let _ = fs::remove_dir_all(&paths.data_dir);
}

#[cfg(unix)]
#[test]
fn held_directory_refuses_metadata_swapped_to_a_symlink() {
    use std::os::unix::fs::symlink;

    let paths = temp_paths("metadata-swap");
    let thread_dir = seed_legacy_thread(&paths.threads_dir, "thread-1", "idle");
    let held = ThreadDirectory::required(&thread_dir).unwrap();
    let external_metadata = paths.data_dir.join("external-metadata.json");
    fs::write(&external_metadata, "external").unwrap();
    fs::remove_file(thread_dir.join("thread.json")).unwrap();
    symlink(&external_metadata, thread_dir.join("thread.json")).unwrap();

    let result = held.update_metadata(true, |_| {});

    assert!(result.is_err());
    assert_eq!(fs::read_to_string(external_metadata).unwrap(), "external");
    let _ = fs::remove_dir_all(&paths.data_dir);
}

fn readiness(thread_dir: &Path) -> RetrievalReadiness {
    metadata(thread_dir).retrieval_readiness
}

fn metadata(thread_dir: &Path) -> crate::threads::ThreadMetadata {
    read_thread_metadata(&metadata_path(thread_dir)).unwrap()
}
