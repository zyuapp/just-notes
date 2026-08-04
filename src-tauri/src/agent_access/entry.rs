#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EntryKind {
    Missing,
    Directory,
    Regular,
    Symlink,
    Other,
}

pub(super) enum RenameChildOutcome {
    Synced,
    RenamedButUnsynced(String),
}

pub(super) enum AtomicWriteOutcome {
    Synced,
    ReplacedButUnsynced(String),
}

impl AtomicWriteOutcome {
    pub(super) fn require_sync(self) -> Result<(), String> {
        match self {
            Self::Synced => Ok(()),
            Self::ReplacedButUnsynced(error) => Err(error),
        }
    }
}
