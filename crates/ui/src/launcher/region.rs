//! Capture Screen Region: the launcher hides, the user drags out a part of
//! the screen (see `loupe_platform_win::region`), and its text is copied
//! (Windows OCR), its QR codes are read, or it's searched with Google Lens
//! in the browser.

use std::{thread, time::Duration};

use gpui::{ClipboardItem, Context, Window};
use loupe_core::item::RegionAction;
use loupe_platform_win::{lens, ocr, region};

use super::Launcher;

/// How long the screen is given to repaint without the launcher (or its
/// HUD) before the still is taken.
const HIDE_SETTLE: Duration = Duration::from_millis(100);
/// How much of the recognised text the HUD shows.
const HUD_PREVIEW: usize = 40;

/// What was done with the picked part of the screen.
enum Done {
    Text(String),
    Codes(Vec<String>),
    Searched,
}

impl Launcher {
    pub(super) fn capture_region(
        &mut self,
        preset: Option<RegionAction>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let was_visible = self.visible || self.hud.is_some();
        self.hud = None;
        self.hide(cx);
        cx.spawn_in(window, async move |this, cx| {
            if was_visible {
                cx.background_executor().timer(HIDE_SETTLE).await;
            }
            let Ok(Some(picked)) = region::pick(preset).recv().await else {
                return;
            };
            // OCR and the browser both want COM: a thread of their own.
            let (tx, rx) = async_channel::bounded(1);
            thread::spawn(move || {
                let done = match picked.action {
                    RegionAction::CopyText => ocr::read(&picked.image).map(Done::Text),
                    RegionAction::ScanQrCode => Ok(Done::Codes(loupe_qr::read(&picked.image))),
                    RegionAction::GoogleLens => {
                        lens::search(&picked.image).map(|()| Done::Searched)
                    }
                };
                let _ = tx.send_blocking(done);
            });
            let Ok(done) = rx.recv().await else {
                return;
            };
            this.update_in(cx, |this, window, cx| {
                let message = match done {
                    Ok(Done::Codes(codes)) => {
                        return this.show_scanned_codes(Ok(codes), "No QR codes there", window, cx);
                    }
                    Ok(Done::Text(text)) if text.trim().is_empty() => {
                        "No text found there".to_owned()
                    }
                    Ok(Done::Text(text)) => {
                        let preview = hud_preview(&text);
                        cx.write_to_clipboard(ClipboardItem::new_string(text));
                        format!("Copied “{preview}”")
                    }
                    Ok(Done::Searched) => return,
                    Err(err) => err,
                };
                this.show_hud(message.into(), cx);
            })
            .ok();
        })
        .detach();
    }
}

/// The text's first line, cut short with an ellipsis.
fn hud_preview(text: &str) -> String {
    let first = text.lines().next().unwrap_or_default().trim();
    let mut preview: String = first.chars().take(HUD_PREVIEW).collect();
    if first.chars().count() > HUD_PREVIEW || text.lines().nth(1).is_some() {
        preview.push('…');
    }
    preview
}

#[cfg(test)]
mod tests {
    use super::hud_preview;

    #[test]
    fn preview_is_the_first_line() {
        assert_eq!(hud_preview("Hello"), "Hello");
        assert_eq!(hud_preview("Hello\nworld"), "Hello…");
        assert_eq!(hud_preview(&"а".repeat(50)), format!("{}…", "а".repeat(40)));
    }
}
