//! macOS counterpart of `loupe_platform_win::index_service`, with its signatures.
//! A stub for now: nothing here is implemented on macOS yet.

use crate::fswatch::Change;
use loupe_files::Listed;
use std::{
    io::{self, Read, Write},
    path::PathBuf,
    time::Duration,
};

use crate::UNSUPPORTED;

/// The pipe the service listens on; one per machine.
pub const PIPE: &str = r"\\.\pipe\loupe-index";

/// The service's name in the Service Control Manager.
pub const SERVICE: &str = "LoupeIndex";

/// Bumped when the messages change; the service refuses other versions.
pub const VERSION: u32 = 1;

/// One entry of a root's snapshot. Entries come parents first.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// The index of the parent among this root's entries; [`ROOT`] for an
    /// entry directly in the root.
    pub parent: u32,
    pub listed: Listed,
}

/// The parent of an entry directly in a root.
pub const ROOT: u32 = u32::MAX;

/// The root of an [`FromService::Unavailable`] that refuses the whole
/// connection (this program isn't the one the service was installed for).
pub const ALL_ROOTS: u32 = u32::MAX;

#[derive(Clone, Debug, PartialEq)]
pub enum ToService {
    /// Sent first.
    Hello { version: u32 },
    /// Asks for these roots (numbered from 0 in this order): each one's
    /// snapshot, then their changes. Sent once, after `Hello`: from then on
    /// only the service writes, since a pipe opened for plain blocking I/O
    /// can't read and write at the same time.
    Watch { roots: Vec<PathBuf> },
}

#[derive(Clone, Debug, PartialEq)]
pub enum FromService {
    /// Part of a root's snapshot.
    Entries {
        root: u32,
        entries: Vec<Entry>,
    },
    /// The root's snapshot is complete; changes follow.
    Ready {
        root: u32,
    },
    Changes {
        root: u32,
        changes: Vec<Change>,
    },
    /// The service can't serve this root (not NTFS, no access); the
    /// launcher walks it itself.
    Unavailable {
        root: u32,
        why: String,
    },
    /// The service lost track (the journal was reset): ask again.
    Resync {
        root: u32,
    },
}

/// Whether the service is installed, and running.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    NotInstalled,
    Stopped,
    Running,
}

/// Asks the Service Control Manager; needs no rights.
pub fn state() -> State {
    State::NotInstalled
}

/// `loupe-service.exe` next to this executable, as a build puts it.
pub fn service_exe() -> Option<PathBuf> {
    None
}

/// Installs and starts the service, allowing this executable to connect.
/// Shows the UAC prompt and waits for the elevated installer.
pub fn install() -> Result<(), String> {
    Err(UNSUPPORTED.into())
}

/// Stops and removes the service and its data, after the UAC prompt.
pub fn uninstall() -> Result<(), String> {
    Err(UNSUPPORTED.into())
}

/// A connection to the service.
pub struct Client {
    _private: (),
}

impl Client {
    /// Connects, waiting up to `wait` for a free pipe instance. `None` when
    /// the service isn't running.
    pub fn connect(_wait: Duration) -> Option<Self> {
        None
    }
    /// [`Client::connect`] to another pipe name, for tests.
    pub fn connect_to(_pipe: &str, _wait: Duration) -> Option<Self> {
        None
    }
    pub fn send(&mut self, _message: &ToService) -> io::Result<()> {
        Err(io::Error::other(UNSUPPORTED))
    }
    /// The next message; blocks.
    pub fn recv(&mut self) -> io::Result<FromService> {
        Err(io::Error::other(UNSUPPORTED))
    }
}

pub fn write_frame(_out: &mut impl Write, _payload: &[u8]) -> io::Result<()> {
    Err(io::Error::other(UNSUPPORTED))
}

pub fn read_frame(_input: &mut impl Read) -> io::Result<Vec<u8>> {
    Err(io::Error::other(UNSUPPORTED))
}

pub fn encode_to(_message: &ToService) -> Vec<u8> {
    Vec::new()
}

pub fn decode_to(_bytes: &[u8]) -> io::Result<ToService> {
    Err(io::Error::other(UNSUPPORTED))
}

pub fn encode_from(_message: &FromService) -> Vec<u8> {
    Vec::new()
}

pub fn decode_from(_bytes: &[u8]) -> io::Result<FromService> {
    Err(io::Error::other(UNSUPPORTED))
}
