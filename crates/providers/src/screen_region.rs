//! A part of the screen: drag it out, then copy its text (Windows OCR),
//! read its QR codes or search it with Google Lens. The launcher draws the
//! selection and does the work (see `loupe_platform_win::region`).

use loupe_core::{
    item::{Command, Icon, RegionAction, SearchItem},
    item_id,
    provider::Provider,
};

use crate::glyph;

pub const ID: &str = "screen-region";

pub struct ScreenRegion;

impl Provider for ScreenRegion {
    fn id(&self) -> &'static str {
        ID
    }

    fn name(&self) -> &'static str {
        "Screen Region"
    }

    fn description(&self) -> &'static str {
        "Select a part of the screen, then copy its text (OCR), read its QR codes or search it with Google Lens."
    }

    fn catalog(&self) -> Vec<SearchItem> {
        vec![
            SearchItem::new(item_id(ID, "capture"), "Capture Screen Region")
                .subtitle("Screen Region")
                .keywords(&[
                    "screenshot",
                    "snip",
                    "ocr",
                    "text",
                    "lens",
                    "google",
                    "recognize",
                    "область",
                    "экран",
                    "текст",
                    "распознать",
                ])
                .icon(Icon::Glyph(glyph::CROP))
                .action("Select Region", Command::CaptureRegion(None))
                .action(
                    "Copy Text from Region",
                    Command::CaptureRegion(Some(RegionAction::CopyText)),
                )
                .action(
                    "Scan QR Code in Region",
                    Command::CaptureRegion(Some(RegionAction::ScanQrCode)),
                )
                .action(
                    "Search Region with Google Lens",
                    Command::CaptureRegion(Some(RegionAction::GoogleLens)),
                ),
            SearchItem::new(item_id(ID, "text"), "Copy Text from Screen Region")
                .subtitle("Screen Region")
                .keywords(&[
                    "ocr",
                    "recognize",
                    "read",
                    "screenshot",
                    "распознать",
                    "текст",
                ])
                .icon(Icon::Glyph(glyph::TEXT))
                .action(
                    "Select Region",
                    Command::CaptureRegion(Some(RegionAction::CopyText)),
                ),
            SearchItem::new(item_id(ID, "qr"), "Scan QR Code from Screen Region")
                .subtitle("Screen Region")
                .keywords(&[
                    "qr code",
                    "read",
                    "decode",
                    "barcode",
                    "screenshot",
                    "сканировать",
                ])
                .icon(Icon::Glyph(loupe_qr::GLYPH))
                .action(
                    "Select Region",
                    Command::CaptureRegion(Some(RegionAction::ScanQrCode)),
                ),
            SearchItem::new(item_id(ID, "lens"), "Search Screen Region with Google Lens")
                .subtitle("Screen Region")
                .keywords(&[
                    "google lens",
                    "image search",
                    "reverse",
                    "screenshot",
                    "найти",
                ])
                .icon(Icon::Glyph(glyph::IMAGE))
                .action(
                    "Select Region",
                    Command::CaptureRegion(Some(RegionAction::GoogleLens)),
                ),
        ]
    }
}
