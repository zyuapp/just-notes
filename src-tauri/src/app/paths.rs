use std::{env, fs, path::PathBuf};

use crate::transcription::models::{discover_whisper_models, TranscriptionPaths};

#[derive(Clone)]
pub(crate) struct AppPaths {
    pub(crate) data_dir: PathBuf,
    pub(crate) threads_dir: PathBuf,
}

impl AppPaths {
    pub(crate) fn discover() -> Result<Self, String> {
        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| "HOME is not set; cannot locate ~/.just-notes".to_string())?;
        let data_dir = home.join(".just-notes");
        Ok(Self {
            threads_dir: data_dir.join("threads"),
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
        })?;
        fs::create_dir_all(self.data_dir.join("models").join("whisper")).map_err(|err| {
            format!(
                "Failed to create model storage at {}: {err}",
                self.data_dir.join("models").join("whisper").display()
            )
        })
    }

    pub(crate) fn thread_dir(&self, thread_id: &str) -> PathBuf {
        self.threads_dir.join(thread_id)
    }

    pub(crate) fn transcription_paths(&self) -> TranscriptionPaths {
        let mut available_models =
            discover_whisper_models(&self.data_dir.join("models").join("whisper"));
        let selected_model = available_models
            .iter()
            .find(|model| model.installed)
            .cloned()
            .unwrap_or_else(|| {
                available_models
                    .last()
                    .expect("whisper model candidates")
                    .clone()
            });
        for model in &mut available_models {
            model.selected = model.filename == selected_model.filename;
        }

        TranscriptionPaths {
            model_path: selected_model.path.clone(),
            model_name: selected_model.name.clone(),
            available_models,
        }
    }
}
