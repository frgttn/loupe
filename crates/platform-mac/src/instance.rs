//! macOS counterpart of `loupe_platform_win::instance`, with its signatures.
//! A stub for now: nothing here is implemented on macOS yet.

use crate::{Result, UNSUPPORTED};

/// Asks the primary instance to show its launcher window.
pub const SHOW: &str = "show";

/// Asks the primary instance to open its settings window.
pub const SETTINGS: &str = "settings";

pub enum Instance {
    /// This process owns the launcher. Keep the guard alive for the process lifetime.
    Primary(Guard),
    /// Another process already does.
    Secondary,
}

/// Holds the mutex; released when dropped (or when the process exits).
pub struct Guard(());

/// Decides whether this process is the primary instance.
///
/// If the mutex can't be created at all, this process acts as primary: a
/// launcher that runs twice is better than one that never starts.
pub fn acquire() -> Instance {
    // No single-instance check yet: every launch is the primary one.
    Instance::Primary(Guard(()))
}

/// Primary instance: starts a thread that receives messages from later
/// launches and forwards each one to `messages`.
pub fn listen(_messages: async_channel::Sender<String>) {}

/// Secondary instance: delivers `message` to the primary one.
///
/// Also lets the primary instance take the foreground: this process just got
/// user input (it was launched), the primary one didn't.
pub fn send(_message: &str) -> Result<()> {
    Err(UNSUPPORTED.into())
}
