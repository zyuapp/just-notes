use std::{
    ffi::{OsStr, OsString},
    fs::File,
    io::{Read, Write},
    os::{fd::OwnedFd, unix::ffi::OsStrExt},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use rustix::{
    fs::{
        fstat, mkdirat, open, openat, readlinkat, renameat, renameat_with, statat, symlinkat,
        unlinkat, AtFlags, Dir, FileType, Mode, OFlags, RenameFlags,
    },
    io::Errno,
};

use super::entry::{EntryKind, RenameChildOutcome};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);
type OptionalFileBytes = Option<Vec<u8>>;
pub(super) struct SafeDirectory {
    pub(super) fd: OwnedFd,
    pub(super) path: PathBuf,
}

impl SafeDirectory {
    pub(super) fn open_absolute(path: &Path) -> Result<Self, String> {
        let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        let fd = open(path, flags, Mode::empty())
            .map_err(|error| format!("Failed to open directory {}: {error}", path.display()))?;
        Self::from_fd(fd, path.to_path_buf())
    }

    pub(super) fn entry_kind(&self, name: &str) -> Result<EntryKind, String> {
        match statat(&self.fd, name, AtFlags::SYMLINK_NOFOLLOW) {
            Ok(stat) => {
                let file_type = FileType::from_raw_mode(stat.st_mode);
                Ok(if file_type.is_dir() {
                    EntryKind::Directory
                } else if file_type.is_file() {
                    EntryKind::Regular
                } else if file_type.is_symlink() {
                    EntryKind::Symlink
                } else {
                    EntryKind::Other
                })
            }
            Err(Errno::NOENT) => Ok(EntryKind::Missing),
            Err(error) => Err(format!(
                "Failed to inspect {}/{}: {error}",
                self.path.display(),
                name
            )),
        }
    }

    pub(super) fn open_child(&self, name: &str) -> Result<Option<Self>, String> {
        match self.entry_kind(name)? {
            EntryKind::Missing => return Ok(None),
            EntryKind::Directory => {}
            _ => return Err(self.type_error(name, "a regular directory")),
        }
        let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        let fd = openat(&self.fd, name, flags, Mode::empty())
            .map_err(|error| format!("Failed to open {}/{}: {error}", self.path.display(), name))?;
        Self::from_fd(fd, self.path.join(name)).map(Some)
    }

    pub(super) fn create_child(&self, name: &str) -> Result<Self, String> {
        if self.entry_kind(name)? == EntryKind::Missing {
            mkdirat(&self.fd, name, Mode::RUSR | Mode::WUSR | Mode::XUSR).map_err(|error| {
                format!("Failed to create {}/{}: {error}", self.path.display(), name)
            })?;
            self.sync()?;
        }
        self.open_child(name)?.ok_or_else(|| {
            format!(
                "Directory disappeared while opening {}/{}",
                self.path.display(),
                name
            )
        })
    }

    pub(super) fn create_child_exclusive(&self, name: &str) -> Result<Self, String> {
        mkdirat(&self.fd, name, Mode::RUSR | Mode::WUSR | Mode::XUSR).map_err(|error| {
            format!("Failed to create {}/{}: {error}", self.path.display(), name)
        })?;
        self.sync()?;
        self.open_child(name)?.ok_or_else(|| {
            format!(
                "Directory disappeared while opening {}/{}",
                self.path.display(),
                name
            )
        })
    }

    pub(super) fn list_names(&self) -> Result<Vec<OsString>, String> {
        let mut names = Vec::new();
        let entries = Dir::read_from(&self.fd)
            .map_err(|error| format!("Failed to read {}: {error}", self.path.display()))?;
        for entry in entries {
            let entry = entry
                .map_err(|error| format!("Failed to read {}: {error}", self.path.display()))?;
            let name = entry.file_name().to_bytes();
            if name != b"." && name != b".." {
                names.push(OsStr::from_bytes(name).to_os_string());
            }
        }
        Ok(names)
    }

