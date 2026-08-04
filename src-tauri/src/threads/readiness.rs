use std::path::Path;

use crate::app::AppPaths;

use super::{repository::thread_dirs_in, RetrievalReadiness, ThreadStatus};

mod storage;
use storage::ThreadDirectory;

pub(crate) fn migrate_retrieval_readiness(paths: &AppPaths) -> Result<(), String> {
    for thread_dir in thread_dirs_in(&paths.threads_dir)? {
        let Some(thread) = ThreadDirectory::open(&thread_dir)? else {
            continue;
        };
        let Ok(metadata) = thread.read_metadata() else {
            continue;
        };
        if metadata.retrieval_readiness != RetrievalReadiness::Unknown {
            continue;
        }

        let readiness = if matches!(metadata.status, ThreadStatus::Idle) {
            transcript_readiness(&thread)
        } else {
            RetrievalReadiness::Unavailable
        };
        thread.update_metadata(true, |metadata| {
            metadata.retrieval_readiness = readiness;
        })?;
    }
    Ok(())
}

pub(crate) fn mark_recording_started(thread_dir: &Path) -> Result<(), String> {
    let thread = ThreadDirectory::required(thread_dir)?;
    thread.update_metadata(false, |metadata| {
        metadata.status = ThreadStatus::Recording;
        metadata.retrieval_readiness = RetrievalReadiness::Unavailable;
    })
}

pub(crate) fn mark_recording_aborted(thread_dir: &Path) -> Result<(), String> {
    let thread = ThreadDirectory::required(thread_dir)?;
    let readiness = transcript_readiness(&thread);
    thread.update_metadata(false, |metadata| {
        metadata.status = ThreadStatus::Idle;
        metadata.retrieval_readiness = readiness;
    })
}

pub(crate) fn resolve_retrieval_readiness(thread_dir: &Path) -> Result<(), String> {
    let thread = ThreadDirectory::required(thread_dir)?;
    let metadata = thread.read_metadata()?;
    let readiness = if matches!(metadata.status, ThreadStatus::Idle) {
        transcript_readiness(&thread)
    } else {
        RetrievalReadiness::Unavailable
    };
    thread.update_metadata(true, |metadata| {
        metadata.retrieval_readiness = readiness;
    })
}

pub(crate) fn mark_recording_finished(
    thread_dir: &Path,
    duration_ms: u64,
    markdown_copy: bool,
) -> Result<bool, String> {
    let thread = ThreadDirectory::required(thread_dir)?;
    let readiness = transcript_readiness(&thread);
    thread.update_metadata(false, |metadata| {
        metadata.status = ThreadStatus::Idle;
        metadata.duration_ms = duration_ms;
        metadata.retrieval_readiness = readiness;
    })?;
    if markdown_copy {
        thread.render_markdown()?;
    }
    is_open_thread_retrievable(&thread)
}

#[cfg(test)]
pub(crate) fn is_thread_retrievable(thread_dir: &Path) -> Result<bool, String> {
    let Some(thread) = ThreadDirectory::open(thread_dir)? else {
        return Ok(false);
    };
    is_open_thread_retrievable(&thread)
}

fn is_open_thread_retrievable(thread: &ThreadDirectory) -> Result<bool, String> {
    let metadata = thread.read_metadata()?;
    if metadata.status.is_busy() || metadata.retrieval_readiness != RetrievalReadiness::Ready {
        return Ok(false);
    }

    thread.valid_non_empty_transcript()
}

fn transcript_readiness(thread: &ThreadDirectory) -> RetrievalReadiness {
    match thread.valid_non_empty_transcript() {
        Ok(true) => RetrievalReadiness::Ready,
        Ok(false) | Err(_) => RetrievalReadiness::Unavailable,
    }
}

#[cfg(test)]
mod tests;
