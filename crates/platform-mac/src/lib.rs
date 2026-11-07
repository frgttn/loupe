//! macOS integration for Loupe, with the same modules and signatures as
//! `loupe-platform-win` so `loupe-platform` can re-export either.
//!
//! For now every module is a stub: calls that report an outcome say macOS
//! isn't supported yet ([`UNSUPPORTED`]), the rest find nothing or do
//! nothing, so the app builds and starts on a Mac while each part gets its
//! real implementation. Window handles are opaque `isize` values here too
//! (an `NSView` pointer, from [`win::native_handle`]).

/// What a stubbed call reports instead of an outcome.
pub const UNSUPPORTED: &str = "not available on macOS yet";

pub mod apps;
pub mod autostart;
pub mod clipboard;
pub mod commands;
pub mod dialog;
pub mod drives;
pub mod expander;
pub mod fswalk;
pub mod fswatch;
pub mod hotkey;
pub mod icons;
pub mod index_service;
pub mod instance;
pub mod links;
pub mod net;
pub mod ntfs;
pub mod ocr;
pub mod paths;
pub mod process;
pub mod protocol;
pub mod region;
pub mod run;
pub mod screen;
pub mod selection;
pub mod system;
pub mod vault;
pub mod wifi;
pub mod win;
pub mod wm;

/// What fails here: the same face as `windows::core::Error` (`Display`,
/// [`Error::message`]), so callers handle either the same way.
#[derive(Clone, Debug)]
pub struct Error(String);

impl Error {
    pub fn message(&self) -> String {
        self.0.clone()
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl From<&str> for Error {
    fn from(message: &str) -> Self {
        Self(message.to_owned())
    }
}

/// `windows::core::Result`'s counterpart.
pub type Result<T> = std::result::Result<T, Error>;
