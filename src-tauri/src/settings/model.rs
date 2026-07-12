#[derive(serde::Serialize, serde::Deserialize, ts_rs::TS, Clone)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub(crate) struct AppSettings {
    pub(crate) transcripts_dir: Option<String>,
    pub(crate) transcripts_folder_unavailable: bool,
    pub(crate) save_raw_audio: bool,
    pub(crate) markdown_copy: bool,
    pub(crate) meeting_reminders_enabled: bool,
    pub(crate) meeting_calendar_ids: Vec<String>,
    pub(crate) meeting_reminder_minutes: u16,
    pub(crate) meeting_end_reminders: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            transcripts_dir: None,
            transcripts_folder_unavailable: false,
            save_raw_audio: true,
            markdown_copy: true,
            meeting_reminders_enabled: false,
            meeting_calendar_ids: Vec::new(),
            meeting_reminder_minutes: 5,
            meeting_end_reminders: true,
        }
    }
}

#[derive(serde::Deserialize, ts_rs::TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub(crate) struct AppPreferencesUpdate {
    pub(crate) save_raw_audio: bool,
    pub(crate) markdown_copy: bool,
    pub(crate) meeting_reminders_enabled: bool,
    pub(crate) meeting_calendar_ids: Vec<String>,
    pub(crate) meeting_reminder_minutes: u16,
    pub(crate) meeting_end_reminders: bool,
}

/// Complete settings, including authorization data that never crosses IPC.
#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct PersistedSettings {
    #[serde(flatten)]
    pub(super) settings: AppSettings,
    #[serde(skip_serializing_if = "Option::is_none")]
    transcripts_bookmark: Option<String>,
}

impl PersistedSettings {
    #[cfg(test)]
    pub(crate) fn new(settings: AppSettings) -> Self {
        Self {
            settings,
            transcripts_bookmark: None,
        }
    }

    pub(crate) fn settings(&self) -> &AppSettings {
        &self.settings
    }

    #[cfg(test)]
    pub(crate) fn into_settings(self) -> AppSettings {
        self.settings
    }

    pub(crate) fn transcripts_bookmark(&self) -> Option<&str> {
        self.transcripts_bookmark.as_deref()
    }

    pub(crate) fn replace_preferences(&mut self, preferences: AppPreferencesUpdate) {
        self.settings.save_raw_audio = preferences.save_raw_audio;
        self.settings.markdown_copy = preferences.markdown_copy;
        self.settings.meeting_reminders_enabled = preferences.meeting_reminders_enabled;
        self.settings.meeting_calendar_ids = preferences.meeting_calendar_ids;
        self.settings.meeting_reminder_minutes = preferences.meeting_reminder_minutes;
        self.settings.meeting_end_reminders = preferences.meeting_end_reminders;
    }

    pub(crate) fn set_transcripts_folder(&mut self, path: String, bookmark: String) {
        self.settings.transcripts_dir = Some(path);
        self.transcripts_bookmark = Some(bookmark);
    }

    pub(crate) fn use_default_transcripts_folder(&mut self) {
        self.settings.transcripts_dir = None;
        self.transcripts_bookmark = None;
    }

    #[cfg(test)]
    pub(super) fn replace_all_settings(&mut self, settings: AppSettings) {
        if self.settings.transcripts_dir != settings.transcripts_dir {
            self.transcripts_bookmark = None;
        }
        self.settings = settings;
    }
}
