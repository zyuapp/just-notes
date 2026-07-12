use std::{fs, io::ErrorKind, path::PathBuf};

use crate::app::{now_ms, AppPaths};

use super::{
    repository::{load_thread_detail, save_thread_metadata},
    title::{normalized_external_title, validated_title},
    transcript_store::write_text_atomic,
    ThreadDetail, ThreadMetadata, ThreadStatus,
};

pub(crate) fn create_thread(paths: &AppPaths) -> Result<ThreadDetail, String> {
    create_thread_with_title(paths, "Untitled thread")
}

pub(crate) fn create_thread_with_title(
    paths: &AppPaths,
    title: &str,
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
    };
    save_thread_metadata(&thread_dir, &metadata)?;
    write_text_atomic(&thread_dir.join("transcript.md"), &format!("# {title}\n\n"))?;
    load_thread_detail(&thread_dir)
}

pub(crate) fn create_thread_with_external_title(
    paths: &AppPaths,
    title: &str,
) -> Result<ThreadDetail, String> {
    create_thread_with_title(paths, &normalized_external_title(title))
}

pub(crate) fn discard_failed_thread(thread_dir: &std::path::Path) {
    let _ = fs::remove_dir_all(thread_dir);
}

fn create_unique_thread_dir(paths: &AppPaths, now: u64) -> Result<(String, PathBuf), String> {
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
    use super::create_unique_thread_dir;
    use crate::app::AppPaths;
    use std::{env, fs};

    #[test]
    fn thread_directory_creation_retries_timestamp_collisions() {
        let root = env::temp_dir().join(format!("just-notes-create-{}", std::process::id()));
        let paths = AppPaths {
            threads_dir: root.join("threads"),
            archived_dir: root.join("archived"),
            data_dir: root.clone(),
        };
        paths.ensure().unwrap();
        let (first_id, _) = create_unique_thread_dir(&paths, 42).unwrap();
        let (second_id, _) = create_unique_thread_dir(&paths, 42).unwrap();
        assert_eq!(first_id, "thread-42");
        assert_eq!(second_id, "thread-42-1");
        let _ = fs::remove_dir_all(root);
    }
}
