//! Decoded icons, whatever drew them: the OS shell (app and file icons) or a
//! site's own favicon.

/// A decoded icon: `width * height` pixels, BGRA, straight (not premultiplied) alpha.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Icon {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
}
