//! Where Loupe keeps its files on macOS.

use std::path::PathBuf;

/// The folder for the database and caches:
/// `~/Library/Application Support/Loupe`. `None` if there is no home folder.
pub fn data_dir() -> Option<PathBuf> {
    std::env::home_dir().map(|home| home.join("Library/Application Support/Loupe"))
}
