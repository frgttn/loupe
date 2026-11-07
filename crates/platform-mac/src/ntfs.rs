//! NTFS file tables, which only the Windows file index reads. Here only the
//! question file search asks before using the index service.

use std::path::Path;

/// Whether the drive holding `path` is NTFS: never, as far as Loupe on
/// macOS is concerned (it reads no NTFS tables).
pub fn is_ntfs(_path: &Path) -> bool {
    false
}
