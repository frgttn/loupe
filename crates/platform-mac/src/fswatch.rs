//! macOS counterpart of `loupe_platform_win::fswatch`, with its signatures.
//! A stub for now: nothing here is implemented on macOS yet.

use std::path::{Path, PathBuf};

use crate::UNSUPPORTED;

/// What changed, by full path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    Added(PathBuf),
    Removed(PathBuf),
    /// Written to, or its attributes changed.
    Modified(PathBuf),
    Renamed {
        from: PathBuf,
        to: PathBuf,
    },
    /// Changes were lost: the whole folder needs a rescan.
    Overflow,
    /// The watch ended (the folder was deleted, or the drive went away).
    Stopped(String),
}

/// Watches until dropped.
pub struct Watcher {
    _private: (),
}

/// Starts watching `root` and everything below it. Each batch of changes
/// Windows reports arrives on `changes`, in order.
pub fn watch(
    _root: &Path,
    _changes: async_channel::Sender<Vec<Change>>,
) -> Result<Watcher, String> {
    Err(UNSUPPORTED.into())
}
