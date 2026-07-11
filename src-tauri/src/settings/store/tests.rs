use super::{effective_paths, load_settings, save_settings, AppSettings};
use crate::app::AppPaths;
use std::{env, fs};

#[test]
fn settings_round_trip_and_defaults() {
    let dir = env::temp_dir().join(format!("just-notes-settings-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();

    let defaults = load_settings(&dir);
    assert!(defaults.save_raw_audio);
    assert!(defaults.markdown_copy);
    assert_eq!(defaults.transcripts_dir, None);
    assert!(!defaults.meeting_reminders_enabled);
    assert!(defaults.meeting_calendar_ids.is_empty());
    assert_eq!(defaults.meeting_reminder_minutes, 5);
    assert!(defaults.meeting_end_reminders);

    let custom = AppSettings {
        transcripts_dir: Some("/tmp/notes".to_string()),
        save_raw_audio: false,
        markdown_copy: true,
        meeting_reminders_enabled: true,
        meeting_calendar_ids: vec!["calendar-1".to_string()],
        meeting_reminder_minutes: 10,
        meeting_end_reminders: false,
    };
    save_settings(&dir, &custom).unwrap();
    let loaded = load_settings(&dir);
    assert_eq!(loaded.transcripts_dir.as_deref(), Some("/tmp/notes"));
    assert!(!loaded.save_raw_audio);
    assert!(loaded.meeting_reminders_enabled);
    assert_eq!(loaded.meeting_calendar_ids, ["calendar-1"]);
    assert_eq!(loaded.meeting_reminder_minutes, 10);
    assert!(!loaded.meeting_end_reminders);

    let base = AppPaths {
        threads_dir: dir.join("threads"),
        archived_dir: dir.join("archived"),
        data_dir: dir.clone(),
    };
    assert_eq!(
        effective_paths(&base, &loaded).threads_dir,
        std::path::PathBuf::from("/tmp/notes")
    );
    assert_eq!(
        effective_paths(&base, &AppSettings::default()).threads_dir,
        dir.join("threads")
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn legacy_transcription_provider_key_is_ignored() {
    let dir = env::temp_dir().join(format!(
        "just-notes-settings-legacy-provider-{}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("settings.json"),
        r#"{"saveRawAudio":false,"markdownCopy":true,"transcriptsDir":"/tmp/notes","transcriptionProvider":"whisper"}"#,
    )
    .unwrap();

    let loaded = load_settings(&dir);

    assert_eq!(loaded.transcripts_dir.as_deref(), Some("/tmp/notes"));
    assert!(!loaded.save_raw_audio);
    assert!(loaded.markdown_copy);
    assert_eq!(loaded.meeting_reminder_minutes, 5);
    assert!(loaded.meeting_end_reminders);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn missing_settings_file_uses_defaults() {
    let dir = env::temp_dir().join(format!(
        "just-notes-settings-no-file-{}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();

    let loaded = load_settings(&dir);

    assert_eq!(loaded.transcripts_dir, None);
    assert!(loaded.save_raw_audio);
    assert!(loaded.markdown_copy);
    assert!(!loaded.meeting_reminders_enabled);
    assert!(loaded.meeting_calendar_ids.is_empty());

    let _ = fs::remove_dir_all(&dir);
}
