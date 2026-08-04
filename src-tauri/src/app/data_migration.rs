use std::{
    error::Error,
    fmt,
    path::{Path, PathBuf},
};

mod filesystem;
mod lock;
use filesystem::{
    inspect_directory, move_inspected_directory_no_replace, prepare_empty_destination,
};
use lock::MigrationLock;

const LEGACY_RELATIVE_DATA_ROOT: &str =
    "Library/Containers/dev.just-notes/Data/Library/Application Support/dev.just-notes";

pub(crate) struct DataRootMigration {
    direct_root: PathBuf,
    legacy_root: PathBuf,
}

#[derive(Debug)]
pub(crate) enum DataRootMigrationError {
    Conflict {
        direct_root: PathBuf,
        legacy_root: PathBuf,
    },
    FileSystem(String),
}

impl DataRootMigrationError {
    pub(crate) fn is_conflict(&self) -> bool {
        matches!(self, Self::Conflict { .. })
    }
}

impl fmt::Display for DataRootMigrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Conflict {
                direct_root,
                legacy_root,
            } => write!(
                formatter,
                "Just Notes found data in both {} and {}. Move one folder aside before launching again; the app will not merge them automatically.",
                direct_root.display(),
                legacy_root.display()
            ),
            Self::FileSystem(message) => formatter.write_str(message),
        }
    }
}

impl Error for DataRootMigrationError {}

impl DataRootMigration {
    pub(crate) fn for_home(direct_root: PathBuf, home_dir: &Path) -> Self {
        Self {
            direct_root,
            legacy_root: home_dir.join(LEGACY_RELATIVE_DATA_ROOT),
        }
    }