    pub(super) fn read_regular(&self, name: &str) -> Result<OptionalFileBytes, String> {
        match self.entry_kind(name)? {
            EntryKind::Missing => return Ok(None),
            EntryKind::Regular => {}
            _ => return Err(self.type_error(name, "a regular file")),
        }
        let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        let fd = openat(&self.fd, name, flags, Mode::empty())
            .map_err(|error| format!("Failed to open {}/{}: {error}", self.path.display(), name))?;
        let mut file = File::from(fd);
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|error| format!("Failed to read {}/{}: {error}", self.path.display(), name))?;
        Ok(Some(bytes))
    }

    pub(super) fn write_atomic(&self, name: &str, label: &str, bytes: &[u8]) -> Result<(), String> {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp_name = format!(".{label}-{}-{sequence}.tmp", std::process::id());
        let flags =
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        let fd = openat(&self.fd, temp_name.as_str(), flags, Mode::RUSR | Mode::WUSR).map_err(
            |error| {
                format!(
                    "Failed to create update in {}: {error}",
                    self.path.display()
                )
            },
        )?;
        let result = (|| {
            let mut file = File::from(fd);
            file.write_all(bytes)
                .map_err(|error| format!("Failed to write {name}: {error}"))?;
            file.sync_all()
                .map_err(|error| format!("Failed to sync {name}: {error}"))?;
            drop(file);
            renameat(&self.fd, temp_name.as_str(), &self.fd, name)
                .map_err(|error| format!("Failed to replace {name}: {error}"))?;
            self.sync()
        })();
        if result.is_err() {
            let _ = unlinkat(&self.fd, temp_name.as_str(), AtFlags::empty());
        }
        result
    }

    pub(super) fn create_empty_file(&self, name: &str) -> Result<(), String> {
        match self.entry_kind(name)? {
            EntryKind::Regular => return Ok(()),
            EntryKind::Missing => {}
            _ => return Err(self.type_error(name, "a regular file")),
        }
        let flags =
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        let file = File::from(
            openat(&self.fd, name, flags, Mode::RUSR | Mode::WUSR).map_err(|error| {
                format!("Failed to create {}/{}: {error}", self.path.display(), name)
            })?,
        );
        file.sync_all()
            .map_err(|error| format!("Failed to sync {}/{}: {error}", self.path.display(), name))?;
        self.sync()
    }

    pub(super) fn read_symlink(&self, name: &str) -> Result<Option<PathBuf>, String> {
        match self.entry_kind(name)? {
            EntryKind::Missing => Ok(None),
            EntryKind::Symlink => readlinkat(&self.fd, name, Vec::new())
                .map(|target| PathBuf::from(OsStr::from_bytes(target.as_bytes())))
                .map(Some)
                .map_err(|error| {
                    format!("Failed to read {}/{}: {error}", self.path.display(), name)
                }),
            _ => Err(self.type_error(name, "a symbolic link")),
        }
    }

    pub(super) fn create_symlink(&self, name: &str, target: &Path) -> Result<(), String> {
        if self.entry_kind(name)? != EntryKind::Missing {
            return Err(format!("{}/{} already exists", self.path.display(), name));
        }
        symlinkat(target, &self.fd, name)
            .map_err(|error| format!("Failed to link {}/{}: {error}", self.path.display(), name))?;
        self.sync()
    }

    pub(super) fn unlink(&self, name: &str) -> Result<(), String> {
        unlinkat(&self.fd, name, AtFlags::empty()).map_err(|error| {
            format!("Failed to remove {}/{}: {error}", self.path.display(), name)
        })?;
        self.sync()
    }

    pub(super) fn remove_child_dir(&self, name: &str) -> Result<(), String> {
        unlinkat(&self.fd, name, AtFlags::REMOVEDIR).map_err(|error| {
            format!("Failed to remove {}/{}: {error}", self.path.display(), name)
        })?;
        self.sync()
    }

    pub(super) fn rename_child(
        &self,
        old_name: &str,
        new_name: &str,
    ) -> Result<RenameChildOutcome, String> {
        renameat_with(
            &self.fd,
            old_name,
            &self.fd,
            new_name,
            RenameFlags::NOREPLACE,
        )
        .map_err(|error| {
            format!(
                "Failed to rename an item to {}/{}: {error}",
                self.path.display(),
                new_name
            )
        })?;
        Ok(match self.sync() {
            Ok(()) => RenameChildOutcome::Synced,
            Err(error) => RenameChildOutcome::RenamedButUnsynced(error),
        })
    }

    fn from_fd(fd: OwnedFd, path: PathBuf) -> Result<Self, String> {
        let file_type = fstat(&fd)
            .map(|stat| FileType::from_raw_mode(stat.st_mode))
            .map_err(|error| format!("Failed to inspect {}: {error}", path.display()))?;
        if !file_type.is_dir() {
            return Err(format!("{} is not a regular directory", path.display()));
        }
        Ok(Self { fd, path })
    }
}
