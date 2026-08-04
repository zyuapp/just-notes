use std::{
    fs::File,
    io::{BufRead, BufReader, Read, Write},
    os::fd::OwnedFd,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use rustix::{
    fs::{fstat, fsync, open, openat, renameat, unlinkat, AtFlags, FileType, Mode, OFlags},
    io::Errno,
};

use crate::app::now_ms;

use super::super::{repository::render_thread_markdown_content, ThreadMetadata, TranscriptSegment};

const METADATA_NAME: &str = "thread.json";
const TRANSCRIPT_NAME: &str = "transcript.jsonl";
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(super) struct ThreadDirectory {
    fd: OwnedFd,
    path: PathBuf,
}

impl ThreadDirectory {
    pub(super) fn open(path: &Path) -> Result<Option<Self>, String> {
        let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        let fd = match open(path, flags, Mode::empty()) {
            Ok(fd) => fd,
            Err(Errno::NOENT | Errno::NOTDIR | Errno::LOOP) => return Ok(None),
            Err(error) => {
                return Err(format!(
                    "Failed to open thread directory {}: {error}",
                    path.display()
                ))
            }
        };
        let file_type = fstat(&fd)
            .map(|stat| FileType::from_raw_mode(stat.st_mode))
            .map_err(|error| format!("Failed to inspect {}: {error}", path.display()))?;
        if !file_type.is_dir() {
            return Ok(None);
        }
        Ok(Some(Self {
            fd,
            path: path.to_path_buf(),
        }))
    }

    pub(super) fn required(path: &Path) -> Result<Self, String> {
        Self::open(path)?
            .ok_or_else(|| format!("Not a regular thread directory: {}", path.display()))
    }

    pub(super) fn read_metadata(&self) -> Result<ThreadMetadata, String> {
        let mut file = self
            .open_regular_file(METADATA_NAME)?
            .ok_or_else(|| format!("Missing regular metadata file in {}", self.path.display()))?;
        let mut json = String::new();
        file.read_to_string(&mut json).map_err(|error| {
            format!(
                "Failed to read {}/{}: {error}",
                self.path.display(),
                METADATA_NAME
            )
        })?;
        serde_json::from_str(&json)
            .map_err(|error| format!("Invalid {}/{}: {error}", self.path.display(), METADATA_NAME))
    }

    pub(super) fn update_metadata(
        &self,
        preserve_activity: bool,
        apply: impl FnOnce(&mut ThreadMetadata),
    ) -> Result<(), String> {
        let mut metadata = self.read_metadata()?;
        apply(&mut metadata);
        if !preserve_activity {
            metadata.updated_at_ms = now_ms()?;
        }
        self.write_metadata(&metadata)
    }

    pub(super) fn valid_non_empty_transcript(&self) -> Result<bool, String> {
        Ok(!self.read_transcript()?.is_empty())
    }

    pub(super) fn render_markdown(&self) -> Result<(), String> {
        let metadata = self.read_metadata()?;
        let segments = self.read_transcript()?;
        let markdown = render_thread_markdown_content(&metadata, &segments);
        self.write_text_atomic("transcript.md", "thread-markdown", &markdown)
    }

    fn read_transcript(&self) -> Result<Vec<TranscriptSegment>, String> {
        let Some(file) = self.open_regular_file(TRANSCRIPT_NAME)? else {
            return Ok(Vec::new());
        };
        let mut segments = Vec::new();
        for line in BufReader::new(file).lines() {
            let line = line.map_err(|error| {
                format!(
                    "Failed to read {}/{}: {error}",
                    self.path.display(),
                    TRANSCRIPT_NAME
                )
            })?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let segment = serde_json::from_str::<TranscriptSegment>(trimmed).map_err(|error| {
                format!(
                    "Invalid transcript line in {}/{}: {error}",
                    self.path.display(),
                    TRANSCRIPT_NAME
                )
            })?;
            segments.push(segment);
        }
        Ok(segments)
    }

    fn open_regular_file(&self, name: &str) -> Result<Option<File>, String> {
        let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        let fd = match openat(&self.fd, name, flags, Mode::empty()) {
            Ok(fd) => fd,
            Err(Errno::NOENT | Errno::NOTDIR | Errno::LOOP) => return Ok(None),
            Err(error) => {
                return Err(format!(
                    "Failed to open {}/{}: {error}",
                    self.path.display(),
                    name
                ))
            }
        };
        let file_type = fstat(&fd)
            .map(|stat| FileType::from_raw_mode(stat.st_mode))
            .map_err(|error| {
                format!(
                    "Failed to inspect {}/{}: {error}",
                    self.path.display(),
                    name
                )
            })?;
        Ok(file_type.is_file().then(|| File::from(fd)))
    }

    fn write_metadata(&self, metadata: &ThreadMetadata) -> Result<(), String> {
        let json = serde_json::to_string_pretty(metadata)
            .map_err(|error| format!("Failed to encode thread metadata: {error}"))?;
        self.write_text_atomic(METADATA_NAME, "thread-readiness", &json)
    }

    fn write_text_atomic(
        &self,
        target_name: &str,
        temp_label: &str,
        text: &str,
    ) -> Result<(), String> {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp_name = format!(".{temp_label}-{}-{sequence}.tmp", std::process::id());
        let flags =
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        let fd = openat(&self.fd, temp_name.as_str(), flags, Mode::RUSR | Mode::WUSR).map_err(
            |error| {
                format!(
                    "Failed to create metadata update in {}: {error}",
                    self.path.display()
                )
            },
        )?;
        let result = (|| {
            let mut file = File::from(fd);
            file.write_all(text.as_bytes())
                .map_err(|error| format!("Failed to write {target_name}: {error}"))?;
            file.sync_all()
                .map_err(|error| format!("Failed to sync {target_name}: {error}"))?;
            drop(file);
            renameat(&self.fd, temp_name.as_str(), &self.fd, target_name)
                .map_err(|error| format!("Failed to replace {target_name}: {error}"))?;
            fsync(&self.fd).map_err(|error| format!("Failed to sync thread directory: {error}"))
        })();
        if result.is_err() {
            let _ = unlinkat(&self.fd, temp_name.as_str(), AtFlags::empty());
        }
        result
    }
}
