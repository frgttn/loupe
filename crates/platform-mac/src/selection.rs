//! macOS counterpart of `loupe_platform_win::selection`, with its signatures.
//! A stub for now: nothing here is implemented on macOS yet.

/// What reading the selection found.
pub type Outcome = Result<Option<String>, String>;

/// Reads the text selected in `window`, the window that was active before
/// the launcher. Call it while Loupe is still the foreground app, which
/// lets it give the foreground back. With `copy`, apps that don't share
/// their selection are sent Ctrl+C; `owner` (a window of ours) then holds
/// the clipboard while it's put back.
///
/// The outcome arrives on the returned channel: the text, `None` when
/// nothing is selected, or why it couldn't be read.
pub fn spawn(_window: isize, _owner: isize, _copy: bool) -> async_channel::Receiver<Outcome> {
    async_channel::unbounded().1
}
