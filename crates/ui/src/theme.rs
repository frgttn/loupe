//! Colours shared by the launcher and settings windows (RGBA, `0xRRGGBBAA`).
//!
//! A theme is a [`Palette`] with an id, a name and an [`Appearance`] (dark or
//! light), listed in [`THEMES`]. Adding a theme is one more entry there: it
//! then shows up in Settings by itself, for its appearance.
//!
//! Which theme is on follows the [`Choice`]: a fixed appearance, or the
//! system's, with one theme preferred per appearance. The windows read the
//! current palette with [`p`] each time they draw, so a change only needs
//! the windows redrawn.

use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

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
    /// Stored in settings; never changes once released.
    pub id: &'static str,
    pub name: &'static str,
    pub appearance: Appearance,
    pub palette: Palette,
}

/// The few colours a Catppuccin flavour is built from, as `0xRRGGBB`.
struct Flavour {
    base: u32,
    mantle: u32,
    /// Popovers: opaque, over `base`.
    panel: u32,
    text: u32,
    border: u32,
    accent: u32,
    on_accent: u32,
    success: u32,
    error: u32,
}

/// `rgb` with `alpha`, as `0xRRGGBBAA`.
const fn with_alpha(rgb: u32, alpha: u32) -> u32 {
    rgb << 8 | alpha
}

/// A palette from a Catppuccin flavour; its overlays are the text colour at
/// the same strengths as Spotlight's own themes use, dark or light.
const fn catppuccin(f: Flavour, light: bool) -> Palette {
    // Overlay strengths differ: dark text on light needs more weight.
    let (muted, faint, dim, empty) = if light {
        (0xa0, 0x8a, 0x90, 0x70)
    } else {
        (0xa8, 0x8a, 0x70, 0x60)
    };
    let (surface, surface_hover, surface_border, control_hover) = if light {
        (0x08, 0x12, 0x18, 0x1c)
    } else {
        (0x0a, 0x1a, 0x14, 0x26)
    };
    let (selected, hover, hairline) = if light {
        (0x1a, 0x0d, 0x1c)
    } else {
        (0x1f, 0x0f, 0x24)
    };
    Palette {
        background: with_alpha(f.base, 0xf7),
        hud: with_alpha(f.base, if light { 0xe6 } else { 0xd2 }),
        sidebar: with_alpha(f.mantle, 0xb0),
        text: with_alpha(f.text, 0xff),
        text_muted: with_alpha(f.text, muted),
        text_faint: with_alpha(f.text, faint),
        text_dim: with_alpha(f.text, dim),
        text_empty: with_alpha(f.text, empty),
        hairline: with_alpha(f.text, hairline),
        window_border: f.border,
        surface: with_alpha(f.text, surface),
        surface_hover: with_alpha(f.text, surface_hover),
        surface_border: with_alpha(f.text, surface_border),
        control_hover: with_alpha(f.text, control_hover),
        control_alt: if light { 0x00000006 } else { 0x00000019 },
        keycap_border: if light { 0x00000030 } else { 0x00000066 },
        selected: with_alpha(f.text, selected),
        hover: with_alpha(f.text, hover),
        panel: with_alpha(f.panel, 0xff),
        accent: with_alpha(f.accent, 0xff),
        accent_soft: with_alpha(f.accent, 0x2e),
        on_accent: with_alpha(f.on_accent, 0xff),
        success: with_alpha(f.success, 0xff),
        error: with_alpha(f.error, 0xff),
        scrollbar: with_alpha(f.text, 0x33),
        scrollbar_active: with_alpha(f.text, 0x66),
        caret: f.accent,
        selection: with_alpha(f.accent, if light { 0x44 } else { 0x55 }),
        placeholder: with_alpha(f.text, if light { 0x73 } else { 0x80 }),
    }
}

pub const SPOTLIGHT_DARK: &str = "spotlight-dark";
pub const SPOTLIGHT_LIGHT: &str = "spotlight-light";

/// The built-in themes. The first of each appearance is its default; the
/// rest are listed in this order.
///
/// The two defaults follow WinUI 3's own theme resources (the Windows 11
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
    Theme {
        id: "catppuccin-mocha",
        name: "Catppuccin Mocha",
        appearance: Appearance::Dark,
        palette: catppuccin(
            Flavour {
                base: 0x1e1e2e,
                mantle: 0x181825,
                panel: 0x313244,
                text: 0xcdd6f4,
                border: 0x45475a,
                accent: 0x89b4fa,
                on_accent: 0x11111b,
                success: 0xa6e3a1,
                error: 0xf38ba8,
            },
            false,
        ),
    },
    Theme {
        id: "catppuccin-macchiato",
        name: "Catppuccin Macchiato",
        appearance: Appearance::Dark,
        palette: catppuccin(
            Flavour {
                base: 0x24273a,
                mantle: 0x1e2030,
                panel: 0x363a4f,
                text: 0xcad3f5,
                border: 0x494d64,
                accent: 0x8aadf4,
                on_accent: 0x181926,
                success: 0xa6da95,
                error: 0xed8796,
            },
            false,
        ),
    },
    Theme {
        id: "catppuccin-frappe",
        name: "Catppuccin Frappé",
        appearance: Appearance::Dark,
        palette: catppuccin(
            Flavour {
                base: 0x303446,
                mantle: 0x292c3c,
                panel: 0x414559,
                text: 0xc6d0f5,
                border: 0x51576d,
                accent: 0x8caaee,
                on_accent: 0x232634,
                success: 0xa6d189,
                error: 0xe78284,
            },
            false,
        ),
    },
    Theme {
        id: "catppuccin-latte",
        name: "Catppuccin Latte",
        appearance: Appearance::Light,
        palette: catppuccin(
            Flavour {
                base: 0xeff1f5,
                mantle: 0xe6e9ef,
                panel: 0xe6e9ef,
                text: 0x4c4f69,
                border: 0xbcc0cc,
                accent: 0x1e66f5,
                on_accent: 0xffffff,
                success: 0x40a02b,
                error: 0xd20f39,
            },
            true,
        ),
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
}

