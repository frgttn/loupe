//! macOS counterpart of `loupe_platform_win::run`, with its signatures.
//! A stub for now: nothing here is implemented on macOS yet.

use crate::system::Outcome;

use crate::UNSUPPORTED;

/// A command that runs by name: `gpedit` → `gpedit.msc` at its path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnownCommand {
    /// The file name, as it's shown: `gpedit.msc`.
    pub file: String,
    pub path: String,
}

impl KnownCommand {
    /// The name without its extension, lowercase: what's typed to find it.
    pub fn name(&self) -> String {
        let lower = self.file.to_lowercase();
        match lower.rsplit_once('.') {
            Some((stem, _)) => stem.to_owned(),
            None => lower,
        }
    }
}

/// Every command that runs by name: programs in the PATH folders with the
/// PATHEXT extensions (and `.msc`, `.cpl`), then the App Paths names. The
/// first found of a name wins, as on the command line. Blocking (reads a few
/// folders and registry keys).
pub fn known_commands() -> Vec<KnownCommand> {
    Vec::new()
}

/// Starts `line` as the Run dialog would: environment variables expanded,
/// the program (or file, folder, URL) split from its arguments; elevated
/// after the UAC prompt with `admin`. Starts in the user's profile folder.
pub fn start(_line: &str, _admin: bool) -> Outcome {
    Err(UNSUPPORTED.into())
}
