//! macOS counterpart of `loupe_platform_win::dialog`, with its signatures.
//! A stub for now: nothing here is implemented on macOS yet.

use std::path::PathBuf;

/// A file type the dialog offers: `("JSON Files", "*.json")`.
pub type FileType = (&'static str, &'static str);

/// What the dialog was asked: the window it belongs to (0 for none), its
/// title and the file types it offers, the first one picked at the start.
pub struct Request {
    pub owner: isize,
    pub title: String,
    pub types: Vec<FileType>,
}

/// Asks for a file to open. The channel gets the path, or `None` when the
/// dialog was cancelled.
pub fn open_file(_request: Request) -> async_channel::Receiver<Result<Option<PathBuf>, String>> {
    async_channel::unbounded().1
}

/// Asks where to save a file, starting in `folder` with `name`; the first
/// file type's extension is added when the name has none.
pub fn save_file(
    _request: Request,
    _folder: PathBuf,
    _name: String,
) -> async_channel::Receiver<Result<Option<PathBuf>, String>> {
    async_channel::unbounded().1
}
