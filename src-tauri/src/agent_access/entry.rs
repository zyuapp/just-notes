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
