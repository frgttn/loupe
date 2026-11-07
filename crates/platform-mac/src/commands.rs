//! macOS counterpart of `loupe_platform_win::commands`, with its signatures.
//! A stub for now: nothing here is implemented on macOS yet.

use crate::system::Outcome;
use loupe_core::item::Command;

use crate::UNSUPPORTED;

/// Runs `command` on a thread of its own (a slow shell, a UAC prompt or a
/// system command never blocks the UI). Its outcome, a message for the HUD
/// or why it failed, arrives on the returned channel.
pub fn spawn(_command: Command) -> async_channel::Receiver<Outcome> {
    async_channel::unbounded().1
}

/// Opens `target` (a file, folder or URL) with its default app and waits
/// for the shell to take it. Starts COM in the calling thread.
pub fn open_blocking(_target: &str) -> Result<(), String> {
    Err(UNSUPPORTED.into())
}