    pub(crate) fn resolve(self) -> Result<PathBuf, DataRootMigrationError> {
        if self.direct_root == self.legacy_root {
            return Ok(self.direct_root);
        }

        let _migration_lock = MigrationLock::acquire(&self.direct_root)?;
        let direct = inspect_directory(&self.direct_root)?;
        let legacy = inspect_directory(&self.legacy_root)?;

        match (direct.has_data(), legacy.has_data()) {
            (true, true) => Err(DataRootMigrationError::Conflict {
                direct_root: self.direct_root,
                legacy_root: self.legacy_root,
            }),
            (false, true) => {
                prepare_empty_destination(&self.direct_root, &direct)?;
                move_inspected_directory_no_replace(&self.legacy_root, &legacy, &self.direct_root)?;
                Ok(self.direct_root)
            }
            _ => Ok(self.direct_root),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        filesystem::{
            inspect_directory, move_inspected_directory_no_replace, prepare_empty_destination,
        },
        DataRootMigration,
    };
    use std::{env, fs, path::PathBuf, time::UNIX_EPOCH};

    struct Fixture {
        root: PathBuf,
        home: PathBuf,
        direct: PathBuf,
        legacy: PathBuf,
    }

    impl Fixture {
        fn new(name: &str) -> Self {
            let stamp = UNIX_EPOCH.elapsed().unwrap().as_nanos();
            let root = env::temp_dir().join(format!("just-notes-migration-{name}-{stamp}"));
            let home = root.join("home");
            let direct = home.join("Library/Application Support/dev.just-notes");
            let legacy = home.join(
                "Library/Containers/dev.just-notes/Data/Library/Application Support/dev.just-notes",
            );
            Self {
                root,
                home,
                direct,
                legacy,
            }
        }

        fn migration(&self) -> DataRootMigration {
            DataRootMigration::for_home(self.direct.clone(), &self.home)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn moves_legacy_data_when_the_direct_root_is_unused() {
        let fixture = Fixture::new("move");
        fs::create_dir_all(fixture.legacy.join("threads/thread-1")).unwrap();
        fs::write(fixture.legacy.join("settings.json"), "{}").unwrap();
        let resolved = fixture.migration().resolve().unwrap();
        assert_eq!(resolved, fixture.direct);
        assert!(resolved.join("settings.json").is_file());
        assert!(resolved.join("threads/thread-1").is_dir());
        assert!(!fixture.legacy.exists());
    }

    #[test]
    fn replaces_an_empty_direct_root_before_moving_legacy_data() {
        let fixture = Fixture::new("empty-direct");
        fs::create_dir_all(fixture.direct.join("threads")).unwrap();
        fs::create_dir_all(fixture.direct.join("engine")).unwrap();
        fs::create_dir_all(&fixture.legacy).unwrap();
        fs::write(fixture.legacy.join("settings.json"), "{}").unwrap();
        fixture.migration().resolve().unwrap();
        assert!(fixture.direct.join("settings.json").is_file());
    }

    #[test]
    fn refuses_to_merge_two_populated_roots() {
        let fixture = Fixture::new("conflict");
        fs::create_dir_all(fixture.direct.join("threads")).unwrap();
        fs::create_dir_all(&fixture.legacy).unwrap();
        fs::write(fixture.direct.join("threads/thread.json"), "direct").unwrap();
        fs::write(fixture.legacy.join("settings.json"), "legacy").unwrap();
        let error = fixture.migration().resolve().unwrap_err();
        assert!(error.to_string().contains("will not merge"));
        assert_eq!(
            fs::read_to_string(fixture.direct.join("threads/thread.json")).unwrap(),
            "direct"
        );
        assert_eq!(
            fs::read_to_string(fixture.legacy.join("settings.json")).unwrap(),
            "legacy"
        );
    }

    #[test]
    fn rejects_symlinked_roots() {
        use std::os::unix::fs::symlink;
        let fixture = Fixture::new("symlink");
        fs::create_dir_all(fixture.direct.parent().unwrap()).unwrap();
        fs::create_dir_all(fixture.root.join("elsewhere")).unwrap();
        symlink(fixture.root.join("elsewhere"), &fixture.direct).unwrap();
        let error = fixture.migration().resolve().unwrap_err();
        assert!(error.to_string().contains("regular data directory"));
    }

    #[test]
    fn refuses_to_follow_symlinks_inside_the_direct_root() {
        use std::os::unix::fs::symlink;
        let fixture = Fixture::new("child-symlink");
        let external = fixture.root.join("external");
        fs::create_dir_all(&fixture.direct).unwrap();
        fs::create_dir_all(&external).unwrap();
        fs::create_dir_all(&fixture.legacy).unwrap();
        fs::write(fixture.legacy.join("settings.json"), "legacy").unwrap();
        symlink(&external, fixture.direct.join("engine")).unwrap();
        let error = fixture.migration().resolve().unwrap_err();
        assert!(error.is_conflict());
        assert!(external.is_dir());
        assert!(fixture.direct.join("engine").is_symlink());
    }

    #[test]
    fn refuses_to_remove_a_replacement_destination() {
        let fixture = Fixture::new("destination-replaced");
        let original = fixture.root.join("original-direct");
        fs::create_dir_all(fixture.direct.join("threads")).unwrap();
        let inspection = inspect_directory(&fixture.direct).unwrap();
        fs::rename(&fixture.direct, &original).unwrap();
        fs::create_dir_all(&fixture.direct).unwrap();
        let error = prepare_empty_destination(&fixture.direct, &inspection).unwrap_err();
        assert!(error.to_string().contains("changed"));
        assert!(fixture.direct.is_dir());
        assert!(original.join("threads").is_dir());
    }

    #[test]
    fn refuses_to_move_a_replacement_source() {
        use std::os::unix::fs::symlink;
        let fixture = Fixture::new("source-replaced");
        let original = fixture.root.join("original-legacy");
        fs::create_dir_all(&fixture.legacy).unwrap();
        fs::write(fixture.legacy.join("settings.json"), "legacy").unwrap();
        let inspection = inspect_directory(&fixture.legacy).unwrap();
        fs::rename(&fixture.legacy, &original).unwrap();
        symlink(&original, &fixture.legacy).unwrap();
        fs::create_dir_all(fixture.direct.parent().unwrap()).unwrap();
        let error =
            move_inspected_directory_no_replace(&fixture.legacy, &inspection, &fixture.direct)
                .unwrap_err();
        assert!(error.to_string().contains("changed"));
        assert!(fixture.legacy.is_symlink());
        assert!(!fixture.direct.exists());
        assert_eq!(
            fs::read_to_string(original.join("settings.json")).unwrap(),
            "legacy"
        );
    }
}
