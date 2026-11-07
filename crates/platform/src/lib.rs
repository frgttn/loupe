//! Loupe's platform layer: the OS integration the rest of the app uses (hotkey,
//! clipboard, app discovery and icons, running commands, window management,
//! screen capture, measurements).
//!
//! `app`, `ui` and `providers` import it as `loupe_platform::<module>` and
//! never name an OS crate. Each OS crate (`loupe-platform-win`,
//! `loupe-platform-mac`, later `loupe-platform-linux`) offers the same modules
//! with the same signatures, and this crate re-exports the one for the target,
//! so switching OS needs no `cfg` in the callers.
//!
//! What needs no OS of its own lives here instead, written once on top of
//! the OS crate's primitives: [`metrics`] (on `process::memory`), [`lens`]
//! (on `commands::open_blocking`) and [`favicon`] (on `net`).
//!
//! Window handles cross this boundary as raw `isize` values, events as
//! `async_channel` receivers; nothing here knows about GPUI.
//!
//! Building with `RUSTFLAGS="--cfg loupe_mac_stub"` on Windows uses the macOS
//! crate instead, as if the target were macOS: a quick check, without a Mac,
//! that it still offers everything the callers use.

#[cfg(all(windows, not(loupe_mac_stub)))]
use loupe_platform_win as os;
#[cfg(all(windows, not(loupe_mac_stub)))]
pub use loupe_platform_win::*;

#[cfg(any(target_os = "macos", loupe_mac_stub))]
use loupe_platform_mac as os;
#[cfg(any(target_os = "macos", loupe_mac_stub))]
pub use loupe_platform_mac::*;

#[cfg(not(any(windows, target_os = "macos")))]
compile_error!("Loupe has no platform layer for this OS yet: only Windows and macOS");

pub mod favicon;
pub mod lens;
pub mod metrics;
