#[derive(serde::Serialize, serde::Deserialize, ts_rs::TS, Clone)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub(crate) struct AppSettings {
    pub(crate) save_raw_audio: bool,
    pub(crate) markdown_copy: bool,
    pub(crate) meeting_reminders_enabled: bool,
    pub(crate) meeting_calendar_ids: Vec<String>,
    pub(crate) meeting_reminder_minutes: u16,
    pub(crate) meeting_end_reminders: bool,
    pub(crate) meeting_auto_record_enabled: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            save_raw_audio: true,
            markdown_copy: true,
            meeting_reminders_enabled: false,
            meeting_calendar_ids: Vec::new(),
            meeting_reminder_minutes: 5,
            meeting_end_reminders: true,
            meeting_auto_record_enabled: false,
        }
    }
}
