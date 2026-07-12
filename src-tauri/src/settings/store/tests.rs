use super::{
    effective_paths, load_persisted_settings, load_settings, save_persisted_settings,
    save_settings, AppPreferencesUpdate, AppSettings, PersistedSettings, SettingsState,
};
use crate::app::AppPaths;
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
    assert_eq!(defaults.transcripts_dir, None);
    assert!(!defaults.meeting_reminders_enabled);
    assert!(defaults.meeting_calendar_ids.is_empty());
    assert_eq!(defaults.meeting_reminder_minutes, 5);
    assert!(defaults.meeting_end_reminders);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn settings_round_trip_and_effective_paths() {
    let dir = settings_test_dir("roundtrip");
    fs::create_dir_all(&dir).unwrap();

    let custom = AppSettings {
        transcripts_dir: Some("/tmp/notes".to_string()),
        transcripts_folder_unavailable: false,
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
fn bookmark_is_private_but_round_trips_with_flat_settings_json() {
    let dir = env::temp_dir().join(format!(
        "just-notes-settings-bookmark-{}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();

    let mut persisted = PersistedSettings::new(AppSettings::default());
    persisted.set_transcripts_folder("/tmp/authorized-notes".to_string(), "AQIDBA==".to_string());
    save_persisted_settings(&dir, &persisted).unwrap();

    let public = load_settings(&dir);
    assert_eq!(
        public.transcripts_dir.as_deref(),
        Some("/tmp/authorized-notes")
    );
    let loaded = load_persisted_settings(&dir);
    assert_eq!(loaded.transcripts_bookmark(), Some("AQIDBA=="));

    let json = fs::read_to_string(dir.join("settings.json")).unwrap();
    assert!(json.contains("\"transcriptsDir\": \"/tmp/authorized-notes\""));
    assert!(json.contains("\"transcriptsBookmark\""));
    assert!(!json.contains("settings\": {"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn generic_preferences_update_preserves_authorized_storage() {
    let mut persisted = PersistedSettings::new(AppSettings::default());
    persisted.set_transcripts_folder("/tmp/authorized-notes".to_string(), "CQgH".to_string());

    let incoming = AppPreferencesUpdate {
        save_raw_audio: false,
        markdown_copy: true,
        meeting_reminders_enabled: false,
        meeting_calendar_ids: Vec::new(),
        meeting_reminder_minutes: 5,
        meeting_end_reminders: true,
    };
    persisted.replace_preferences(incoming);

    assert_eq!(
        persisted.settings().transcripts_dir.as_deref(),
        Some("/tmp/authorized-notes")
    );
    assert!(!persisted.settings().save_raw_audio);
    assert_eq!(persisted.transcripts_bookmark(), Some("CQgH"));
}

#[test]
fn choosing_default_storage_clears_path_and_bookmark() {
    let mut persisted = PersistedSettings::new(AppSettings::default());
    persisted.set_transcripts_folder("/tmp/authorized-notes".to_string(), "BQQD".to_string());

    persisted.use_default_transcripts_folder();

    assert_eq!(persisted.settings().transcripts_dir, None);
    assert_eq!(persisted.transcripts_bookmark(), None);
}

#[test]
fn settings_state_can_atomically_replace_persisted_settings() {
    let state = SettingsState::new(AppSettings::default());
    let mut persisted = state.persisted_snapshot();
    persisted.set_transcripts_folder("/tmp/authorized-notes".to_string(), "AgQG".to_string());

    state.replace_persisted(persisted);

    assert_eq!(
        state.snapshot().transcripts_dir.as_deref(),
        Some("/tmp/authorized-notes")
    );
    assert_eq!(
        state.persisted_snapshot().transcripts_bookmark(),
        Some("AgQG")
    );
}

#[test]
fn unavailable_bookmark_falls_back_without_discarding_authorization() {
    let mut persisted = PersistedSettings::new(AppSettings {
        save_raw_audio: false,
        ..AppSettings::default()
    });
    persisted.set_transcripts_folder("/tmp/offline-notes".to_string(), "AgQG".to_string());
    let state = SettingsState::from_unavailable_folder(persisted);

    assert_eq!(state.snapshot().transcripts_dir, None);
    assert!(state.snapshot().transcripts_folder_unavailable);
    assert!(!state.snapshot().save_raw_audio);
    assert_eq!(
        state
            .persisted_snapshot()
            .settings()
            .transcripts_dir
            .as_deref(),
        Some("/tmp/offline-notes")
    );
    assert_eq!(
        state.persisted_snapshot().transcripts_bookmark(),
        Some("AgQG")
    );
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
