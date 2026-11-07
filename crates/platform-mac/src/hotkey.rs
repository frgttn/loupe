//! System-wide hotkeys on macOS, with `loupe_platform_win::hotkey`'s
//! signatures: the launcher's own, plus any number of item hotkeys that run
//! one command.
//!
//! Hotkeys are named, parsed and shown as on Windows, so settings carry
//! over: `vk` stays a Windows virtual-key code and is turned into the Mac
//! key at the same place on the keyboard when registering. Ctrl is Control,
//! Alt is Option and Win is Command.
//!
//! Registering goes through Carbon's `RegisterEventHotKey`, which needs no
//! permission. Carbon delivers presses through the application's event loop,
//! the one AppKit (and so GPUI) runs on the main thread, so everything here
//! happens on the main thread and the "hotkey thread" of the Windows version
//! doesn't exist. Calls from another thread fail.
//!
//! A combination macOS keeps for itself (Cmd+Space for Spotlight, Ctrl+Space
//! for switching input sources…, see System Settings → Keyboard → Keyboard
//! Shortcuts) registers fine but never arrives, so it counts as taken.
//!
//! Not yet: recording a shortcut ([`Service::record`]) does nothing; that
//! comes with the settings window.

use std::{fmt, time::Instant};

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
    pub fn new(ctrl: bool, alt: bool, shift: bool, win: bool, key: &str) -> Option<Self> {
        let vk = vk_from_name(key)?;
        (ctrl || alt || shift || win).then_some(Self {
            ctrl,
            alt,
            shift,
            win,
            vk,
        })
    }
    /// Parses the [`Display`](fmt::Display) form, e.g. `"Ctrl+Shift+Space"`.
    pub fn parse(text: &str) -> Option<Self> {
        let mut parts: Vec<&str> = text.split('+').map(str::trim).collect();
        // "Ctrl++" names the plus key: splitting leaves two empty parts at the end.
        if text.trim_end().ends_with("++") {
            parts.truncate(parts.len() - 2);
            parts.push("+");
        }
        let key = parts.pop()?;
        let (mut ctrl, mut alt, mut shift, mut win) = (false, false, false, false);
        for part in parts {
            match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => ctrl = true,
                "alt" => alt = true,
                "shift" => shift = true,
                "win" | "windows" | "super" => win = true,
                _ => return None,
            }
        }
        Self::new(ctrl, alt, shift, win, key)
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
        f.write_str(&key_name(self.vk))
    }
}

/// Key names and their Windows virtual-key codes, as `loupe_platform_win`
/// has them.
const NAMED_KEYS: &[(&str, u16)] = &[
    ("Space", 0x20),
    ("Enter", 0x0D),
    ("Tab", 0x09),
    ("Backspace", 0x08),
    ("Delete", 0x2E),
    ("Insert", 0x2D),
    ("Home", 0x24),
    ("End", 0x23),
    ("PageUp", 0x21),
    ("PageDown", 0x22),
    ("Left", 0x25),
    ("Up", 0x26),
    ("Right", 0x27),
    ("Down", 0x28),
    (";", 0xBA),
    ("=", 0xBB),
    (",", 0xBC),
    ("-", 0xBD),
    (".", 0xBE),
    ("/", 0xBF),
    ("`", 0xC0),
    ("[", 0xDB),
    ("\\", 0xDC),
    ("]", 0xDD),
    ("'", 0xDE),
    ("+", 0xBB),
];

fn vk_from_name(name: &str) -> Option<u16> {
    let name = name.trim();
    if let [ch] = name.as_bytes()
        && ch.is_ascii_alphanumeric()
    {
        return Some(ch.to_ascii_uppercase() as u16);
    }
    if let Some(n) = name
        .strip_prefix(['F', 'f'])
        .and_then(|n| n.parse::<u16>().ok())
        && (1..=24).contains(&n)
    {
        return Some(0x70 + n - 1);
    }
    NAMED_KEYS
        .iter()
        .find(|(known, _)| known.eq_ignore_ascii_case(name))
        .map(|&(_, vk)| vk)
}

