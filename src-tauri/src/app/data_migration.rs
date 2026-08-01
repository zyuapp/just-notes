use std::{
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
};

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

        let direct_has_data = directory_has_data(&self.direct_root)?;
        let legacy_has_data = directory_has_data(&self.legacy_root)?;

        match (direct_has_data, legacy_has_data) {
            (true, true) => Err(DataRootMigrationError::Conflict {
                direct_root: self.direct_root,
                legacy_root: self.legacy_root,
            }),
            (false, true) => {
                replace_empty_destination(&self.direct_root)?;
                let parent = self.direct_root.parent().ok_or_else(|| {
                    DataRootMigrationError::FileSystem(format!(
                        "Direct data root has no parent: {}",
                        self.direct_root.display()
                    ))
                })?;
                fs::create_dir_all(parent).map_err(|error| {
                    DataRootMigrationError::FileSystem(format!(
                        "Failed to prepare direct data location at {}: {error}",
                        parent.display()
                    ))
                })?;
                fs::rename(&self.legacy_root, &self.direct_root).map_err(|error| {
                    DataRootMigrationError::FileSystem(format!(
                        "Failed to migrate Just Notes data from {} to {}: {error}",
                        self.legacy_root.display(),
                        self.direct_root.display()
                    ))
                })?;
                Ok(self.direct_root)
            }
            _ => Ok(self.direct_root),
        }
    }
}

fn directory_has_data(path: &Path) -> Result<bool, DataRootMigrationError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(DataRootMigrationError::FileSystem(format!(
                "Failed to inspect data location at {}: {error}",
                path.display()
            )))
        }
    };

    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(DataRootMigrationError::FileSystem(format!(
            "Expected a regular data directory at {}, but found another file type",
            path.display()
        )));
    }

    let mut entries = fs::read_dir(path).map_err(|error| {
        DataRootMigrationError::FileSystem(format!(
            "Failed to read data location at {}: {error}",
            path.display()
        ))
    })?;
    Ok(entries
        .next()
        .transpose()
        .map_err(|error| {
            DataRootMigrationError::FileSystem(format!(
                "Failed to inspect data location at {}: {error}",
                path.display()
            ))
        })?
        .is_some())
}

fn replace_empty_destination(path: &Path) -> Result<(), DataRootMigrationError> {
    match fs::remove_dir(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(DataRootMigrationError::FileSystem(format!(
            "Failed to prepare empty direct data location at {}: {error}",
            path.display()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::DataRootMigration;
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
        fs::create_dir_all(&fixture.direct).unwrap();
        fs::create_dir_all(&fixture.legacy).unwrap();
        fs::write(fixture.legacy.join("settings.json"), "{}").unwrap();

        fixture.migration().resolve().unwrap();

        assert!(fixture.direct.join("settings.json").is_file());
    }

    #[test]
    fn refuses_to_merge_two_populated_roots() {
        let fixture = Fixture::new("conflict");
        fs::create_dir_all(&fixture.direct).unwrap();
        fs::create_dir_all(&fixture.legacy).unwrap();
        fs::write(fixture.direct.join("settings.json"), "direct").unwrap();
        fs::write(fixture.legacy.join("settings.json"), "legacy").unwrap();

        let error = fixture.migration().resolve().unwrap_err();

        assert!(error.to_string().contains("will not merge"));
        assert_eq!(
            fs::read_to_string(fixture.direct.join("settings.json")).unwrap(),
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
}
