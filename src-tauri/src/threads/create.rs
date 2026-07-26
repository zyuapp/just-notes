use std::{fs, io::ErrorKind, path::PathBuf};

use crate::app::{now_ms, AppPaths};

use super::{
    repository::{load_thread_detail, save_thread_metadata},
    title::{normalized_external_title, validated_title},
    transcript_store::write_text_atomic,
    CalendarProvenance, ThreadDetail, ThreadMetadata, ThreadStatus,
};

type ThreadDirectory = (String, PathBuf);

pub(crate) fn create_thread(paths: &AppPaths) -> Result<ThreadDetail, String> {
    create_thread_with_title(paths, "Untitled thread", None)
}

pub(crate) fn create_thread_with_title(
    paths: &AppPaths,
    title: &str,
    calendar: Option<CalendarProvenance>,
) -> Result<ThreadDetail, String> {
    let title = validated_title(title)?;
    paths.ensure()?;
    let now = now_ms()?;
    let (id, thread_dir) = create_unique_thread_dir(paths, now)?;
    let metadata = ThreadMetadata {
        id,
        title: title.to_string(),
        created_at_ms: now,
        updated_at_ms: now,
        status: ThreadStatus::Idle,
        duration_ms: 0,
        calendar,
    };
    save_thread_metadata(&thread_dir, &metadata)?;
    write_text_atomic(&thread_dir.join("transcript.md"), &format!("# {title}\n\n"))?;
    load_thread_detail(&thread_dir)
}

pub(crate) fn create_thread_with_external_title(
    paths: &AppPaths,
    title: &str,
    calendar: Option<CalendarProvenance>,
) -> Result<ThreadDetail, String> {
    create_thread_with_title(paths, &normalized_external_title(title), calendar)
}

pub(crate) fn discard_failed_thread(thread_dir: &std::path::Path) {
    let _ = fs::remove_dir_all(thread_dir);
}

fn create_unique_thread_dir(paths: &AppPaths, now: u64) -> Result<ThreadDirectory, String> {
    for attempt in 0..1_000 {
        let id = if attempt == 0 {
            format!("thread-{now}")
        } else {
            format!("thread-{now}-{attempt}")
        };
        let thread_dir = paths.thread_dir(&id);
        match fs::create_dir(&thread_dir) {
            Ok(()) => {
                fs::create_dir(thread_dir.join("work")).map_err(|err| {
                    format!(
                        "Failed to create thread work folder at {}: {err}",
                        thread_dir.display()
                    )
                })?;
                return Ok((id, thread_dir));
            }
            Err(err) if err.kind() == ErrorKind::AlreadyExists => continue,
            Err(err) => {
                return Err(format!(
                    "Failed to create thread folder at {}: {err}",
                    thread_dir.display()
                ));
            }
        }
    }
    Err("Could not allocate a unique thread folder".to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        create_thread, create_thread_with_external_title, create_unique_thread_dir,
        CalendarProvenance,
    };
    use crate::{
        app::AppPaths,
        threads::repository::{metadata_path, read_thread_metadata},
    };
    use std::{env, fs, time::UNIX_EPOCH};

    fn temp_paths(name: &str) -> AppPaths {
        let stamp = UNIX_EPOCH.elapsed().unwrap().as_nanos();
        let data_dir = env::temp_dir().join(format!("just-notes-{name}-{stamp}"));
        AppPaths {
            threads_dir: data_dir.join("threads"),
            archived_dir: data_dir.join("archived"),
            data_dir,
        }
    }

    fn provenance() -> CalendarProvenance {
        CalendarProvenance {
            event_id: "event-1:1000".to_string(),
            calendar_id: "calendar-1".to_string(),
            attendees: vec!["Alice".to_string(), "Bob".to_string()],
            start_at_ms: 1_000,
            end_at_ms: 2_000,
        }
    }

    #[test]
    fn thread_directory_creation_retries_timestamp_collisions() {
        let paths = temp_paths("create-collision");
        paths.ensure().unwrap();
        let (first_id, _) = create_unique_thread_dir(&paths, 42).unwrap();
        let (second_id, _) = create_unique_thread_dir(&paths, 42).unwrap();
        assert_eq!(first_id, "thread-42");
        assert_eq!(second_id, "thread-42-1");
        let _ = fs::remove_dir_all(&paths.data_dir);
    }

    #[test]
    fn a_meeting_thread_is_created_with_its_calendar_provenance() {
        let paths = temp_paths("create-provenance");
        let thread =
            create_thread_with_external_title(&paths, "Pricing sync", Some(provenance())).unwrap();

        let stored = read_thread_metadata(&metadata_path(&paths.thread_dir(&thread.summary.id)))
            .unwrap()
            .calendar
            .expect("a meeting thread keeps its calendar provenance");
        assert_eq!(stored, provenance());
        let _ = fs::remove_dir_all(&paths.data_dir);
    }

    #[test]
    fn manually_created_threads_carry_no_calendar_provenance() {
        let paths = temp_paths("create-manual");
        let thread = create_thread(&paths).unwrap();

        let stored =
            read_thread_metadata(&metadata_path(&paths.thread_dir(&thread.summary.id))).unwrap();
        assert!(stored.calendar.is_none());
        let _ = fs::remove_dir_all(&paths.data_dir);
    }

    #[test]
    fn a_blank_meeting_title_falls_back_to_the_default_thread_title() {
        let paths = temp_paths("create-blank-title");
        let thread = create_thread_with_external_title(&paths, "", None).unwrap();

        assert_eq!(thread.summary.title, "Untitled thread");
        let _ = fs::remove_dir_all(&paths.data_dir);
    }
}
