//! macOS counterpart of `loupe_platform_win::region`, with its signatures.
//! A stub for now: nothing here is implemented on macOS yet.

use image::RgbaImage;
use loupe_core::item::RegionAction;

use crate::UNSUPPORTED;

/// The part of the screen picked, as captured, and what to do with it.
pub struct Picked {
    pub image: RgbaImage,
    pub action: RegionAction,
}

/// Takes a still of every display as they are now, shows it and lets the
/// user pick a part of it. With `preset`, releasing the mouse is the end of
/// it; without, a bar asks. `None` arrives when they cancel (Esc, right
/// click, switching away).
pub fn pick(_preset: Option<RegionAction>) -> async_channel::Receiver<Option<Picked>> {
    async_channel::unbounded().1
}

/// How the picker looks over `screen` with `selection` made and the bar's
/// button `hover` under the pointer, without a window: for checking the
/// design. Measured as on the display at the screen's top left corner.
pub fn preview(
    _screen: RgbaImage,
    _selection: (i32, i32, i32, i32),
    _hover: Option<usize>,
) -> Result<RgbaImage, String> {
    Err(UNSUPPORTED.into())
}
