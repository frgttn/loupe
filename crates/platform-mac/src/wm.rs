//! macOS counterpart of `loupe_platform_win::wm`, with its signatures.
//! A stub for now: nothing here is implemented on macOS yet.

use crate::system::Outcome;
use loupe_core::window::{Rect, WindowCommand};

use crate::UNSUPPORTED;

/// A window as the switcher lists it.
#[derive(Clone, Debug)]
pub struct WindowInfo {
    /// Raw HWND.
    pub hwnd: isize,
    pub title: String,
    /// The app's process: for Store apps, the app's own, not the frame host's.
    pub pid: u32,
    pub exe: Option<String>,
    /// "Google Chrome", from the program's version resource.
    pub app: String,
    pub minimized: bool,
}

/// A process of this user's session, as the process list shows it.
#[derive(Clone, Debug)]
pub struct ProcessInfo {
    pub pid: u32,
    /// `chrome.exe`
    pub name: String,
    pub exe: Option<String>,
    /// Private memory, in bytes; 0 when it can't be read.
    pub memory: u64,
    /// How many app windows it has.
    pub windows: usize,
}

/// How layouts place windows from now on: `gap` pixels apart (and from the
/// screen's edges), and whether running a half again cycles through thirds.
pub fn set_layout_options(_gap: i32, _cycle: bool) {}

/// A running app, as root search matches it to an installed one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunningApp {
    /// The process to quit: the one with windows, if any.
    pub pid: u32,
    pub exe: Option<String>,
    /// A packaged app's AUMID (`Microsoft.WindowsCalculator_8wekyb3d8bbwe!App`).
    pub aumid: Option<String>,
}

/// The apps running in this session that have windows, one per program
/// (or packaged app): its process with windows.
pub fn running_apps() -> Vec<RunningApp> {
    Vec::new()
}

/// Every app window except Loupe's, front to back.
pub fn windows() -> Vec<WindowInfo> {
    Vec::new()
}

/// This session's processes (not services, not Loupe), most memory first.
pub fn processes() -> Vec<ProcessInfo> {
    Vec::new()
}

/// Does `command` to window `target` (a raw HWND).
pub fn run(_target: isize, _command: WindowCommand) -> Outcome {
    Err(UNSUPPORTED.into())
}

/// Moves and resizes `target` to the numbers given (see
/// [`window::frame_from`]); Restore puts it back.
pub fn set_frame_from(_target: isize, _numbers: [&String; 4]) -> Outcome {
    Err(UNSUPPORTED.into())
}

/// Where `target` is now: its frame relative to its display's work area.
pub fn position(_target: isize) -> Option<Rect> {
    None
}

/// Asks every window of process `pid` to close.
pub fn quit_app(_pid: u32) -> Outcome {
    Err(UNSUPPORTED.into())
}

/// Ends process `pid` at once.
pub fn force_quit(_pid: u32) -> Outcome {
    Err(UNSUPPORTED.into())
}
