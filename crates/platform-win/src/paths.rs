//! Where Loupe keeps its files on this OS.

use std::{env, path::PathBuf};

/// The folder for the database and caches: `%APPDATA%\loupe`. `None` if
/// Windows doesn't say where the roaming app data is.
///
/// Other OS crates answer with their own convention (on macOS,
/// `~/Library/Application Support/Loupe`).
pub fn data_dir() -> Option<PathBuf> {
    env::var_os("APPDATA").map(|appdata| PathBuf::from(appdata).join("loupe"))
}
