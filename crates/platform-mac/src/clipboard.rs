//! macOS counterpart of `loupe_platform_win::clipboard`, with its signatures.
//! A stub for now: nothing here is implemented on macOS yet.

use image::RgbaImage;
use loupe_core::clip::RichText;

use crate::UNSUPPORTED;

/// What was copied.
#[derive(Clone, Debug, PartialEq)]
pub enum Content {
    Text(String),
    /// Paths of copied files and folders.
    Files(Vec<String>),
    Image(Image),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// One change of the clipboard the history should keep.
#[derive(Clone, Debug)]
pub struct Copied {
    pub content: Content,
    /// The formatting copied with text; empty for anything else.
    pub rich: RichText,
    /// The app that copied it: its display name and program path. `None`
    /// when it can't be told, and when Loupe itself wrote it.
    pub app: Option<(String, String)>,
}

/// Starts watching the clipboard: everything copied from now on that isn't
/// marked private arrives on `copied`. The thread ends when `copied` closes.
pub fn listen(_copied: async_channel::Sender<Copied>) -> Result<(), String> {
    Err(UNSUPPORTED.into())
}

/// The image on the clipboard now, or the picture file when one file was
/// copied in Explorer. `Ok(None)`: there's no image. Blocking: decodes it.
pub fn image(_owner: isize) -> Result<Option<RgbaImage>, String> {
    Err(UNSUPPORTED.into())
}

/// Puts `content` on the clipboard, as copying it in the first place did.
/// `owner` is a window of ours, which Windows records as the clipboard's owner.
pub fn write(_owner: isize, _content: &Content) -> Result<(), String> {
    Err(UNSUPPORTED.into())
}

/// Puts `text` on the clipboard with the formatting it was copied with, so
/// apps that understand it paste it formatted and the others get the text.
pub fn write_rich(_owner: isize, _text: &str, _rich: &RichText) -> Result<(), String> {
    Err(UNSUPPORTED.into())
}

/// The window that has the keyboard now, as a raw HWND (0 for none).
pub fn foreground_window() -> isize {
    0
}

/// Activates `window` and presses Ctrl+V in it, to paste what was just put on
/// the clipboard, then Left `caret_back` times (to put the caret inside what
/// was pasted). Call it while Loupe is still the foreground app: only
/// then may it hand the foreground to another window. Returns `false` (and
/// does nothing) when the window is gone.
pub fn paste_into(_window: isize, _caret_back: usize) -> bool {
    false
}

/// Asks Windows to activate `window`. Only allowed while Loupe is the
/// foreground app. `false` when the window is gone.
pub fn bring_to_front(_window: isize) -> bool {
    false
}
