//! macOS counterpart of `loupe_platform_win::protocol`, with its signatures.
//! A stub for now: nothing here is implemented on macOS yet.

use crate::UNSUPPORTED;

/// Points the scheme at the running executable, unless it already does.
/// Whichever copy of Loupe ran last owns the links.
pub fn register(_scheme: &str) -> Result<(), String> {
    Err(UNSUPPORTED.into())
}

/// Removes the scheme, if it points at the running executable: another copy
/// of Loupe may own the links now.
pub fn unregister(_scheme: &str) -> Result<(), String> {
    Err(UNSUPPORTED.into())
}
