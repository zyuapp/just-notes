use std::{fs, os::fd::OwnedFd, path::Path};

use rustix::{
    fs::{flock, fstat, open, FileType, FlockOperation, Mode, OFlags},
    io::Errno,
};

use super::super::DataRootMigrationError;

const LOCK_NAME: &str = ".dev.just-notes-migration.lock";

pub(super) struct MigrationLock {
    _file: OwnedFd,
}

impl MigrationLock {
    pub(super) fn acquire(direct_root: &Path) -> Result<Self, DataRootMigrationError> {
        let parent = direct_root.parent().ok_or_else(|| {
            DataRootMigrationError::FileSystem(format!(
                "Direct data root has no parent: {}",
                direct_root.display()
            ))
        })?;
        fs::create_dir_all(parent).map_err(|error| {
            DataRootMigrationError::FileSystem(format!(
                "Failed to prepare direct data location at {}: {error}",
                parent.display()
            ))
        })?;
        let path = parent.join(LOCK_NAME);
        let flags = OFlags::RDWR | OFlags::CREATE | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        let file = open(&path, flags, Mode::RUSR | Mode::WUSR).map_err(|error| match error {
            Errno::LOOP => DataRootMigrationError::FileSystem(format!(
                "Expected a regular migration lock at {}, but found a symbolic link",
                path.display()
            )),
            _ => DataRootMigrationError::FileSystem(format!(
                "Failed to open migration lock at {}: {error}",
                path.display()
            )),
        })?;
        let file_type = fstat(&file)
            .map(|stat| FileType::from_raw_mode(stat.st_mode))
            .map_err(|error| {
                DataRootMigrationError::FileSystem(format!(
                    "Failed to inspect migration lock at {}: {error}",
                    path.display()
                ))
            })?;
        if !file_type.is_file() {
            return Err(DataRootMigrationError::FileSystem(format!(
                "Expected a regular migration lock at {}, but found another file type",
                path.display()
            )));
        }
        flock(&file, FlockOperation::LockExclusive).map_err(|error| {
            DataRootMigrationError::FileSystem(format!(
                "Failed to lock data migration at {}: {error}",
                path.display()
            ))
        })?;
        Ok(Self { _file: file })
    }
}

#[cfg(test)]
mod tests {
    use super::{MigrationLock, LOCK_NAME};
    use rustix::{
        fs::{flock, open, FlockOperation, Mode, OFlags},
        io::Errno,
    };
    use std::{env, fs, time::UNIX_EPOCH};

    #[test]
    fn serializes_migration_processes() {
        let stamp = UNIX_EPOCH.elapsed().unwrap().as_nanos();
        let root = env::temp_dir().join(format!("just-notes-migration-lock-{stamp}"));
        let direct = root.join("dev.just-notes");
        let first = MigrationLock::acquire(&direct).unwrap();
        let second = open(
            root.join(LOCK_NAME),
            OFlags::RDWR | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .unwrap();

        assert_eq!(
            flock(&second, FlockOperation::NonBlockingLockExclusive),
            Err(Errno::WOULDBLOCK)
        );
        drop((first, second));
        fs::remove_dir_all(root).unwrap();
    }
}
