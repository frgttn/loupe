//! Colours shared by the launcher and settings windows (RGBA, `0xRRGGBBAA`).
//!
//! There are two themes, one per [`Appearance`], listed in [`THEMES`].
//!
//! Which one is on follows the [`Mode`]: a fixed appearance, or the
//! system's. The windows read the current palette with [`p`] each time they
//! draw, so a change only needs the windows redrawn.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Appearance {
    Dark,
    Light,
}

impl Appearance {
    pub fn title(self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
        }
    }
}

/// Every colour the windows draw with. Alpha-carrying ones are `0xRRGGBBAA`.
#[derive(Debug)]
pub struct Palette {
    /// Background over GPUI's `Blurred` window, as `0xRRGGBBAA`. GPUI's
    /// accent-policy blur barely blurs on Windows 11, so the tint has to hide
    /// what's behind on its own. (The DWM acrylic system backdrop was tried:
    /// light grey, and the window lost focus and hid itself after a few
    /// seconds. Again with dark mode on: a flat #121214, no blur at all.)
    pub background: u32,
    /// The HUD toast: the same tint, but see-through enough to read as glass.
    pub hud: u32,
    /// The settings sidebar, laid over the window background.
    pub sidebar: u32,
    pub text: u32,
    pub text_muted: u32,
    pub text_faint: u32,
    /// Secondary text in rows and the empty-list message.
    pub text_dim: u32,
    pub text_empty: u32,
    pub hairline: u32,
    /// The launcher window's outline (`0xRRGGBB`, drawn by Windows): marks
    /// its edge over whatever is behind.
    pub window_border: u32,
    /// Raised surfaces (cards, key caps) on the window background.
    pub surface: u32,
    pub surface_hover: u32,
    pub surface_border: u32,
    /// A control under the pointer, over `surface_hover`.
    pub control_hover: u32,
    /// The well of a control that is off, like a switch's track.
    pub control_alt: u32,
    /// The bottom edge of a key cap.
    pub keycap_border: u32,
    /// A result row or tile that is selected, and one under the pointer.
    pub selected: u32,
    pub hover: u32,
    /// Popovers over the window (the action panel): opaque, so the list doesn't show through.
    pub panel: u32,
    pub accent: u32,
    pub accent_soft: u32,
    /// Text and glyphs on an accent or badge background.
    pub on_accent: u32,
    pub success: u32,
    pub error: u32,
    pub scrollbar: u32,
    pub scrollbar_active: u32,
    /// The text field's caret (`0xRRGGBB`), selection and placeholder.
    pub caret: u32,
    pub selection: u32,
    pub placeholder: u32,
}

pub struct Theme {
    pub id: &'static str,
    pub name: &'static str,
    pub appearance: Appearance,
    pub palette: Palette,
}

pub const SPOTLIGHT_DARK: &str = "spotlight-dark";
pub const SPOTLIGHT_LIGHT: &str = "spotlight-light";

