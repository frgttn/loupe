//! macOS counterpart of `loupe_platform_win::net`, with its signatures.
//! A stub for now: nothing here is implemented on macOS yet.

use crate::UNSUPPORTED;

/// Fetches `url` and returns the body as text. Blocking: call it from a
/// thread of its own.
pub fn get_text(_url: &str) -> Result<String, String> {
    Err(UNSUPPORTED.into())
}

/// Fetches `url` and returns the body, refusing more than `limit` bytes.
/// Blocking: call it from a thread of its own.
pub fn get_bytes(_url: &str, _limit: u32) -> Result<Vec<u8>, String> {
    Err(UNSUPPORTED.into())
}
