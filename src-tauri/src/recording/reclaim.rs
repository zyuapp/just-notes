use super::RecorderState;
use crate::{
    app::AppPaths,
    threads::{storage, StorageUsage},
};

/// Deletes the raw audio of every idle thread. Recording owns this because it
/// mutates the thread store: without the operation lock a recording could start
/// between the idle check and the unlink, and the walk would delete the WAVs the
/// live audio sink is still writing.
pub(crate) fn reclaim_raw_audio(
    paths: &AppPaths,
    recorder: &RecorderState,
) -> Result<StorageUsage, String> {
    let _operation_guard = recorder.lock_operation()?;
    storage::delete_reclaimable_raw_audio(paths)?;
    storage::usage(paths)
}