/// The built-in themes: dark first, then light.
///
/// They follow WinUI 3's own theme resources (the Windows 11
/// Fluent tokens), with the default system accent #0078D4. The token each
/// value comes from is named beside it; where the launcher needs a step
/// WinUI doesn't have, the nearest token is used and the comment says so.
pub static THEMES: &[Theme] = &[
    Theme {
        id: SPOTLIGHT_DARK,
        name: "Spotlight Dark",
        appearance: Appearance::Dark,
        palette: Palette {
            // SolidBackgroundFillColorBase / Mica base.
            background: 0x202020f7,
            // AcrylicBackgroundFillColorDefault tint.
            hud: 0x2c2c2cd2,
            // Darker than the Mica base, like NavigationView's pane.
            sidebar: 0x0000001f,
            // TextFillColorPrimary / Secondary / Tertiary / Disabled.
            text: 0xffffffff,
            text_muted: 0xffffffc5,
            text_faint: 0xffffff87,
            text_dim: 0xffffff87,
            text_empty: 0xffffff5d,
            // DividerStrokeColorDefault.
            hairline: 0xffffff15,
            window_border: 0x484848,
            // CardBackgroundFillColorDefault, ControlFillColorSecondary,
            // CardStrokeColorDefault.
            surface: 0xffffff0d,
            surface_hover: 0xffffff15,
            surface_border: 0x00000019,
            // A step above ControlFillColorSecondary.
            control_hover: 0xffffff1f,
            // ControlAltFillColorSecondary.
            control_alt: 0x00000019,
            // ControlStrokeColorDefault, the lower edge of a control.
            keycap_border: 0xffffff12,
            // SubtleFillColorSecondary for hover; selection one step up
            // (WinUI marks it with the accent pill instead).
            selected: 0xffffff15,
            hover: 0xffffff0f,
            // SolidBackgroundFillColorQuarternary, a flyout.
            panel: 0x2c2c2cff,
            // AccentFillColorDefault (SystemAccentColorLight2) and
            // TextOnAccentFillColorPrimary.
            accent: 0x60cdffff,
            accent_soft: 0x60cdff2e,
            on_accent: 0x000000ff,
            // SystemFillColorSuccess / Critical.
            success: 0x6ccb5fff,
            error: 0xff99a4ff,
            // ControlStrongFillColorDisabled / Default.
            scrollbar: 0xffffff5d,
            scrollbar_active: 0xffffff8b,
            // TextBox: the caret is the text colour; the selection is
            // SystemAccentColor.
            caret: 0xffffff,
            selection: 0x0078d4b3,
            // TextFillColorTertiary, so it reads as a hint next to typed text.
            placeholder: 0xffffff87,
        },
    },
    Theme {
        id: SPOTLIGHT_LIGHT,
        name: "Spotlight Light",
        appearance: Appearance::Light,
        palette: Palette {
            // SolidBackgroundFillColorBase / Mica base.
            background: 0xf3f3f3f7,
            // AcrylicBackgroundFillColorDefault tint.
            hud: 0xfcfcfce6,
            sidebar: 0x0000000a,
            // TextFillColorPrimary / Secondary / Tertiary / Disabled.
            text: 0x000000e4,
            text_muted: 0x0000009e,
            text_faint: 0x00000072,
            text_dim: 0x00000072,
            text_empty: 0x0000005c,
            // DividerStrokeColorDefault.
            hairline: 0x0000000f,
            window_border: 0xc6c6c6,
            // CardBackgroundFillColorDefault, ControlFillColorSecondary,
            // CardStrokeColorDefault.
            surface: 0xffffffb3,
            surface_hover: 0xf9f9f980,
            surface_border: 0x0000000f,
            // SubtleFillColorSecondary over a card.
            control_hover: 0x0000000d,
            // ControlAltFillColorSecondary.
            control_alt: 0x00000006,
            // ControlStrokeColorSecondary, the lower edge of a control.
            keycap_border: 0x00000029,
            // SubtleFillColorSecondary for hover; selection one step up.
            selected: 0x0000000f,
            hover: 0x00000009,
            // SolidBackgroundFillColorQuarternary, a flyout.
            panel: 0xffffffff,
            // AccentFillColorDefault (SystemAccentColorDark1) and
            // TextOnAccentFillColorPrimary.
            accent: 0x005fb8ff,
            accent_soft: 0x005fb826,
            on_accent: 0xffffffff,
            // SystemFillColorSuccess / Critical.
            success: 0x0f7b0fff,
            error: 0xc42b1cff,
            // ControlStrongFillColorDisabled / Default.
            scrollbar: 0x0000005c,
            scrollbar_active: 0x00000072,
            // TextBox: the caret is the text colour; the selection is
            // SystemAccentColor, lighter so dark text stays readable on it.
            caret: 0x000000,
            selection: 0x0078d466,
            placeholder: 0x00000072,
        },
    },
];

