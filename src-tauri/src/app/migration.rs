use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::Value;

/// Discovers the external active-recordings folder used by an older release.
/// Import validation and merge policy belong to the threads domain.
pub(crate) fn legacy_custom_threads_path(source: &Path) -> Result<Option<PathBuf>, String> {
    let settings_path = source.join("settings.json");
    if !settings_path.is_file() {
        return Ok(None);
    }
    let settings: Value = serde_json::from_slice(
        &fs::read(&settings_path)
            .map_err(|error| format!("Failed to read legacy settings: {error}"))?,
    )
    .map_err(|error| format!("Invalid {}: {error}", settings_path.display()))?;
    Ok(settings
        .get("transcriptsDir")
        .and_then(Value::as_str)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from))
}

#[cfg(test)]
mod tests {
    use super::legacy_custom_threads_path;
    use std::{env, fs};

    #[test]
    fn reads_custom_transcripts_path_from_legacy_settings() {
        let source =
            env::temp_dir().join(format!("just-notes-legacy-layout-{}", std::process::id()));
        fs::create_dir_all(&source).unwrap();
        fs::write(
            source.join("settings.json"),
            r#"{"transcriptsDir":"/Volumes/Notes/Recordings"}"#,
        )
        .unwrap();

        assert_eq!(
            legacy_custom_threads_path(&source).unwrap(),
            Some("/Volumes/Notes/Recordings".into())
        );
        let _ = fs::remove_dir_all(source);
    }
}
