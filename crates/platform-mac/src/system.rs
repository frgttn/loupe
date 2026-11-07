//! macOS counterpart of `loupe_platform_win::system`, with its signatures.
//! A stub for now: nothing here is implemented on macOS yet.

use loupe_core::item::SystemCommand;

use crate::UNSUPPORTED;

/// What a command reports: a short message for the HUD ("Bluetooth Off",
/// "Volume 40%"), nothing, or why it failed, in words for the user.
pub type Outcome = Result<Option<String>, String>;

/// Runs `command` in the background; its outcome arrives on the returned channel.
pub fn spawn(_command: SystemCommand) -> async_channel::Receiver<Outcome> {
    async_channel::unbounded().1
}

/// Whether apps currently use the dark theme.
pub fn dark_mode() -> bool {
    false
}

/// Whether the taskbar and tray use the dark theme (a separate setting from apps).
pub fn taskbar_dark() -> bool {
    false
}

/// For each active display that supports HDR: its GDI name, whether HDR is
/// on, and whether the Windows 11 call answered. For diagnostics.
pub fn hdr_status() -> Result<Vec<(String, bool, bool)>, String> {
    Err(UNSUPPORTED.into())
}
