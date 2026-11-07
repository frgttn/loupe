//! macOS counterpart of `loupe_platform_win::expander`, with its signatures.
//! A stub for now: nothing here is implemented on macOS yet.

use crate::UNSUPPORTED;

/// A snippet to expand when its keyword is typed.
#[derive(Clone, Debug)]
pub struct Keyword {
    pub keyword: String,
    pub name: String,
    /// With the snippets it includes already in it.
    pub text: String,
}

/// A keyword whose snippet has arguments was typed (and deleted) in
/// `window`: the launcher asks for them, then calls [`Service::paste`].
#[derive(Clone, Debug)]
pub struct Ask {
    pub window: isize,
    pub name: String,
    pub text: String,
}

/// Talks to the expander's threads.
#[derive(Clone)]
pub struct Service {
    _private: (),
}

impl Service {
    /// The snippets to expand. None: the hooks go.
    pub fn set_snippets(&self, _snippets: Vec<Keyword>) {}
    /// The programs (paths) where keywords don't expand.
    pub fn set_excluded(&self, _programs: Vec<String>) {}
    /// The form [`Ask`] asked for is up and has the keyboard: the keys held
    /// since the keyword can go to it.
    pub fn form_shown(&self) {}
    /// Pastes `text` (a snippet with its arguments filled in) into `window`
    /// once it's in front, as a typed keyword is, and puts the clipboard back.
    pub fn paste(&self, _window: isize, _text: String) {}
}

/// Starts the expander, with no snippets yet. `owner` (a window of ours, on
/// a thread that handles messages) holds the clipboard while a snippet is
/// pasted. Snippets with arguments go to `asks`.
pub fn spawn(_owner: isize, _asks: async_channel::Sender<Ask>) -> Result<Service, String> {
    Err(UNSUPPORTED.into())
}
