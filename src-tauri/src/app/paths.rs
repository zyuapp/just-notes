use std::{env, fs, path::PathBuf};

#[derive(Clone)]
pub(crate) struct AppPaths {
    pub(crate) data_dir: PathBuf,
    pub(crate) threads_dir: PathBuf,
    // Archived threads live outside threads_dir so anything pointed at the live
    // corpus (including AI agents crawling it) never sees archived content.
    pub(crate) archived_dir: PathBuf,
}

impl AppPaths {
    pub(crate) fn discover() -> Result<Self, String> {
        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| "HOME is not set; cannot locate ~/.just-notes".to_string())?;
        let data_dir = home.join(".just-notes");
        Ok(Self {
            threads_dir: data_dir.join("threads"),
            archived_dir: data_dir.join("archived"),
            data_dir,
        })
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
}
