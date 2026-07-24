use std::{fs, path::PathBuf};

// The archive store's directory name under the data dir. Single source of truth
// so the live and settings-overlay paths can't drift apart.
pub(crate) const ARCHIVED_DIR_NAME: &str = "archived";

#[derive(Clone)]
pub(crate) struct AppPaths {
    pub(crate) data_dir: PathBuf,
    pub(crate) threads_dir: PathBuf,
    // Archived threads live outside threads_dir so anything pointed at the live
    // corpus (including AI agents crawling it) never sees archived content.
    pub(crate) archived_dir: PathBuf,
}

impl AppPaths {
    /// Builds the application's filesystem layout below the platform-provided
    /// app data directory. Production startup should obtain this directory
    /// from Tauri's path resolver so a sandboxed build uses its container.
    pub(crate) fn from_data_dir(data_dir: PathBuf) -> Self {
        Self {
            threads_dir: data_dir.join("threads"),
            archived_dir: data_dir.join(ARCHIVED_DIR_NAME),
            data_dir,
        }
    }

    pub(crate) fn ensure(&self) -> Result<(), String> {
        fs::create_dir_all(&self.threads_dir).map_err(|err| {
            format!(
                "Failed to create thread storage at {}: {err}",
                self.threads_dir.display()
            )
        })?;
        fs::create_dir_all(self.data_dir.join("engine")).map_err(|err| {
            format!(
                "Failed to create engine storage at {}: {err}",
                self.data_dir.join("engine").display()
            )
        })
    }

    pub(crate) fn thread_dir(&self, thread_id: &str) -> PathBuf {
        self.threads_dir.join(thread_id)
    }

    pub(crate) fn archived_thread_dir(&self, thread_id: &str) -> PathBuf {
        self.archived_dir.join(thread_id)
    }

    /// Removes copies left by an import interrupted before this feature was retired.
    pub(crate) fn cleanup_abandoned_import_staging(&self) -> Result<(), String> {
        let staging = self.data_dir.join(".legacy-import-staging");
        match fs::remove_dir_all(&staging) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!(
                "Failed to clear abandoned import staging at {}: {error}",
                staging.display()
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AppPaths;
    use std::path::PathBuf;

    #[test]
    fn platform_data_directory_is_the_root_for_all_internal_storage() {
        let root = PathBuf::from("/container/Library/Application Support/dev.just-notes");
        let paths = AppPaths::from_data_dir(root.clone());

        assert_eq!(paths.data_dir, root);
        assert_eq!(paths.threads_dir, paths.data_dir.join("threads"));
        assert_eq!(paths.archived_dir, paths.data_dir.join("archived"));
        assert_eq!(
            paths.thread_dir("thread-1"),
            paths.threads_dir.join("thread-1")
        );
        assert_eq!(
            paths.archived_thread_dir("thread-1"),
            paths.archived_dir.join("thread-1")
        );
    }
}