/// What the user picked: the mode and, for each appearance, the theme to use
/// when that appearance is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Choice {
    pub mode: Mode,
    pub light: &'static str,
    pub dark: &'static str,
}

impl Default for Choice {
    fn default() -> Self {
        Self {
            mode: Mode::System,
            light: SPOTLIGHT_LIGHT,
            dark: SPOTLIGHT_DARK,
        }
    }
}

impl Choice {
    /// The theme for `appearance`.
    pub fn theme_for(&self, appearance: Appearance) -> &'static str {
        match appearance {
            Appearance::Dark => self.dark,
            Appearance::Light => self.light,
        }
    }

    /// Which appearance is on, given the system's.
    pub fn appearance(&self, system: Appearance) -> Appearance {
        match self.mode {
            Mode::System => system,
            Mode::Light => Appearance::Light,
            Mode::Dark => Appearance::Dark,
        }
    }

    /// The same choice, with `id` the theme of its own appearance.
    pub fn with_theme(mut self, id: &'static str) -> Self {
        match theme(id).map(|theme| theme.appearance) {
            Some(Appearance::Dark) => self.dark = id,
            Some(Appearance::Light) => self.light = id,
            None => {}
        }
        self
    }

    /// A choice from its saved forms; unknown or mismatched ids fall back to
    /// the defaults.
    pub fn from_settings(mode: Option<&str>, light: Option<&str>, dark: Option<&str>) -> Self {
        let mut choice = Self {
            mode: mode.and_then(Mode::from_setting).unwrap_or_default(),
            ..Self::default()
        };
        for id in [light, dark].into_iter().flatten() {
            if let Some(theme) = theme(id.trim()) {
                choice = choice.with_theme(theme.id);
            }
        }
        choice
    }
}

pub const MODE_SETTING: &str = "theme_mode";
pub const LIGHT_SETTING: &str = "theme_light";
pub const DARK_SETTING: &str = "theme_dark";

pub fn theme(id: &str) -> Option<&'static Theme> {
    THEMES.iter().find(|theme| theme.id == id)
}

/// The themes of `appearance`, in listing order.
pub fn themes_for(appearance: Appearance) -> impl Iterator<Item = &'static Theme> {
    THEMES
        .iter()
        .filter(move |theme| theme.appearance == appearance)
}

static CHOICE: Mutex<Choice> = Mutex::new(Choice {
    mode: Mode::System,
    light: SPOTLIGHT_LIGHT,
    dark: SPOTLIGHT_DARK,
});
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

pub fn choice() -> Choice {
    *CHOICE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
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

/// Makes `choice` the one in use. Returns whether the palette changed, which
/// is when the windows need redrawing.
pub fn set_choice(choice: Choice) -> bool {
    *CHOICE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = choice;
    resolve()
}

/// Tells the theme what the system's appearance is now. Returns whether the
/// palette changed.
pub fn set_system(appearance: Appearance) -> bool {
    SYSTEM_LIGHT.store(appearance == Appearance::Light, Ordering::Relaxed);
    resolve()
}

fn resolve() -> bool {
    let choice = choice();
    let appearance = choice.appearance(system_appearance());
    let id = choice.theme_for(appearance);
    let index = THEMES
        .iter()
        .position(|theme| theme.id == id)
        .or_else(|| {
            THEMES
                .iter()
                .position(|theme| theme.appearance == appearance)
        })
        .unwrap_or(0);
    CURRENT.swap(index, Ordering::Relaxed) != index
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_each_appearance_has_a_default() {
        for (ix, theme) in THEMES.iter().enumerate() {
            assert!(THEMES[..ix].iter().all(|other| other.id != theme.id));
        }
        for appearance in [Appearance::Dark, Appearance::Light] {
            assert!(themes_for(appearance).next().is_some());
        }
        assert_eq!(
            theme(Choice::default().dark).map(|t| t.appearance),
            Some(Appearance::Dark)
        );
        assert_eq!(
            theme(Choice::default().light).map(|t| t.appearance),
            Some(Appearance::Light)
        );
    }

    #[test]
    fn choice_round_trips_and_ignores_unknown_ids() {
        let choice = Choice::from_settings(Some("dark"), Some("nope"), Some(SPOTLIGHT_DARK));
        assert_eq!(choice.mode, Mode::Dark);
        assert_eq!(choice.light, SPOTLIGHT_LIGHT);
        // A light theme saved as the dark one is not taken.
        let choice = Choice::from_settings(None, None, Some(SPOTLIGHT_LIGHT));
        assert_eq!(choice.dark, SPOTLIGHT_DARK);
        assert_eq!(Mode::from_setting("system"), Some(Mode::System));
        assert_eq!(Mode::from_setting("blue"), None);
    }

    #[test]
    fn mode_picks_the_appearance() {
        let mut choice = Choice::default();
        assert_eq!(choice.appearance(Appearance::Light), Appearance::Light);
        choice.mode = Mode::Dark;
        assert_eq!(choice.appearance(Appearance::Light), Appearance::Dark);
    }
}
