use std::{
    fs::File,
    io::Write,
    sync::atomic::{AtomicU64, Ordering},
};

use rustix::fs::{fsync, openat, renameat, unlinkat, AtFlags, Mode, OFlags};

use super::{entry::AtomicWriteOutcome, filesystem::SafeDirectory};

#[cfg(test)]
use std::cell::Cell;

#[cfg(test)]
thread_local! {
    static SYNC_FAULT: Cell<bool> = const { Cell::new(false) };
}

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

impl SafeDirectory {
    pub(super) fn type_error(&self, name: &str, expected: &str) -> String {
        format!("{}/{} is not {expected}", self.path.display(), name)
    }

    pub(super) fn sync(&self) -> Result<(), String> {
        #[cfg(test)]
        if SYNC_FAULT.with(|fault| fault.replace(false)) {
            return Err(format!("Injected sync failure for {}", self.path.display()));
        }
        fsync(&self.fd).map_err(|error| format!("Failed to sync {}: {error}", self.path.display()))
    }

    pub(super) fn write_atomic(
        &self,
        name: &str,
        label: &str,
        bytes: &[u8],
    ) -> Result<AtomicWriteOutcome, String> {
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
            Ok(match self.sync() {
                Ok(()) => AtomicWriteOutcome::Synced,
                Err(error) => AtomicWriteOutcome::ReplacedButUnsynced(error),
            })
        })();
        if result.is_err() {
            let _ = unlinkat(&self.fd, temp_name.as_str(), AtFlags::empty());
        }
        result
    }
}

#[cfg(test)]
pub(super) fn fail_next_sync() {
    SYNC_FAULT.with(|fault| fault.set(true));
}
