//! macOS counterpart of `loupe_platform_win::win`, with its signatures.
//! A stub for now: nothing here is implemented on macOS yet.

use loupe_core::clip::IgnoredApp;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// The apps with a window on screen (as the taskbar would show them), each
/// once, by name; not Loupe. For choosing apps in Settings.
/// The OS handle of a window the UI made (its HWND), for the other calls in
/// this crate. `None` if it isn't a Win32 window.
pub fn native_handle(window: &impl HasWindowHandle) -> Option<isize> {
    match window.window_handle().ok()?.as_raw() {
        RawWindowHandle::AppKit(handle) => Some(handle.ns_view.as_ptr() as isize),
        _ => None,
    }
}

pub fn apps_with_windows() -> Vec<IgnoredApp> {
    Vec::new()
}

/// Asks DWM for Windows 11 rounded corners (frameless popups don't get them by default).
pub fn round_corners(_raw: isize) {}

/// Clips the window to the top `logical_height` of its client area (all of
/// it with `None`). Clicks below go to the windows beneath, and Windows
/// stops drawing its own rounded corners and border: a transparent window
/// draws them itself, inside.
///
/// Sends WM_WINDOWPOSCHANGED, like a resize: call it from a spawned task or
/// an update, not while GPUI draws.
pub fn clip_height(_raw: isize, _logical_height: Option<f32>) {}

/// Sets the colour (`0xRRGGBB`) of the thin border Windows 11 draws around
/// the window, along its rounded corners.
pub fn set_border_color(_raw: isize, _rgb: u32) {}

/// Shows the window topmost, centered (as Raycast places it) in
/// the work area of the monitor under the mouse cursor, then takes focus.
///
/// `logical_size` (in 96-DPI units) is scaled by that monitor's DPI and applied;
/// pass `None` to keep the window's current size. With `client_height`, the
/// window is placed as if it had `logical_size` but is only that tall inside
/// (a collapsed launcher), so growing it later keeps its top edge.
///
/// GPUI misses a size change made here while it updates the window (see
/// [`resize`]), so the window should already have the size asked for.
pub fn show_on_active_monitor(
    _raw: isize,
    _logical_size: Option<(f32, f32)>,
    _client_height: Option<f32>,
) {
}

/// Changes the window's size, keeping its top-left corner and z-order.
/// `logical_size` and `client_height` are as for [`show_on_active_monitor`].
///
/// GPUI learns the new size from WM_SIZE, which it can only take while no
/// update of the window is running: call this from a spawned task, as GPUI's
/// own resize does, or its layout keeps the old size.
pub fn resize(_raw: isize, _logical_size: (f32, f32), _client_height: Option<f32>) {}

/// Shows the window topmost, horizontally centered near the bottom of the work
/// area of the monitor under the mouse cursor, without activating it: focus
/// stays with whatever window has it. `logical_size` is in 96-DPI units.
pub fn show_without_focus_near_bottom(_raw: isize, _logical_size: (f32, f32)) {}

/// Lets a GUI-subsystem process print to the console it was started from, if
/// any. Output that is redirected (to a file or a pipe) already works without it.
pub fn attach_parent_console() {}

pub fn hide(_raw: isize) {}

/// Brings an already visible window to the front.
/// Brings our window to the front when Windows wouldn't let us (no hotkey
/// or click of ours came just before): a no-op mouse input makes our
/// process the last to send input, which allows it.
pub fn take_foreground(_raw: isize) {}

pub fn focus(_raw: isize) {}

/// The user's language as a two-letter code (`"ru"`), from their Windows
/// regional format; `"en"` when it can't be told.
pub fn user_language() -> String {
    // `LANG=ru_RU.UTF-8` → `ru`, until macOS's own preferred languages are read.
    std::env::var("LANG")
        .ok()
        .and_then(|lang| lang.get(..2).map(str::to_lowercase))
        .filter(|code| code.chars().all(|ch| ch.is_ascii_lowercase()))
        .unwrap_or_else(|| "en".to_owned())
}

pub fn is_visible(_raw: isize) -> bool {
    false
}
