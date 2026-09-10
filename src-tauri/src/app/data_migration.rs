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

const LEGACY_RELATIVE_DATA_ROOTS: [&str; 2] = [
    "Library/Application Support/dev.just-notes",
    "Library/Containers/dev.just-notes/Data/Library/Application Support/dev.just-notes",
];

pub(crate) struct DataRootMigration {
    direct_root: PathBuf,
    legacy_roots: Vec<PathBuf>,
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
            legacy_roots: LEGACY_RELATIVE_DATA_ROOTS
                .iter()
                .map(|relative| home_dir.join(relative))
                .collect(),
        }
    }

    pub(crate) fn resolve(self) -> Result<PathBuf, DataRootMigrationError> {
        let legacy_roots: Vec<PathBuf> = self
            .legacy_roots
            .into_iter()
            .filter(|root| *root != self.direct_root)
            .collect();
        if legacy_roots.is_empty() {
            return Ok(self.direct_root);
        }

        let _migration_lock = MigrationLock::acquire(&self.direct_root)?;
        let direct = inspect_directory(&self.direct_root)?;
        let mut populated = Vec::new();
        for root in legacy_roots {
            let inspection = inspect_directory(&root)?;
            if inspection.has_data() {
                populated.push((root, inspection));
            }
        }

        match (direct.has_data(), populated.len()) {
            (_, 0) => Ok(self.direct_root),
            (true, _) => Err(DataRootMigrationError::Conflict {
                direct_root: self.direct_root,
                legacy_root: populated.remove(0).0,
            }),
            (false, 1) => {
                let (legacy_root, legacy) = populated.remove(0);
                prepare_empty_destination(&self.direct_root, &direct)?;
                move_inspected_directory_no_replace(&legacy_root, &legacy, &self.direct_root)?;
                Ok(self.direct_root)
            }
            (false, _) => {
                let (first, _) = populated.remove(0);
                let (second, _) = populated.remove(0);
                Err(DataRootMigrationError::Conflict {
                    direct_root: first,
                    legacy_root: second,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests;
