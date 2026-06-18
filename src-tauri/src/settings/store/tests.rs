use super::{
    effective_paths, load_settings, save_settings, AppSettings, TranscriptionProviderPreference,
};
use crate::app::AppPaths;
use std::{env, fs};

#[test]
fn settings_round_trip_and_defaults() {
    let dir = env::temp_dir().join(format!("just-notes-settings-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();

    let defaults = load_settings(&dir, TranscriptionProviderPreference::default);
    assert!(defaults.save_raw_audio);
    assert!(defaults.markdown_copy);
    assert_eq!(defaults.transcripts_dir, None);

    let custom = AppSettings {
        transcripts_dir: Some("/tmp/notes".to_string()),
        save_raw_audio: false,
        markdown_copy: true,
        transcription_provider: TranscriptionProviderPreference::Whisper,
    };
    save_settings(&dir, &custom).unwrap();
    let loaded = load_settings(&dir, TranscriptionProviderPreference::default);
    assert_eq!(loaded.transcripts_dir.as_deref(), Some("/tmp/notes"));
    assert!(!loaded.save_raw_audio);
    assert_eq!(
        loaded.transcription_provider,
        TranscriptionProviderPreference::Whisper
    );

    let base = AppPaths {
        threads_dir: dir.join("threads"),
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
fn missing_provider_uses_supplied_default() {
    let dir = env::temp_dir().join(format!(
        "just-notes-settings-whisper-upgrade-{}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("settings.json"),
        r#"{"saveRawAudio":true,"markdownCopy":true,"transcriptsDir":null}"#,
    )
    .unwrap();

    let loaded = load_settings(&dir, || TranscriptionProviderPreference::Whisper);

    assert_eq!(
        loaded.transcription_provider,
        TranscriptionProviderPreference::Whisper
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn missing_settings_file_uses_supplied_default() {
    let dir = env::temp_dir().join(format!(
        "just-notes-settings-no-file-whisper-upgrade-{}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();

    let loaded = load_settings(&dir, || TranscriptionProviderPreference::Whisper);

    assert_eq!(
        loaded.transcription_provider,
        TranscriptionProviderPreference::Whisper
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn missing_provider_keeps_new_default_without_legacy_whisper() {
    let dir = env::temp_dir().join(format!(
        "just-notes-settings-parakeet-default-{}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("settings.json"),
        r#"{"saveRawAudio":true,"markdownCopy":true,"transcriptsDir":null}"#,
    )
    .unwrap();

    let loaded = load_settings(&dir, TranscriptionProviderPreference::default);

    assert_eq!(
        loaded.transcription_provider,
        TranscriptionProviderPreference::Parakeet
    );

    let _ = fs::remove_dir_all(&dir);
}
