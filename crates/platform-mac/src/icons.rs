//! macOS counterpart of `loupe_platform_win::icons`, with its signatures.
//! A stub for now: nothing here is implemented on macOS yet.

use crate::{Result, UNSUPPORTED};

/// What an icon decodes to; shared with sites' icons (`loupe_platform::favicon`).
pub use loupe_core::icon::Icon;

/// Extracts the icon of a shell item, `size` pixels square. `parsing_name`
/// is a file path, an absolute parsing name (`::{…}`), or
/// `shell:AppsFolder\\<launch id>` for an app. Blocking and COM-heavy: call it
/// from a thread that initialised COM as an STA.
pub fn shell_icon(_parsing_name: &str, _size: u32) -> Result<Icon> {
    Err(UNSUPPORTED.into())
}

/// The thumbnail Explorer shows for a file (a picture, a video's frame, a
/// PDF's first page), fitting in `size` pixels, or its icon when it has none.
/// Blocking; initialises COM on the calling thread.
pub fn thumbnail(_path: &str, _size: u32) -> Result<Icon> {
    Err(UNSUPPORTED.into())
}

/// Extracts icons for `parsing_names` (see [`shell_icon`]) on a dedicated COM
/// thread and sends each one with its index in `parsing_names`. Items whose
/// icon can't be extracted are skipped.
pub fn spawn(
    _parsing_names: Vec<String>,
    _size: u32,
    _icons: async_channel::Sender<(usize, Icon)>,
) {
}
