use std::path::{Path, PathBuf};

#[derive(Clone)]
pub(crate) struct RecordingAudioPaths {
    mic_path: PathBuf,
    system_path: PathBuf,
}

impl RecordingAudioPaths {
    pub(crate) fn for_thread_dir(thread_dir: &Path) -> Self {
        Self {
            mic_path: thread_dir.join("mic.wav"),
            system_path: thread_dir.join("system.wav"),
        }
    }

    pub(crate) fn mic_path(&self) -> &Path {
        &self.mic_path
    }

    pub(crate) fn system_path(&self) -> &Path {
        &self.system_path
    }

    pub(crate) fn has_any(&self) -> bool {
        self.mic_path.is_file() || self.system_path.is_file()
    }

    pub(crate) fn remove_files(&self) -> Result<(), String> {
        for path in [self.mic_path(), self.system_path()] {
            match std::fs::remove_file(path) {
                Ok(()) => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => return Err(format!("Failed to remove {}: {err}", path.display())),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use super::RecordingAudioPaths;

    #[test]
    fn remove_files_deletes_recording_wavs() {
        let dir = env::temp_dir().join(format!(
            "just-notes-raw-audio-cleanup-{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        let files = RecordingAudioPaths::for_thread_dir(&dir);
        fs::write(files.mic_path(), b"mic").unwrap();
        fs::write(files.system_path(), b"system").unwrap();

        files.remove_files().unwrap();

        assert!(!files.mic_path().exists());
        assert!(!files.system_path().exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn remove_files_tolerates_missing_recording_wavs() {
        let dir = env::temp_dir().join(format!(
            "just-notes-raw-audio-missing-{}",
            std::process::id()
        ));
        let files = RecordingAudioPaths::for_thread_dir(&dir);

        files.remove_files().unwrap();
    }
}
