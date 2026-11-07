//! macOS counterpart of `loupe_platform_win::wifi`, with its signatures.
//! A stub for now: nothing here is implemented on macOS yet.

use loupe_core::item::WifiNetwork;

use crate::UNSUPPORTED;

/// Saves `network` and starts connecting to it on the first Wi-Fi adapter.
/// Windows connects in the background; `Ok` says it was asked to.
pub fn connect(_network: &WifiNetwork) -> Result<Option<String>, String> {
    Err(UNSUPPORTED.into())
}
