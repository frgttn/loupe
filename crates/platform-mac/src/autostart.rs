//! macOS counterpart of `loupe_platform_win::autostart`, with its signatures.
//! A stub for now: nothing here is implemented on macOS yet.

use crate::UNSUPPORTED;

/// Whether Loupe is registered to start at login (from any path) and not
/// disabled in Task Manager.
pub fn is_enabled() -> bool {
    false
}

/// Registers the running executable to start at login, or removes the entry.
pub fn set_enabled(_enabled: bool) -> Result<(), String> {
    Err(UNSUPPORTED.into())
}
