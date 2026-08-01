use std::{
    ffi::{OsStr, OsString},
    os::{fd::OwnedFd, unix::ffi::OsStrExt},
    path::Path,
};

use rustix::{
    fs::{
        fstat, open, openat, renameat_with, statat, unlinkat, AtFlags, Dir, FileType, Mode, OFlags,
        RenameFlags,
    },
    io::Errno,
};

use super::DataRootMigrationError;

const DIRECTORY_FLAGS: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);
type ParentEntry<'a> = (&'a Path, &'a OsStr);

pub(super) struct DirectoryInspection {
    directory: Option<OwnedFd>,
    has_data: bool,
}

impl DirectoryInspection {
    pub(super) fn has_data(&self) -> bool {
        self.has_data
    }
}

pub(super) fn inspect_directory(
    path: &Path,
) -> Result<DirectoryInspection, DataRootMigrationError> {
    let Some(directory) = open_root(path)? else {
        return Ok(DirectoryInspection {
            directory: None,
            has_data: false,
        });
    };
    let has_data = entries_have_data(&directory, path)?;
    Ok(DirectoryInspection {
        directory: Some(directory),
        has_data,
    })
}

pub(super) fn prepare_empty_destination(
    path: &Path,
    inspection: &DirectoryInspection,
) -> Result<(), DataRootMigrationError> {
    let Some(directory) = inspection.directory.as_ref() else {
        return ensure_missing(path);
    };
    if inspection.has_data {
        return Err(DataRootMigrationError::FileSystem(format!(
            "Refusing to replace populated data location at {}",
            path.display()
        )));
    }
    let (parent_path, name) = parent_and_name(path)?;
    let parent = open_required_directory(parent_path)?;
    ensure_same_directory(&parent, name, directory, path)?;
    remove_empty_children(directory, path)?;
    ensure_same_directory(&parent, name, directory, path)?;
    unlinkat(&parent, name, AtFlags::REMOVEDIR)
        .map_err(|error| file_error("prepare empty direct", path, error))
}

pub(super) fn move_inspected_directory_no_replace(
    source: &Path,
    source_inspection: &DirectoryInspection,
    destination: &Path,
) -> Result<(), DataRootMigrationError> {
    let Some(directory) = source_inspection.directory.as_ref() else {
        return Err(DataRootMigrationError::FileSystem(format!(
            "Legacy data location disappeared before migration at {}",
            source.display()
        )));
    };
    let (source_parent_path, source_name) = parent_and_name(source)?;
    let (destination_parent_path, destination_name) = parent_and_name(destination)?;
    let source_parent = open_required_directory(source_parent_path)?;
    let destination_parent = open_required_directory(destination_parent_path)?;
    ensure_same_directory(&source_parent, source_name, directory, source)?;
    renameat_with(
        &source_parent,
        source_name,
        &destination_parent,
        destination_name,
        RenameFlags::NOREPLACE,
    )
    .map_err(|error| migration_error(source, destination, error))?;
    ensure_same_directory(
        &destination_parent,
        destination_name,
        directory,
        destination,
    )
    .map_err(|error| migration_error(source, destination, error))
}

fn entries_have_data(directory: &OwnedFd, path: &Path) -> Result<bool, DataRootMigrationError> {
    for name in list_names(directory, path)? {
        let entry_path = path.join(&name);
        let Some(child) = open_child_directory(directory, &name, &entry_path)? else {
            return Ok(true);
        };
        if entries_have_data(&child, &entry_path)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn remove_empty_children(directory: &OwnedFd, path: &Path) -> Result<(), DataRootMigrationError> {
    for name in list_names(directory, path)? {
        let entry_path = path.join(&name);
        let child = open_child_directory(directory, &name, &entry_path)?.ok_or_else(|| {
            DataRootMigrationError::FileSystem(format!(
                "Direct data location became populated at {}",
                entry_path.display()
            ))
        })?;
        remove_empty_children(&child, &entry_path)?;
        ensure_same_directory(directory, &name, &child, &entry_path)?;
        unlinkat(directory, &name, AtFlags::REMOVEDIR)
            .map_err(|error| file_error("remove empty", &entry_path, error))?;
    }
    Ok(())
}

fn list_names(directory: &OwnedFd, path: &Path) -> Result<Vec<OsString>, DataRootMigrationError> {
    let entries = Dir::read_from(directory).map_err(|error| file_error("read", path, error))?;
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| file_error("read", path, error))?;
        let name = entry.file_name().to_bytes();
        if name != b"." && name != b".." {
            names.push(OsStr::from_bytes(name).to_os_string());
        }
    }
    Ok(names)
}

