//! macOS counterpart of `loupe_platform_win::fswalk`, with its signatures.
//! A stub for now: nothing here is implemented on macOS yet.

use loupe_files::Listed;
use std::{io, path::Path};

use crate::UNSUPPORTED;

/// The names in folder `dir`, without `.` and `..`. An empty folder lists
/// nothing; one that can't be read is an error.
pub fn list(_dir: &Path) -> io::Result<Vec<Listed>> {
    Err(io::Error::other(UNSUPPORTED))
}

/// What a listing of its folder would say about `path`, or `None` if it is
/// gone: for a change notification, which names a path and nothing more.
pub fn stat(_path: &Path) -> Option<Listed> {
    None
}

/// `dir` with the `\\?\` prefix, so paths past 260 characters list too. A
/// drive keeps its backslash: `\\?\C:` would be the volume itself.
pub fn long_path(dir: &Path) -> String {
    // macOS has no path length limit to work around.
    dir.to_string_lossy().into_owned()
}
