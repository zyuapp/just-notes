use super::{plan_legacy_import, AppPaths};
use std::{env, fs, path::Path};

fn roots(tag: &str) -> (std::path::PathBuf, std::path::PathBuf, AppPaths) {
    let root = env::temp_dir().join(format!("just-notes-migration-{tag}-{}", std::process::id()));
    let source = root.join(".just-notes");
    let destination = AppPaths::from_data_dir(root.join("container"));
    fs::create_dir_all(&source).unwrap();
    destination.ensure().unwrap();
    (root, source, destination)
}

fn recording(root: &Path, id: &str, title: &str) {
    recording_with_status(root, id, title, "idle");
}

fn recording_with_status(root: &Path, id: &str, title: &str, status: &str) {
    let folder = root.join(id);
    fs::create_dir_all(&folder).unwrap();
    fs::write(
        folder.join("thread.json"),
        format!(
            r#"{{"id":"{id}","title":"{title}","createdAtMs":1,"updatedAtMs":1,"status":"{status}"}}"#
        ),
    )
    .unwrap();
    fs::write(folder.join("transcript.jsonl"), format!("{title}\n")).unwrap();
}

fn imported_status(path: &Path) -> String {
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(path.join("thread.json")).unwrap()).unwrap();
    value["status"].as_str().unwrap().to_string()
}

fn imported_id(path: &Path) -> String {
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(path.join("thread.json")).unwrap()).unwrap();
    value["id"].as_str().unwrap().to_string()
}

#[test]
fn merges_recordings_without_replacing_current_data_or_models() {
    let (root, source, destination) = roots("merge");
    recording(&source.join("threads"), "old-active", "Legacy active");
    recording(&source.join("archived"), "old-archived", "Legacy archived");
    recording(&destination.threads_dir, "current", "Current");
    fs::create_dir_all(destination.data_dir.join("models/current-model")).unwrap();

    let plan = plan_legacy_import(&source, None, &destination).unwrap();
    let preview = plan.preview();
    let result = plan.execute().unwrap();

    assert_eq!(preview.active_recordings, 1);
    assert_eq!(preview.archived_recordings, 1);
    assert_eq!(result.imported, 2);
    assert!(destination.thread_dir("current").exists());
    assert!(destination.thread_dir("old-active").exists());
    assert!(destination.archived_thread_dir("old-archived").exists());
    assert!(destination.data_dir.join("models/current-model").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn skips_exact_duplicates() {
    let (root, source, destination) = roots("duplicate");
    recording(&source.join("threads"), "same", "Same");
    recording(&destination.threads_dir, "same", "Same");
    let plan = plan_legacy_import(&source, None, &destination).unwrap();
    assert_eq!(plan.preview().duplicates, 1);
    assert_eq!(plan.execute().unwrap().imported, 0);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_boundaries_prevent_ambiguous_duplicate_hashes() {
    let (root, source, destination) = roots("digest-framing");
    recording(&source.join("threads"), "same", "Same");
    recording(&destination.threads_dir, "same", "Same");
    fs::write(source.join("threads/same/a"), "bc").unwrap();
    fs::write(destination.thread_dir("same").join("ab"), "c").unwrap();

    let plan = plan_legacy_import(&source, None, &destination).unwrap();
    assert_eq!(plan.preview().duplicates, 0);
    assert_eq!(plan.preview().conflicts, 1);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn clears_abandoned_staging_outside_thread_roots() {
    let (root, _, destination) = roots("staging-cleanup");
    let staging = destination.legacy_import_staging_dir();
    fs::create_dir_all(staging.join("orphan")).unwrap();
    fs::write(staging.join("orphan/thread.json"), "incomplete").unwrap();

    super::cleanup_stale_import_staging(&destination).unwrap();

    assert!(!staging.exists());
    assert!(!destination
        .threads_dir
        .join(".legacy-import-orphan")
        .exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn preserves_different_recordings_with_the_same_id() {
    let (root, source, destination) = roots("conflict");
    recording(&source.join("threads"), "collision", "Legacy");
    recording(&destination.threads_dir, "collision", "Current");
    let plan = plan_legacy_import(&source, None, &destination).unwrap();
    assert_eq!(plan.preview().conflicts, 1);
    plan.execute().unwrap();
    let imported = destination.thread_dir("collision-imported");
    assert_eq!(imported_id(&imported), "collision-imported");
    assert_eq!(
        imported_id(&destination.thread_dir("collision")),
        "collision"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn rerunning_a_conflicting_import_skips_the_preserved_copy() {
    let (root, source, destination) = roots("conflict-rerun");
    recording(&source.join("threads"), "collision", "Legacy");
    recording(&destination.threads_dir, "collision", "Current");

    plan_legacy_import(&source, None, &destination)
        .unwrap()
        .execute()
        .unwrap();
    let second = plan_legacy_import(&source, None, &destination).unwrap();
    assert_eq!(second.preview().duplicates, 1);
    assert_eq!(second.execute().unwrap().imported, 0);
    assert!(!destination.thread_dir("collision-imported-2").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn normalizes_interrupted_recording_statuses_during_import() {
    let (root, source, destination) = roots("busy-status");
    recording_with_status(
        &source.join("threads"),
        "recording",
        "Recording",
        "recording",
    );
    recording_with_status(
        &source.join("threads"),
        "transcribing",
        "Transcribing",
        "transcribing",
    );

    plan_legacy_import(&source, None, &destination)
        .unwrap()
        .execute()
        .unwrap();

    assert_eq!(
        imported_status(&destination.thread_dir("recording")),
        "idle"
    );
    assert_eq!(
        imported_status(&destination.thread_dir("transcribing")),
        "idle"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn custom_legacy_transcripts_are_merged_into_the_active_store() {
    let (root, source, destination) = roots("custom");
    fs::create_dir_all(source.join("archived")).unwrap();
    let custom = root.join("legacy-custom");
    recording(&custom, "custom-thread", "Custom");
    plan_legacy_import(&source, Some(&custom), &destination)
        .unwrap()
        .execute()
        .unwrap();
    assert!(destination.thread_dir("custom-thread").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn rolls_back_recordings_installed_before_a_late_collision() {
    let (root, source, destination) = roots("rollback");
    recording(&source.join("threads"), "a-first", "First");
    recording(&source.join("threads"), "z-second", "Second");
    let plan = plan_legacy_import(&source, None, &destination).unwrap();
    recording(
        &destination.threads_dir,
        "z-second",
        "Created after preview",
    );
    let error = plan.execute().err().unwrap();
    assert!(error.contains("changed after the import preview"));
    assert!(!destination.thread_dir("a-first").exists());
    assert_eq!(imported_id(&destination.thread_dir("z-second")), "z-second");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn requires_the_legacy_folder_itself() {
    let (root, source, destination) = roots("wrong-folder");
    recording(&source.join("threads"), "old", "Old");
    let error = plan_legacy_import(&source.join("threads"), None, &destination)
        .err()
        .unwrap();
    assert!(error.contains(".just-notes folder"));
    let _ = fs::remove_dir_all(root);
}
