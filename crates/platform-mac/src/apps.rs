//! macOS counterpart of `loupe_platform_win::apps`, with its signatures.
//! A stub for now: nothing here is implemented on macOS yet.

use crate::{Result, UNSUPPORTED};

#[derive(Clone, Debug)]
pub struct AppEntry {
    /// Name as shown in the Start menu.
    pub name: String,
    /// AppsFolder parsing name: an AUMID for packaged apps, a (known-folder relative) path otherwise.
    pub launch_id: String,
    /// The program or shortcut on disk, for Win32 apps whose parsing name is a
    /// path. `None` for packaged apps and apps known only by an AUMID.
    pub path: Option<String>,
}

/// Enumerates all apps. Blocking and COM-heavy: call it from a dedicated thread.
pub fn discover() -> Result<Vec<AppEntry>> {
    Err(UNSUPPORTED.into())
}