/// Whether the theme follows the system's light/dark setting or is fixed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    System,
    Light,
    Dark,
}

impl Mode {
    pub const ALL: [Self; 3] = [Self::System, Self::Light, Self::Dark];

    pub fn title(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }

    pub fn to_setting(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub fn from_setting(text: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|mode| mode.to_setting() == text.trim())
    }

    /// Which appearance is on, given the system's.
    pub fn appearance(self, system: Appearance) -> Appearance {
        match self {
            Self::System => system,
            Self::Light => Appearance::Light,
            Self::Dark => Appearance::Dark,
        }
    }
}

pub const MODE_SETTING: &str = "theme_mode";

/// The theme of `appearance`.
pub fn theme_of(appearance: Appearance) -> &'static Theme {
    THEMES
        .iter()
        .find(|theme| theme.appearance == appearance)
        .unwrap_or(&THEMES[0])
}

/// The mode in use, as [`Mode`]'s index.
static MODE: AtomicUsize = AtomicUsize::new(0);
static SYSTEM_LIGHT: AtomicBool = AtomicBool::new(false);
/// Index into [`THEMES`] of the theme in use; the first is dark.
static CURRENT: AtomicUsize = AtomicUsize::new(0);

/// The palette in use.
pub fn p() -> &'static Palette {
    &current().palette
}

pub fn current() -> &'static Theme {
    &THEMES[CURRENT.load(Ordering::Relaxed).min(THEMES.len() - 1)]
}

pub fn mode() -> Mode {
    Mode::ALL[MODE.load(Ordering::Relaxed).min(Mode::ALL.len() - 1)]
}

pub fn system_appearance() -> Appearance {
    if SYSTEM_LIGHT.load(Ordering::Relaxed) {
        Appearance::Light
    } else {
        Appearance::Dark
    }
}

pub fn appearance_of(appearance: gpui::WindowAppearance) -> Appearance {
    match appearance {
        gpui::WindowAppearance::Light | gpui::WindowAppearance::VibrantLight => Appearance::Light,
        gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark => Appearance::Dark,
    }
}

/// Makes `mode` the one in use. Returns whether the palette changed, which
/// is when the windows need redrawing.
pub fn set_mode(mode: Mode) -> bool {
    let index = Mode::ALL.iter().position(|m| *m == mode).unwrap_or(0);
    MODE.store(index, Ordering::Relaxed);
    resolve()
}

/// Tells the theme what the system's appearance is now. Returns whether the
/// palette changed.
pub fn set_system(appearance: Appearance) -> bool {
    SYSTEM_LIGHT.store(appearance == Appearance::Light, Ordering::Relaxed);
    resolve()
}

fn resolve() -> bool {
    let appearance = mode().appearance(system_appearance());
    let index = THEMES
        .iter()
        .position(|theme| theme.appearance == appearance)
        .unwrap_or(0);
    CURRENT.swap(index, Ordering::Relaxed) != index
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_appearance_has_its_theme() {
        for appearance in [Appearance::Dark, Appearance::Light] {
            assert_eq!(theme_of(appearance).appearance, appearance);
        }
        assert_eq!(theme_of(Appearance::Dark).id, SPOTLIGHT_DARK);
        assert_eq!(theme_of(Appearance::Light).id, SPOTLIGHT_LIGHT);
    }

    #[test]
    fn mode_round_trips_and_picks_the_appearance() {
        assert_eq!(Mode::from_setting("system"), Some(Mode::System));
        assert_eq!(Mode::from_setting(" dark "), Some(Mode::Dark));
        assert_eq!(Mode::from_setting("blue"), None);
        assert_eq!(
            Mode::System.appearance(Appearance::Light),
            Appearance::Light
        );
        assert_eq!(Mode::Dark.appearance(Appearance::Light), Appearance::Dark);
    }
}