fn key_name(vk: u16) -> String {
    match vk {
        0x30..=0x39 | 0x41..=0x5A => char::from(vk as u8).to_string(),
        0x70..=0x87 => format!("F{}", vk - 0x70 + 1),
        _ => NAMED_KEYS
            .iter()
            .find(|&&(_, known)| known == vk)
            .map_or_else(|| format!("0x{vk:02X}"), |(name, _)| (*name).to_owned()),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SetError {
    /// Another app (or macOS itself) already owns this combination.
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

/// Handle to the hotkeys. Its state lives on the main thread.
#[derive(Clone)]
pub struct Service {
    _private: (),
}

impl Service {
    /// Switches to `hotkey` and ends recording. On failure the previous hotkey
    /// stays registered.
    pub fn set(&self, hotkey: Hotkey) -> Result<(), SetError> {
        carbon::with_registrar(|registrar| registrar.set(hotkey))
            .unwrap_or_else(|err| Err(SetError::Other(err)))
    }
    /// Gives the item `item_id` the hotkey `hotkey`, or takes its hotkey away
    /// (`None`), and ends recording. On failure the item keeps its previous
    /// hotkey. The launcher's own hotkey counts as taken.
    pub fn set_item(&self, item_id: &str, hotkey: Option<Hotkey>) -> Result<(), SetError> {
        carbon::with_registrar(|registrar| registrar.set_item(item_id, hotkey))
            .unwrap_or_else(|err| Err(SetError::Other(err)))
    }
    /// Starts recording a shortcut. Not yet on macOS: nothing gets recorded,
    /// and the hotkeys stay registered.
    pub fn record(&self, _recorded: async_channel::Sender<Recorded>) {}
    /// Ends recording and re-registers the current hotkey.
    pub fn resume(&self) {}
    /// On Windows, makes Win+V run the item `item_id`. Command+V is paste on
    /// macOS, so taking it is refused; giving it back (`None`) succeeds.
    pub fn intercept_win_v(&self, item_id: Option<&str>) -> Result<(), String> {
        match item_id {
            Some(_) => Err("macOS has no Win+V to take: Command+V is paste".into()),
            None => Ok(()),
        }
    }
}

/// Starts listening for hotkeys; call it on the main thread. Each press of
/// the launcher hotkey sends the instant the press arrived (the start point
/// for the hotkey-to-visible measurement) on `presses`; each press of an item
/// hotkey sends that item's id on `item_presses`.
///
/// Tries `preferred` first, then [`Hotkey::DEFAULTS`]. Returns the service and
/// the hotkey that got registered, or an error naming what was tried (usually
/// because other apps own all of them; the service still works for `set`).
pub fn spawn(
    preferred: Option<Hotkey>,
    presses: async_channel::Sender<Instant>,
    item_presses: async_channel::Sender<String>,
) -> Result<(Service, Result<Hotkey, String>), String> {
    carbon::start(presses, item_presses)?;
    let candidates: Vec<Hotkey> = preferred.into_iter().chain(Hotkey::DEFAULTS).collect();
    let registered = carbon::with_registrar(|registrar| {
        candidates
            .iter()
            .find(|hotkey| registrar.set(**hotkey).is_ok())
            .copied()
    })?
    .ok_or_else(|| {
        let tried: Vec<_> = candidates.iter().map(ToString::to_string).collect();
        format!("could not register any of {tried:?}")
    });
    Ok((Service { _private: () }, registered))
}

/// Carbon modifier flags (`Events.h`).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
const CMD_KEY: u32 = 1 << 8;
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
const SHIFT_KEY: u32 = 1 << 9;
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
const OPTION_KEY: u32 = 1 << 11;
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
const CONTROL_KEY: u32 = 1 << 12;

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
impl Hotkey {
    /// The Carbon modifier flags of this hotkey.
    fn carbon_modifiers(self) -> u32 {
        [
            (self.ctrl, CONTROL_KEY),
            (self.alt, OPTION_KEY),
            (self.shift, SHIFT_KEY),
            (self.win, CMD_KEY),
        ]
        .into_iter()
        .filter(|&(on, _)| on)
        .fold(0, |mods, (_, flag)| mods | flag)
    }
}

/// The Mac virtual key code (`kVK_*`, a place on the keyboard) of the key
/// that sits where the Windows virtual key `vk` does on a US keyboard.
/// `None` for keys a Mac keyboard doesn't have.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn mac_key_code(vk: u16) -> Option<u32> {
    const LETTERS: [u32; 26] = [
        0x00, 0x0B, 0x08, 0x02, 0x0E, 0x03, 0x05, 0x04, 0x22, 0x26, 0x28, 0x25, 0x2E, // A–M
        0x2D, 0x1F, 0x23, 0x0C, 0x0F, 0x01, 0x11, 0x20, 0x09, 0x0D, 0x07, 0x10, 0x06, // N–Z
    ];
    const DIGITS: [u32; 10] = [0x1D, 0x12, 0x13, 0x14, 0x15, 0x17, 0x16, 0x1A, 0x1C, 0x19];
    const F_KEYS: [u32; 20] = [
        0x7A, 0x78, 0x63, 0x76, 0x60, 0x61, 0x62, 0x64, 0x65, 0x6D, // F1–F10
        0x67, 0x6F, 0x69, 0x6B, 0x71, 0x6A, 0x40, 0x4F, 0x50, 0x5A, // F11–F20
    ];
    let code = match vk {
        0x41..=0x5A => LETTERS[usize::from(vk - 0x41)],
        0x30..=0x39 => DIGITS[usize::from(vk - 0x30)],
        0x70..=0x83 => F_KEYS[usize::from(vk - 0x70)],
        0x20 => 0x31, // Space
        0x0D => 0x24, // Return
        0x09 => 0x30, // Tab
        0x08 => 0x33, // Backspace (Delete on a Mac)
        0x2E => 0x75, // Delete (Forward Delete)
        0x2D => 0x72, // Insert (Help)
        0x24 => 0x73, // Home
        0x23 => 0x77, // End
        0x21 => 0x74, // PageUp
        0x22 => 0x79, // PageDown
        0x25 => 0x7B, // Left
        0x26 => 0x7E, // Up
        0x27 => 0x7C, // Right
        0x28 => 0x7D, // Down
        0xBA => 0x29, // ;
        0xBB => 0x18, // =
        0xBC => 0x2B, // ,
        0xBD => 0x1B, // -
        0xBE => 0x2F, // .
        0xBF => 0x2C, // /
        0xC0 => 0x32, // `
        0xDB => 0x21, // [
        0xDC => 0x2A, // \
        0xDD => 0x1E, // ]
        0xDE => 0x27, // '
        _ => return None,
    };
    Some(code)
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
/// What a registered hotkey runs.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Target {
    Launcher,
    Item(String),
}

#[cfg(target_os = "macos")]
use carbon_impl as carbon;
#[cfg(not(target_os = "macos"))]
use no_carbon as carbon;

/// Without Carbon (building on Windows with `--cfg loupe_mac_stub`): the
/// service starts, and no hotkey ever registers.
#[cfg(not(target_os = "macos"))]
mod no_carbon {
    use std::time::Instant;

    use super::{Hotkey, SetError};
    use crate::UNSUPPORTED;

    pub struct Registrar;

    impl Registrar {
        pub fn set(&mut self, _hotkey: Hotkey) -> Result<(), SetError> {
            Err(SetError::Other(UNSUPPORTED.into()))
        }
        pub fn set_item(&mut self, _item: &str, _hotkey: Option<Hotkey>) -> Result<(), SetError> {
            Err(SetError::Other(UNSUPPORTED.into()))
        }
    }

    pub fn start(
        _presses: async_channel::Sender<Instant>,
        _item_presses: async_channel::Sender<String>,
    ) -> Result<(), String> {
        Ok(())
    }

    pub fn with_registrar<T>(f: impl FnOnce(&mut Registrar) -> T) -> Result<T, String> {
        Ok(f(&mut Registrar))
    }
}

#[cfg(target_os = "macos")]
mod carbon_impl {
    use std::{
        cell::RefCell,
        collections::HashMap,
        ffi::{c_char, c_void},
        ptr,
        time::Instant,
    };

    use super::{Hotkey, SetError, Target, mac_key_code};

    type OSStatus = i32;
    type EventRef = *mut c_void;
    type EventHandlerCallRef = *mut c_void;
    type EventHandlerRef = *mut c_void;
    type EventTargetRef = *mut c_void;
    type EventHotKeyRef = *mut c_void;
    type EventHandlerUPP = extern "C" fn(EventHandlerCallRef, EventRef, *mut c_void) -> OSStatus;

    #[repr(C)]
    struct EventTypeSpec {
        event_class: u32,
        event_kind: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct EventHotKeyID {
        signature: u32,
        id: u32,
    }

    const fn four_cc(code: &[u8; 4]) -> u32 {
        u32::from_be_bytes(*code)
    }

    const NO_ERR: OSStatus = 0;
    const EVENT_NOT_HANDLED_ERR: OSStatus = -9874;
    const EVENT_HOT_KEY_EXISTS_ERR: OSStatus = -9878;
    const K_EVENT_CLASS_KEYBOARD: u32 = four_cc(b"keyb");
    const K_EVENT_HOT_KEY_PRESSED: u32 = 5;
    const K_EVENT_PARAM_DIRECT_OBJECT: u32 = four_cc(b"----");
    const TYPE_EVENT_HOT_KEY_ID: u32 = four_cc(b"hkid");
    /// Fails, rather than sharing, when another app registered the
    /// combination exclusively.
    const K_EVENT_HOT_KEY_EXCLUSIVE: u32 = 1;
    /// Ours, to tell our hotkeys from anyone else's.
    const SIGNATURE: u32 = four_cc(b"LOUP");

    #[link(name = "Carbon", kind = "framework")]
    unsafe extern "C" {
        fn GetApplicationEventTarget() -> EventTargetRef;
        fn InstallEventHandler(
            target: EventTargetRef,
            handler: EventHandlerUPP,
            num_types: u32,
            list: *const EventTypeSpec,
            user_data: *mut c_void,
            out_ref: *mut EventHandlerRef,
        ) -> OSStatus;
        fn RegisterEventHotKey(
            key_code: u32,
            modifiers: u32,
            id: EventHotKeyID,
            target: EventTargetRef,
            options: u32,
            out_ref: *mut EventHotKeyRef,
        ) -> OSStatus;
        fn UnregisterEventHotKey(hot_key: EventHotKeyRef) -> OSStatus;
        fn GetEventParameter(
            event: EventRef,
            name: u32,
            desired_type: u32,
            actual_type: *mut u32,
            buffer_size: usize,
            actual_size: *mut usize,
            data: *mut c_void,
        ) -> OSStatus;
        fn CopySymbolicHotKeys(hot_keys: *mut CFArrayRef) -> OSStatus;
    }

    type CFTypeRef = *const c_void;
    type CFArrayRef = *const c_void;
    type CFDictionaryRef = *const c_void;
    type CFStringRef = *const c_void;
    type CFNumberRef = *const c_void;
    type CFBooleanRef = *const c_void;
    const K_CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;
    const K_CF_NUMBER_SINT64_TYPE: isize = 4;

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFArrayGetCount(array: CFArrayRef) -> isize;
        fn CFArrayGetValueAtIndex(array: CFArrayRef, index: isize) -> CFTypeRef;
        fn CFDictionaryGetValue(dict: CFDictionaryRef, key: CFTypeRef) -> CFTypeRef;
        fn CFNumberGetValue(number: CFNumberRef, kind: isize, value: *mut c_void) -> bool;
        fn CFBooleanGetValue(boolean: CFBooleanRef) -> bool;
        fn CFStringCreateWithCString(
            alloc: *const c_void,
            text: *const c_char,
            encoding: u32,
        ) -> CFStringRef;
        fn CFRelease(object: CFTypeRef);
    }

    fn is_main_thread() -> bool {
        unsafe extern "C" {
            fn pthread_main_np() -> i32;
        }
        unsafe { pthread_main_np() != 0 }
    }

    /// State owned by the main thread.
    pub struct Registrar {
        presses: async_channel::Sender<Instant>,
        item_presses: async_channel::Sender<String>,
        /// The launcher hotkey and how Carbon holds it.
        launcher: Option<(Hotkey, EventHotKeyRef)>,
        /// Item id → (hotkey, how Carbon holds it).
        items: HashMap<String, (Hotkey, EventHotKeyRef)>,
        /// What each hotkey id runs. Ids are never reused, so a press that
        /// was already queued when its hotkey went runs nothing.
        targets: HashMap<u32, Target>,
        last_id: u32,
    }

    thread_local! {
        static REGISTRAR: RefCell<Option<Registrar>> = const { RefCell::new(None) };
    }

    /// Installs the press handler and sets up the state. Main thread only.
    pub fn start(
        presses: async_channel::Sender<Instant>,
        item_presses: async_channel::Sender<String>,
    ) -> Result<(), String> {
        if !is_main_thread() {
            return Err("hotkeys must be started on the main thread".into());
        }
        if REGISTRAR.with(|registrar| registrar.borrow().is_some()) {
            return Err("hotkeys are already started".into());
        }
        let spec = EventTypeSpec {
            event_class: K_EVENT_CLASS_KEYBOARD,
            event_kind: K_EVENT_HOT_KEY_PRESSED,
        };
        let mut handler: EventHandlerRef = ptr::null_mut();
        let status = unsafe {
            InstallEventHandler(
                GetApplicationEventTarget(),
                on_hot_key,
                1,
                &spec,
                ptr::null_mut(),
                &mut handler,
            )
        };
        if status != NO_ERR {
            return Err(format!("could not listen for hotkeys (error {status})"));
        }
        REGISTRAR.with(|registrar| {
            *registrar.borrow_mut() = Some(Registrar {
                presses,
                item_presses,
                launcher: None,
                items: HashMap::new(),
                targets: HashMap::new(),
                last_id: 0,
            })
        });
        Ok(())
    }

    /// Runs `f` with the state. Fails off the main thread and before `start`.
    pub fn with_registrar<T>(f: impl FnOnce(&mut Registrar) -> T) -> Result<T, String> {
        if !is_main_thread() {
            return Err("hotkeys can only be changed on the main thread".into());
        }
        REGISTRAR.with(|registrar| match registrar.borrow_mut().as_mut() {
            Some(registrar) => Ok(f(registrar)),
            None => Err("hotkeys aren't started".into()),
        })
    }

    /// Carbon calls this on the main thread for every hotkey press in the app.
    extern "C" fn on_hot_key(
        _next: EventHandlerCallRef,
        event: EventRef,
        _user_data: *mut c_void,
    ) -> OSStatus {
        let pressed_at = Instant::now();
        let mut id = EventHotKeyID::default();
        let status = unsafe {
            GetEventParameter(
                event,
                K_EVENT_PARAM_DIRECT_OBJECT,
                TYPE_EVENT_HOT_KEY_ID,
                ptr::null_mut(),
                size_of::<EventHotKeyID>(),
                ptr::null_mut(),
                (&raw mut id).cast(),
            )
        };
        if status != NO_ERR || id.signature != SIGNATURE {
            return EVENT_NOT_HANDLED_ERR;
        }
        // `try_borrow`: a press can't arrive while the state is borrowed
        // (both run on the main thread, and registering doesn't run the
        // event loop), but if it ever did, dropping it beats panicking.
        REGISTRAR.with(|registrar| {
            let Ok(registrar) = registrar.try_borrow() else {
                return;
            };
            let Some(registrar) = registrar.as_ref() else {
                return;
            };
            match registrar.targets.get(&id.id) {
                Some(Target::Launcher) => {
                    let _ = registrar.presses.try_send(pressed_at);
                }
                Some(Target::Item(item)) => {
                    let _ = registrar.item_presses.try_send(item.clone());
                }
                None => {}
            }
        });
        NO_ERR
    }

    impl Registrar {
        /// Registers `hotkey` for `target` and returns Carbon's handle.
        fn register(&mut self, hotkey: Hotkey, target: Target) -> Result<EventHotKeyRef, SetError> {
            let key_code = mac_key_code(hotkey.vk)
                .ok_or_else(|| SetError::Other(format!("a Mac keyboard has no {hotkey} key")))?;
            let modifiers = hotkey.carbon_modifiers();
            if reserved_by_macos(key_code, modifiers) {
                return Err(SetError::InUse);
            }
            let id = self.last_id + 1;
            let mut hot_key: EventHotKeyRef = ptr::null_mut();
            let status = unsafe {
                RegisterEventHotKey(
                    key_code,
                    modifiers,
                    EventHotKeyID {
                        signature: SIGNATURE,
                        id,
                    },
                    GetApplicationEventTarget(),
                    K_EVENT_HOT_KEY_EXCLUSIVE,
                    &mut hot_key,
                )
            };
            match status {
                NO_ERR => {
                    self.last_id = id;
                    self.targets.insert(id, target);
                    Ok(hot_key)
                }
                EVENT_HOT_KEY_EXISTS_ERR => Err(SetError::InUse),
                status => Err(SetError::Other(format!(
                    "macOS refused {hotkey} (error {status})"
                ))),
            }
        }

        fn unregister(&mut self, hot_key: EventHotKeyRef) {
            unsafe {
                UnregisterEventHotKey(hot_key);
            }
        }

        /// Whether an item already holds `hotkey`.
        fn held_by_item(&self, hotkey: Hotkey) -> bool {
            self.items.values().any(|&(held, _)| held == hotkey)
        }

        /// Registers `hotkey` as the launcher hotkey, then lets go of the old one.
        pub fn set(&mut self, hotkey: Hotkey) -> Result<(), SetError> {
            if self.launcher.is_some_and(|(current, _)| current == hotkey) {
                return Ok(());
            }
            if self.held_by_item(hotkey) {
                return Err(SetError::InUse);
            }
            let hot_key = self.register(hotkey, Target::Launcher)?;
            if let Some((_, old)) = self.launcher.replace((hotkey, hot_key)) {
                self.unregister(old);
            }
            Ok(())
        }

        pub fn set_item(&mut self, item: &str, hotkey: Option<Hotkey>) -> Result<(), SetError> {
            let previous = self.items.get(item).copied();
            let Some(hotkey) = hotkey else {
                if let Some((_, old)) = self.items.remove(item) {
                    self.unregister(old);
                }
                return Ok(());
            };
            if previous.is_some_and(|(old, _)| old == hotkey) {
                return Ok(());
            }
            if self.launcher.is_some_and(|(current, _)| current == hotkey)
                || self.held_by_item(hotkey)
            {
                return Err(SetError::InUse);
            }
            let hot_key = self.register(hotkey, Target::Item(item.to_owned()))?;
            if let Some((_, old)) = self.items.insert(item.to_owned(), (hotkey, hot_key)) {
                self.unregister(old);
            }
            Ok(())
        }
    }

    /// Whether macOS keeps the key `key_code` with `modifiers` for one of
    /// its own shortcuts that is turned on (System Settings → Keyboard →
    /// Keyboard Shortcuts). Such a hotkey would register and never arrive.
    fn reserved_by_macos(key_code: u32, modifiers: u32) -> bool {
        const MODIFIER_MASK: i64 =
            (super::CMD_KEY | super::SHIFT_KEY | super::OPTION_KEY | super::CONTROL_KEY) as i64;
        let mut shortcuts: CFArrayRef = ptr::null();
        if unsafe { CopySymbolicHotKeys(&mut shortcuts) } != NO_ERR || shortcuts.is_null() {
            return false;
        }
        let key = |name: &[u8]| unsafe {
            CFStringCreateWithCString(ptr::null(), name.as_ptr().cast(), K_CF_STRING_ENCODING_UTF8)
        };
        let (code_key, modifiers_key, enabled_key) = (
            key(b"kHISymbolicHotKeyCode\0"),
            key(b"kHISymbolicHotKeyModifiers\0"),
            key(b"kHISymbolicHotKeyEnabled\0"),
        );
        let number = |dict: CFDictionaryRef, key: CFStringRef| -> Option<i64> {
            let value = unsafe { CFDictionaryGetValue(dict, key) };
            let mut out = 0i64;
            (!value.is_null()
                && unsafe {
                    CFNumberGetValue(value, K_CF_NUMBER_SINT64_TYPE, (&raw mut out).cast())
                })
            .then_some(out)
        };
        let reserved = (0..unsafe { CFArrayGetCount(shortcuts) }).any(|index| {
            let dict = unsafe { CFArrayGetValueAtIndex(shortcuts, index) };
            let enabled = unsafe { CFDictionaryGetValue(dict, enabled_key) };
            !enabled.is_null()
                && unsafe { CFBooleanGetValue(enabled) }
                && number(dict, code_key) == Some(i64::from(key_code))
                && number(dict, modifiers_key)
                    .is_some_and(|mods| mods & MODIFIER_MASK == i64::from(modifiers))
        });
        unsafe {
            for object in [code_key, modifiers_key, enabled_key, shortcuts] {
                if !object.is_null() {
                    CFRelease(object);
                }
            }
        }
        reserved
    }

    #[cfg(test)]
    mod tests {
        /// Depends on this Mac's settings, so run it by hand.
        #[test]
        #[ignore = "reads this Mac's keyboard shortcuts"]
        fn spotlight_keeps_command_space() {
            assert!(super::reserved_by_macos(0x31, super::super::CMD_KEY));
            assert!(!super::reserved_by_macos(0x31, super::super::OPTION_KEY));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_map_to_the_same_place_on_a_mac_keyboard() {
        let code = |text: &str| mac_key_code(Hotkey::parse(text).unwrap().vk);
        assert_eq!(code("Alt+Space"), Some(0x31));
        assert_eq!(code("Ctrl+A"), Some(0x00));
        assert_eq!(code("Ctrl+K"), Some(0x28));
        assert_eq!(code("Ctrl+Z"), Some(0x06));
        assert_eq!(code("Ctrl+0"), Some(0x1D));
        assert_eq!(code("Ctrl+5"), Some(0x17));
        assert_eq!(code("Ctrl+F1"), Some(0x7A));
        assert_eq!(code("Ctrl+F20"), Some(0x5A));
        assert_eq!(code("Ctrl+F21"), None);
        assert_eq!(code("Ctrl+/"), Some(0x2C));
        assert_eq!(code("Ctrl+PageDown"), Some(0x79));
    }

    #[test]
    fn modifiers_map_to_carbon_flags() {
        let mods = |text: &str| Hotkey::parse(text).unwrap().carbon_modifiers();
        assert_eq!(mods("Alt+Space"), OPTION_KEY);
        assert_eq!(mods("Win+Shift+K"), CMD_KEY | SHIFT_KEY);
        assert_eq!(
            mods("Ctrl+Alt+Shift+Win+K"),
            CMD_KEY | SHIFT_KEY | OPTION_KEY | CONTROL_KEY
        );
    }
}
