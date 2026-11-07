//! macOS counterpart of `loupe_platform_win::drives`, with its signatures.
//! A stub for now: nothing here is implemented on macOS yet.

use crate::UNSUPPORTED;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Drive {
    pub letter: char,
    /// The volume label, or what kind of drive it is.
    pub label: String,
    pub kind: DriveKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DriveKind {
    /// USB sticks, card readers, external USB disks.
    Removable,
    /// CD/DVD/Blu-ray drives: ejecting opens the tray.
    Optical,
}

/// Drives that can be ejected: removable and optical drives, and fixed disks
/// on USB (external hard drives report themselves as fixed). Network drives
/// and internal disks are left out. Takes a few milliseconds.
pub fn removable() -> Vec<Drive> {
    Vec::new()
}

/// Ejects drive `letter` and says how it went. Blocking: call it from a
/// thread of its own. Windows shows its own message if something keeps the
/// drive busy.
pub fn eject(_letter: char) -> Result<String, String> {
    Err(UNSUPPORTED.into())
}

/// Ejects every removable drive; reports which ones went and which didn't.
pub fn eject_all() -> Result<String, String> {
    Err(UNSUPPORTED.into())
}
