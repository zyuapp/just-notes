use super::{load_settings, save_settings, AppSettings, SettingsState};
use std::{env, fs};

fn settings_test_dir(tag: &str) -> std::path::PathBuf {
    env::temp_dir().join(format!("just-notes-settings-{tag}-{}", std::process::id()))
}

#[test]
fn settings_defaults_are_safe() {
    let dir = settings_test_dir("defaults");
    fs::create_dir_all(&dir).unwrap();
    let defaults = load_settings(&dir);
    assert!(defaults.save_raw_audio);
    assert!(defaults.markdown_copy);
    assert!(!defaults.meeting_reminders_enabled);
    assert!(defaults.meeting_calendar_ids.is_empty());
    assert_eq!(defaults.meeting_reminder_minutes, 5);
    assert!(defaults.meeting_end_reminders);
    assert!(!defaults.meeting_auto_record_enabled);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn settings_round_trip() {
    let dir = settings_test_dir("roundtrip");
    fs::create_dir_all(&dir).unwrap();
    let custom = AppSettings {
        save_raw_audio: false,
        markdown_copy: true,
        meeting_reminders_enabled: true,
        meeting_calendar_ids: vec!["calendar-1".to_string()],
        meeting_reminder_minutes: 10,
        meeting_end_reminders: false,
        meeting_auto_record_enabled: true,
    };
    save_settings(&dir, &custom).unwrap();
    let loaded = load_settings(&dir);
    assert!(!loaded.save_raw_audio);
    assert!(loaded.meeting_reminders_enabled);
    assert_eq!(loaded.meeting_calendar_ids, ["calendar-1"]);
    assert_eq!(loaded.meeting_reminder_minutes, 10);
    assert!(!loaded.meeting_end_reminders);
    assert!(loaded.meeting_auto_record_enabled);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn legacy_settings_keep_existing_values_and_receive_automation_defaults() {
    let dir = settings_test_dir("legacy");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        super::settings_path(&dir),
        r#"{
          "saveRawAudio": false,
          "markdownCopy": true,
          "meetingRemindersEnabled": true,
          "meetingCalendarIds": ["calendar-1"],
          "meetingReminderMinutes": 10,
          "meetingEndReminders": false
        }"#,
    )
    .unwrap();
    let loaded = load_settings(&dir);
    assert!(!loaded.save_raw_audio);
    assert!(loaded.meeting_reminders_enabled);
    assert_eq!(loaded.meeting_calendar_ids, ["calendar-1"]);
    assert_eq!(loaded.meeting_reminder_minutes, 10);
    assert!(!loaded.meeting_end_reminders);
    assert!(!loaded.meeting_auto_record_enabled);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn settings_state_can_atomically_replace_settings() {
    let state = SettingsState::new(AppSettings::default());
    let replacement = AppSettings {
        save_raw_audio: false,
        ..AppSettings::default()
    };
    state.replace(replacement);
    assert!(!state.snapshot().save_raw_audio);
}

#[test]
fn missing_settings_file_uses_defaults() {
    let dir = settings_test_dir("no-file");
    fs::create_dir_all(&dir).unwrap();
    let loaded = load_settings(&dir);
    assert!(loaded.save_raw_audio);
    assert!(loaded.markdown_copy);
    assert!(!loaded.meeting_reminders_enabled);
    assert!(loaded.meeting_calendar_ids.is_empty());
    let _ = fs::remove_dir_all(&dir);
}
