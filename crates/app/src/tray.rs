//! Notification-area icon: open the launcher or settings, toggle start at login,
//! update, quit.
//!
//! `tray-icon` creates a hidden window on the calling thread, so this must run
//! on GPUI's main thread, whose message loop then dispatches the tray messages.

use loupe_core::update::Status as UpdateStatus;
use loupe_platform_win::autostart;
use tray_icon::{
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem},
};

pub enum Command {
    Open,
    Settings,
    ToggleAutostart,
    /// Check for updates, or restart into the downloaded one.
    Update,
    Quit,
}

pub struct Tray {
    icon: TrayIcon,
    autostart: CheckMenuItem,
    update: MenuItem,
}

impl Tray {
    /// Shows the tray icon. Menu picks and left clicks arrive on `commands`.
    pub fn create(hotkey: &str, commands: async_channel::Sender<Command>) -> Result<Self, String> {
        let open = MenuItem::new("Open Loupe", true, None);
        let settings = MenuItem::new("Settings…", true, None);
        let autostart = CheckMenuItem::new("Start at login", true, autostart::is_enabled(), None);
        let update = MenuItem::new("Check for Updates", true, None);
        let quit = MenuItem::new("Quit", true, None);
        let menu = Menu::new();
        menu.append_items(&[
            &open,
            &settings,
            &PredefinedMenuItem::separator(),
            &autostart,
            &update,
            &PredefinedMenuItem::separator(),
            &quit,
        ])
        .map_err(|err| err.to_string())?;

        let ids: [(MenuId, fn() -> Command); 5] = [
            (open.id().clone(), || Command::Open),
            (settings.id().clone(), || Command::Settings),
            (autostart.id().clone(), || Command::ToggleAutostart),
            (update.id().clone(), || Command::Update),
            (quit.id().clone(), || Command::Quit),
        ];
        let menu_commands = commands.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            if let Some((_, command)) = ids.iter().find(|(id, _)| *id == event.id) {
                let _ = menu_commands.send_blocking(command());
            }
        }));
        TrayIconEvent::set_event_handler(Some(move |event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = commands.send_blocking(Command::Open);
            }
        }));

        let icon = TrayIconBuilder::new()
            .with_icon(icon()?)
            .with_tooltip(tooltip(hotkey))
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .build()
            .map_err(|err| err.to_string())?;
        Ok(Self {
            icon,
            autostart,
            update,
        })
    }

    pub fn set_hotkey(&self, hotkey: &str) {
        let _ = self.icon.set_tooltip(Some(tooltip(hotkey)));
    }

    /// Flips start at login and shows the resulting state, which stays the old
    /// one if the registry write failed.
    pub fn toggle_autostart(&self) {
        if let Err(err) = autostart::set_enabled(!autostart::is_enabled()) {
            eprintln!("[tray] could not change start at login: {err}");
        }
        self.sync_autostart();
    }

    /// Updates the check mark after start at login changed elsewhere (the settings window).
    pub fn sync_autostart(&self) {
        self.autostart.set_checked(autostart::is_enabled());
    }

    /// Shows what the update item does now. Off while checking or
    /// downloading, and for a copy the installer didn't put there.
    pub fn set_update_status(&self, status: &UpdateStatus) {
        let text = match status {
            UpdateStatus::Checking => "Checking for Updates…".to_owned(),
            UpdateStatus::Downloading(version) => format!("Downloading v{version}…"),
            UpdateStatus::Ready(version) => format!("Restart to Update to v{version}"),
            _ => "Check for Updates".to_owned(),
        };
        self.update.set_text(text);
        self.update
            .set_enabled(!status.busy() && *status != UpdateStatus::Unmanaged);
    }
}

fn tooltip(hotkey: &str) -> String {
    format!("Loupe ({hotkey})")
}

/// The logo's polygons, the subpaths of `assets/logo-white.svg` and
/// `assets/logo-black.svg` (196 x 192 grid): eleven stripes that fade out to
/// the left, then the arrow. None of them overlap.
const MARK: &[&[(f32, f32)]] = &[
    &[(87.7, 16.3), (93.3, 10.7), (93.3, 64.7), (87.7, 70.3)],
    &[(87.7, 121.7), (93.3, 127.3), (93.3, 181.3), (87.7, 175.7)],
    &[(76.1, 27.9), (81.7, 22.3), (81.7, 76.3), (76.1, 81.9)],
    &[(76.1, 110.1), (81.7, 115.7), (81.7, 169.7), (76.1, 164.1)],
    &[(64.5, 39.5), (70.1, 33.9), (70.1, 87.9), (64.5, 93.5)],
    &[(64.5, 98.5), (70.1, 104.1), (70.1, 158.1), (64.5, 152.5)],
    &[(52.9, 51.1), (58.5, 45.5), (58.5, 146.5), (52.9, 140.9)],
    &[(41.3, 62.7), (46.9, 57.1), (46.9, 134.9), (41.3, 129.3)],
    &[(29.7, 74.3), (35.3, 68.7), (35.3, 123.3), (29.7, 117.7)],
    &[(18.1, 85.9), (23.7, 80.3), (23.7, 111.7), (18.1, 106.1)],
    &[(8., 96.), (12.1, 91.9), (12.1, 100.1)],
    &[
        (98., 6.),
        (98., 60.),
        (134., 96.),
        (98., 132.),
        (98., 186.),
        (188., 96.),
    ],
];

/// Whether (u, v) is inside `poly` (even-odd crossing test).
fn inside(poly: &[(f32, f32)], u: f32, v: f32) -> bool {
    let mut hit = false;
    let mut prev = poly[poly.len() - 1];
    for &(x, y) in poly {
        if (y > v) != (prev.1 > v) && u < (prev.0 - x) * (v - y) / (prev.1 - y) + x {
            hit = !hit;
        }
        prev = (x, y);
    }
    hit
}

/// The logo (`MARK`) on a transparent background, drawn at 32 px with 4x4
/// supersampling. White on a dark taskbar, black on a light one.
fn icon() -> Result<Icon, String> {
    const SIZE: u32 = 32;
    const SAMPLES: u32 = 4;
    // The mark spans 180 units (8..188 by 6..186) around (98, 96); this many
    // pixels per unit fills the icon with half a pixel to spare on each side.
    const SCALE: f32 = 31. / 180.;
    let mark: [u8; 3] = if loupe_platform_win::system::taskbar_dark() {
        [255, 255, 255]
    } else {
        [0, 0, 0]
    };

    let in_mark = |u: f32, v: f32| MARK.iter().any(|poly| inside(poly, u, v));

    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let mut covered = 0u32;
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let px = x as f32 + (sx as f32 + 0.5) / SAMPLES as f32;
                    let py = y as f32 + (sy as f32 + 0.5) / SAMPLES as f32;
                    if in_mark(98. + (px - 16.) / SCALE, 96. + (py - 16.) / SCALE) {
                        covered += 1;
                    }
                }
            }
            let alpha = covered as f32 / (SAMPLES * SAMPLES) as f32;
            rgba.extend([mark[0], mark[1], mark[2], (alpha * 255.).round() as u8]);
        }
    }
    Icon::from_rgba(rgba, SIZE, SIZE).map_err(|err| err.to_string())
}
