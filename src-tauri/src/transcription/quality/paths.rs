use std::path::{Path, PathBuf};

pub(super) fn quality_app_data_dir_from_env() -> PathBuf {
    let home = std::env::var_os("HOME").expect("resolve home directory");
    let configured = std::env::var_os("JUST_NOTES_QUALITY_DATA_DIR").map(PathBuf::from);
    quality_app_data_dir(Path::new(&home), configured)
}

fn quality_app_data_dir(home: &Path, configured: Option<PathBuf>) -> PathBuf {
    configured.unwrap_or_else(|| {
        home.join(
            "Library/Containers/dev.just-notes/Data/Library/Application Support/dev.just-notes",
        )
    })
}

#[test]
fn defaults_to_the_sandbox_app_data_directory() {
    let home = Path::new("/Users/tester");
    assert_eq!(
        quality_app_data_dir(home, None),
        home.join(
            "Library/Containers/dev.just-notes/Data/Library/Application Support/dev.just-notes"
        )
    );
    assert_eq!(
        quality_app_data_dir(home, Some(PathBuf::from("/tmp/quality-data"))),
        PathBuf::from("/tmp/quality-data")
    );
}
