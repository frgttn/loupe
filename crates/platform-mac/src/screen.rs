//! macOS counterpart of `loupe_platform_win::screen`, with its signatures.
//! A stub for now: nothing here is implemented on macOS yet.

use image::RgbaImage;

use crate::UNSUPPORTED;

/// Every display as it is now, in physical pixels, as one image spanning
/// them (areas between displays of different sizes are black). Blocking.
pub fn capture() -> Result<RgbaImage, String> {
    Err(UNSUPPORTED.into())
}
