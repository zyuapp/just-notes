use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use super::{
    format_io,
    scan::{recording_directories, tree_size, trees_match, validate_recording},
    AppPaths, LegacyImportPlan, LegacyImportPreviewData, PlannedRecording, RecordingStore,
};

const THREADS_DIR: &str = "threads";
const ARCHIVED_DIR: &str = "archived";

struct PlanDraft {
    occupied_ids: HashSet<String>,
    recordings: Vec<PlannedRecording>,
    duplicates: usize,
    conflicts: usize,
    active_recordings: usize,
    archived_recordings: usize,
    bytes_to_copy: u64,
}

impl PlanDraft {
    fn new(destination: &AppPaths) -> Result<Self, String> {
        Ok(Self {
            occupied_ids: existing_ids(destination)?,
            recordings: Vec::new(),
            duplicates: 0,
            conflicts: 0,
            active_recordings: 0,
            archived_recordings: 0,
            bytes_to_copy: 0,
        })
    }

    fn add_store(
        &mut self,
        store: RecordingStore,
        source_root: &Path,
        destination_root: &Path,
        destination: &AppPaths,
    ) -> Result<(), String> {
        for recording in recording_directories(source_root)? {
            let original_id = validate_recording(&recording)?;
            if has_existing_copy(destination, &self.occupied_ids, &original_id, &recording)? {
                self.duplicates += 1;
                continue;
            }
            let replacement_id = self.replacement_id(&original_id);
            let destination_id = replacement_id.as_deref().unwrap_or(&original_id);
            self.occupied_ids.insert(destination_id.to_string());
            self.bytes_to_copy += tree_size(&recording)?;
            match store {
                RecordingStore::Active => self.active_recordings += 1,
                RecordingStore::Archived => self.archived_recordings += 1,
            }
            self.recordings.push(PlannedRecording {
                source: recording,
                destination: destination_root.join(destination_id),
                replacement_id,
            });
        }
        Ok(())
    }

    fn replacement_id(&mut self, original_id: &str) -> Option<String> {
        if !self.occupied_ids.contains(original_id) {
            return None;
        }
        self.conflicts += 1;
        Some(allocate_import_id(original_id, &self.occupied_ids))
    }
}

pub(crate) fn plan_legacy_import(
    source: &Path,
    custom_threads_source: Option<&Path>,
    destination: &AppPaths,
) -> Result<LegacyImportPlan, String> {
    validate_source(source, custom_threads_source)?;
    fs::create_dir_all(&destination.threads_dir)
        .map_err(format_io("prepare transcripts folder"))?;
    fs::create_dir_all(&destination.archived_dir).map_err(format_io("prepare archive folder"))?;

    let active_source = custom_threads_source
        .map(Path::to_path_buf)
        .unwrap_or_else(|| source.join(THREADS_DIR));
    let archived_source = source.join(ARCHIVED_DIR);
    let mut draft = PlanDraft::new(destination)?;
    draft.add_store(
        RecordingStore::Active,
        &active_source,
        &destination.threads_dir,
        destination,
    )?;
    draft.add_store(
        RecordingStore::Archived,
        &archived_source,
        &destination.archived_dir,
        destination,
    )?;
    if draft.recordings.is_empty() && draft.duplicates == 0 {
        return Err("The selected legacy data contains no recordings".to_string());
    }

    Ok(LegacyImportPlan {
        preview: LegacyImportPreviewData {
            active_recordings: draft.active_recordings,
            archived_recordings: draft.archived_recordings,
            duplicates: draft.duplicates,
            conflicts: draft.conflicts,
            bytes_to_copy: draft.bytes_to_copy,
            source_path: source.display().to_string(),
        },
        recordings: draft.recordings,
        active_destination: destination.threads_dir.clone(),
        archive_destination: destination.archived_dir.clone(),
        staging_root: destination.legacy_import_staging_dir(),
    })
}

fn validate_source(source: &Path, custom_threads_source: Option<&Path>) -> Result<(), String> {
    if source.file_name().and_then(|name| name.to_str()) != Some(".just-notes") {
        return Err("Choose the existing .just-notes folder".to_string());
    }
    let default_threads = source.join(THREADS_DIR);
    let threads = custom_threads_source.unwrap_or(&default_threads);
    if !threads.is_dir() && !source.join(ARCHIVED_DIR).is_dir() {
        return Err("The selected folder does not contain Just Notes recordings".to_string());
    }
    Ok(())
}

fn existing_ids(paths: &AppPaths) -> Result<HashSet<String>, String> {
    let mut ids = HashSet::new();
    for root in [&paths.threads_dir, &paths.archived_dir] {
        for recording in recording_directories(root)? {
            if let Some(id) = recording.file_name().and_then(|name| name.to_str()) {
                ids.insert(id.to_string());
            }
        }
    }
    Ok(ids)
}

fn find_existing(paths: &AppPaths, id: &str) -> Option<PathBuf> {
    [paths.thread_dir(id), paths.archived_thread_dir(id)]
        .into_iter()
        .find(|path| path.is_dir())
}

fn has_existing_copy(
    paths: &AppPaths,
    occupied: &HashSet<String>,
    original_id: &str,
    source: &Path,
) -> Result<bool, String> {
    for candidate in occupied {
        if !is_import_variant(original_id, candidate) {
            continue;
        }
        if let Some(existing) = find_existing(paths, candidate) {
            if trees_match(source, &existing)? {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn is_import_variant(original_id: &str, candidate: &str) -> bool {
    if candidate == original_id {
        return true;
    }
    let prefix = format!("{original_id}-imported");
    let Some(suffix) = candidate.strip_prefix(&prefix) else {
        return false;
    };
    suffix.is_empty()
        || suffix.strip_prefix('-').is_some_and(|number| {
            !number.is_empty() && number.chars().all(|ch| ch.is_ascii_digit())
        })
}

fn allocate_import_id(original: &str, occupied: &HashSet<String>) -> String {
    let first = format!("{original}-imported");
    if !occupied.contains(&first) {
        return first;
    }
    for suffix in 2.. {
        let candidate = format!("{original}-imported-{suffix}");
        if !occupied.contains(&candidate) {
            return candidate;
        }
    }
    unreachable!()
}
