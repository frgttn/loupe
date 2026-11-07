//! macOS counterpart of `loupe_platform_win::process`, with its signatures.
//! A stub for now: nothing here is implemented on macOS yet.

/// `(working set, committed private)` in bytes: the physical memory mapped
/// into the process (Task Manager's details tab) and its private commit
/// (close to Task Manager's "Memory" column). `None` if Windows won't say.
pub fn memory() -> Option<(u64, u64)> {
    None
}
