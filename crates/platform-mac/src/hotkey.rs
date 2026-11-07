//! macOS counterpart of `loupe_platform_win::hotkey`, with its signatures.
//! A stub for now: nothing here is implemented on macOS yet.

use std::{fmt, time::Instant};

use crate::UNSUPPORTED;

/// A key combination. `vk` is a Windows virtual-key code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hotkey {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub win: bool,
    pub vk: u16,
}

impl Hotkey {
    /// Tried in order when no hotkey is configured or the configured one is taken.
    pub const DEFAULTS: [Hotkey; 2] = [
        Hotkey {
            ctrl: false,
            alt: true,
            shift: false,
            win: false,
            vk: 0x20,
        },
        Hotkey {
            ctrl: true,
            alt: false,
            shift: true,
            win: false,
            vk: 0x20,
        },
    ];
    /// Builds a hotkey from modifier flags and a key name (`"Space"`, `"K"`,
    /// `"F5"`, `"PageUp"`, `"/"`…, case-insensitive). `None` for keys that
    /// can't be a hotkey and for combinations without a modifier, which
    /// would swallow that key everywhere.
    pub fn new(_ctrl: bool, _alt: bool, _shift: bool, _win: bool, _key: &str) -> Option<Self> {
        None
    }
    /// Parses the [`Display`](fmt::Display) form, e.g. `"Ctrl+Shift+Space"`.
    pub fn parse(_text: &str) -> Option<Self> {
        None
    }
}

impl fmt::Display for Hotkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (on, name) in [
            (self.ctrl, "Ctrl"),
            (self.alt, "Alt"),
            (self.shift, "Shift"),
            (self.win, "Win"),
        ] {
            if on {
                write!(f, "{name}+")?;
            }
        }
        match self.vk {
            0x20 => f.write_str("Space"),
            vk => write!(f, "{}", char::from(vk as u8)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SetError {
    /// Another app (or Windows itself) already owns this combination.
    InUse,
    Other(String),
}

impl fmt::Display for SetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SetError::InUse => f.write_str("already used by another app or by macOS"),
            SetError::Other(message) => f.write_str(message),
        }
    }
}

/// What the recorder saw, see [`Service::record`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recorded {
    Hotkey(Hotkey),
    /// Esc with no modifiers.
    Cancel,
    /// A key pressed without Ctrl, Alt, Shift or Win.
    NeedsModifier,
}

/// Handle to the hotkey thread.
#[derive(Clone)]
pub struct Service {
    _private: (),
}

impl Service {
    /// Switches to `hotkey` and ends recording. On failure the previous hotkey
    /// stays registered. Blocks until the hotkey thread answers, which takes
    /// well under a millisecond.
    pub fn set(&self, _hotkey: Hotkey) -> Result<(), SetError> {
        Err(SetError::Other(UNSUPPORTED.into()))
    }
    /// Gives the item `item_id` the hotkey `hotkey`, or takes its hotkey away
    /// (`None`), and ends recording. On failure the item keeps its previous
    /// hotkey. The launcher's own hotkey counts as taken.
    pub fn set_item(&self, _item_id: &str, _hotkey: Option<Hotkey>) -> Result<(), SetError> {
        Err(SetError::Other(UNSUPPORTED.into()))
    }
    /// Starts recording a shortcut: the hotkey is unregistered and a
    /// low-level keyboard hook reports every key combination pressed anywhere
    /// on `recorded`, swallowing it. The hook sees keys before Windows matches
    /// them against hotkeys, so a combination owned by another app arrives too
    /// (and [`Self::set`] then reports it as [`SetError::InUse`]).
    ///
    /// Recording ends with [`Self::set`] or [`Self::resume`]; until then the
    /// user can't type anywhere else, so callers end it on Esc and when their
    /// window loses focus.
    pub fn record(&self, _recorded: async_channel::Sender<Recorded>) {}
    /// Ends recording and re-registers the current hotkey.
    pub fn resume(&self) {}
    /// Makes Win+V run the item `item_id` (its id arrives on `item_presses`,
    /// as for an item hotkey) instead of opening Windows' clipboard history,
    /// or (`None`) gives Win+V back to Windows. Fails while Win+V is the
    /// hotkey of the launcher or of an item.
    pub fn intercept_win_v(&self, _item_id: Option<&str>) -> Result<(), String> {
        Err(UNSUPPORTED.into())
    }
}

/// Starts the hotkey thread. Each press of the launcher hotkey sends the
/// instant `WM_HOTKEY` was dequeued (the start point for the hotkey-to-visible
/// measurement) on `presses`; each press of an item hotkey sends that item's
/// id on `item_presses`.
///
/// Tries `preferred` first, then [`Hotkey::DEFAULTS`]. Returns the service and
/// the hotkey that got registered, or an error naming what was tried (usually
/// because other apps own all of them; the service still works for `set`).
pub fn spawn(
    _preferred: Option<Hotkey>,
    _presses: async_channel::Sender<Instant>,
    _item_presses: async_channel::Sender<String>,
) -> Result<(Service, Result<Hotkey, String>), String> {
    Err(UNSUPPORTED.into())
}