fn ensure_missing(path: &Path) -> Result<(), DataRootMigrationError> {
    match open(path, DIRECTORY_FLAGS, Mode::empty()) {
        Err(Errno::NOENT) => Ok(()),
        Ok(_) | Err(Errno::NOTDIR | Errno::LOOP) => {
            Err(DataRootMigrationError::FileSystem(format!(
                "Data location appeared during migration at {}",
                path.display()
            )))
        }
        Err(error) => Err(file_error("reinspect", path, error)),
    }
}

fn open_root(path: &Path) -> Result<Option<OwnedFd>, DataRootMigrationError> {
    match open(path, DIRECTORY_FLAGS, Mode::empty()) {
        Ok(directory) => Ok(Some(directory)),
        Err(Errno::NOENT) => Ok(None),
        Err(Errno::NOTDIR | Errno::LOOP) => Err(type_error(path)),
        Err(error) => Err(file_error("open", path, error)),
    }
}

fn open_required_directory(path: &Path) -> Result<OwnedFd, DataRootMigrationError> {
    open_root(path)?.ok_or_else(|| {
        DataRootMigrationError::FileSystem(format!(
            "Expected a regular data directory at {}, but it does not exist",
            path.display()
        ))
    })
}

fn open_child_directory(
    parent: &OwnedFd,
    name: &OsStr,
    path: &Path,
) -> Result<Option<OwnedFd>, DataRootMigrationError> {
    match openat(parent, name, DIRECTORY_FLAGS, Mode::empty()) {
        Ok(directory) => Ok(Some(directory)),
        Err(Errno::NOTDIR | Errno::LOOP) => Ok(None),
        Err(error) => Err(file_error("open", path, error)),
    }
}

fn ensure_same_directory(
    parent: &OwnedFd,
    name: &OsStr,
    opened: &OwnedFd,
    path: &Path,
) -> Result<(), DataRootMigrationError> {
    let opened = fstat(opened).map_err(|error| file_error("inspect", path, error))?;
    let current = statat(parent, name, AtFlags::SYMLINK_NOFOLLOW)
        .map_err(|error| file_error("reinspect", path, error))?;
    let current_type = FileType::from_raw_mode(current.st_mode);
    if !current_type.is_dir() || opened.st_dev != current.st_dev || opened.st_ino != current.st_ino
    {
        return Err(DataRootMigrationError::FileSystem(format!(
            "Data location changed while preparing {}",
            path.display()
        )));
    }
    Ok(())
}

fn parent_and_name(path: &Path) -> Result<ParentEntry<'_>, DataRootMigrationError> {
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => Ok((parent, name)),
        _ => Err(DataRootMigrationError::FileSystem(format!(
            "Data root has no parent or name: {}",
            path.display()
        ))),
    }
}

fn type_error(path: &Path) -> DataRootMigrationError {
    DataRootMigrationError::FileSystem(format!(
        "Expected a regular data directory at {}, but found another file type",
        path.display()
    ))
}

fn migration_error(
    source: &Path,
    destination: &Path,
    error: impl std::fmt::Display,
) -> DataRootMigrationError {
    DataRootMigrationError::FileSystem(format!(
        "Failed to migrate Just Notes data from {} to {}: {error}",
        source.display(),
        destination.display()
    ))
}

fn file_error(
    operation: &str,
    path: &Path,
    error: impl std::fmt::Display,
) -> DataRootMigrationError {
    DataRootMigrationError::FileSystem(format!(
        "Failed to {operation} data location at {}: {error}",
        path.display()
    ))
}
