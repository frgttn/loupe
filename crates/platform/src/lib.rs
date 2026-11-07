//! Loupe's platform layer: the OS integration the rest of the app uses (hotkey,
//! clipboard, app discovery and icons, running commands, window management,
//! screen capture, measurements).
//!
//! `app`, `ui` and `providers` import it as `loupe_platform::<module>` and
//! never name an OS crate. Each OS crate (`loupe-platform-win`, later
//! `loupe-platform-mac` and `loupe-platform-linux`) offers the same modules
//! with the same signatures, and this crate re-exports the one for the target,
//! so switching OS needs no `cfg` in the callers.
//!
//! Window handles cross this boundary as raw `isize` values, events as
//! `async_channel` receivers; nothing here knows about GPUI.

#[cfg(windows)]
pub use loupe_platform_win::*;

#[cfg(not(windows))]
compile_error!("Loupe has no platform layer for this OS yet: only Windows is implemented");
