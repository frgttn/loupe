//! macOS counterpart of `loupe_platform_win::ocr`, with its signatures.
//! A stub for now: nothing here is implemented on macOS yet.

use image::RgbaImage;

use crate::UNSUPPORTED;

/// The text in `image`, a line per line of text. Empty when there is none.
/// Blocking, and it starts COM in the calling thread: give it a thread of
/// its own.
pub fn read(_image: &RgbaImage) -> Result<String, String> {
    Err(UNSUPPORTED.into())
}
