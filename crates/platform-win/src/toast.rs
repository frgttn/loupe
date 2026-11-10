//! Windows notifications (toasts) for an unpackaged app.
//!
//! A toast needs an AppUserModelID Windows knows: Loupe registers its own
//! under `HKCU\Software\Classes\AppUserModelId`, which names it in the
//! notification and in Settings › Notifications, without a Start menu
//! shortcut; its `IconUri` is the app icon at the size Windows draws it,
//! saved as a PNG next to the data. Clicking a toast opens its `loupe://` link, which reaches the
//! running instance the way any link does (see `protocol`), so nothing has to
//! listen for the click.

use windows::{
    Data::Xml::Dom::XmlDocument,
    UI::Notifications::{ToastNotification, ToastNotificationManager},
    Win32::{
        Foundation::ERROR_FILE_NOT_FOUND,
        System::{
            Com::{COINIT_MULTITHREADED, CoInitializeEx},
            Registry::{HKEY_CURRENT_USER, RegDeleteTreeW},
        },
    },
    core::HSTRING,
};

use crate::{paths, protocol, registry};

/// The same id the windows carry (`WindowOptions::app_id`).
pub const APP_ID: &str = "dev.frgttn.loupe";
const DISPLAY_NAME: &str = "Loupe";

/// Shows a toast with `title` and `body`; clicking it opens `link`.
/// `icon` is the app's `.ico`, which the notification shows as its app
/// icon. Blocking but quick; fine from any thread.
pub fn show(title: &str, body: &str, link: &str, icon: &[u8]) -> Result<(), String> {
    register(icon)?;
    unsafe {
        // WinRT needs COM on this thread; already initialized is fine.
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let xml = format!(
        r#"<toast activationType="protocol" launch="{link}"><visual><binding template="ToastGeneric"><text>{title}</text><text>{body}</text></binding></visual></toast>"#,
        link = escape(link),
        title = escape(title),
        body = escape(body),
    );
    let shown = (|| {
        let document = XmlDocument::new()?;
        document.LoadXml(&HSTRING::from(xml))?;
        let toast = ToastNotification::CreateToastNotification(&document)?;
        ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(APP_ID))?.Show(&toast)
    })();
    shown.map_err(|err| format!("could not show the notification: {err}"))
}

/// Removes the registered AppUserModelID (when uninstalling).
pub fn unregister() -> Result<(), String> {
    let key = HSTRING::from(key());
    let status = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, &key) };
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(());
    }
    status
        .ok()
        .map_err(|err| format!("could not remove {key}: {err}"))
}

fn key() -> String {
    format!(r"Software\Classes\AppUserModelId\{APP_ID}")
}

fn register(icon: &[u8]) -> Result<(), String> {
    let key = key();
    let current = |name| registry::string(&format!(r"HKCU\{key}"), name);
    if current("DisplayName").as_deref() != Some(DISPLAY_NAME) {
        protocol::set(&key, "DisplayName", DISPLAY_NAME)?;
    }
    // Without an icon the notification still shows, with Windows' placeholder.
    match save_icon(icon) {
        Ok(path) => {
            let path = path.display().to_string();
            if current("IconUri").as_deref() != Some(path.as_str()) {
                protocol::set(&key, "IconUri", &path)?;
            }
        }
        Err(err) => eprintln!("[toast] no icon: {err}"),
    }
    Ok(())
}

/// The size Windows draws a notification's app icon at, in logical pixels.
const ICON_SIZE: u32 = 16;

/// Saves the `.ico`'s picture for the notification's app icon as a PNG in
/// the data folder, unless it is there already. Windows shrinks a large
/// picture badly at this size, so this takes the icon's own hand-sized one
/// for the screen's scale (16 px at 100 %, 24 px at 150 %…).
fn save_icon(icon: &[u8]) -> Result<std::path::PathBuf, String> {
    let dir = paths::data_dir().ok_or("no data folder")?;
    let size = ICON_SIZE * screen_dpi() / 96;
    let frame = ico_frame(icon, size).ok_or("the icon has no pictures")?;
    let image = image::load_from_memory_with_format(&frame, image::ImageFormat::Ico)
        .map_err(|err| format!("could not read the icon: {err}"))?;
    let image = if image.width() == size {
        image
    } else {
        image.resize(size, size, image::imageops::FilterType::Lanczos3)
    };
    let mut png = Vec::new();
    image
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|err| format!("could not make the PNG: {err}"))?;
    // The size in the name, so a new scale isn't hidden by Windows' cache.
    let path = dir.join(format!("notification-icon-{}.png", image.width()));
    if std::fs::read(&path).ok().as_deref() != Some(png.as_slice()) {
        std::fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
        std::fs::write(&path, &png)
            .map_err(|err| format!("could not save {}: {err}", path.display()))?;
    }
    Ok(path)
}

/// The screen's DPI as the user set it (96 at 100 %). From the registry,
/// since DPI calls answer 96 to a process that isn't DPI aware.
fn screen_dpi() -> u32 {
    registry::dword(r"HKCU\Control Panel\Desktop\WindowMetrics", "AppliedDPI")
        .filter(|dpi| (96..=480).contains(dpi))
        .unwrap_or(96)
}

/// The picture of an `.ico` closest to `size` (the smallest at least that
/// big, else the largest), as an `.ico` of its own.
fn ico_frame(icon: &[u8], size: u32) -> Option<Vec<u8>> {
    let u16_at = |at: usize| Some(u16::from_le_bytes(icon.get(at..at + 2)?.try_into().ok()?));
    let u32_at = |at: usize| Some(u32::from_le_bytes(icon.get(at..at + 4)?.try_into().ok()?));
    let count = u16_at(4)? as usize;
    // (width, entry offset, data length, data offset); 0 means 256.
    let entries: Vec<(u32, usize, usize, usize)> = (0..count)
        .filter_map(|ix| {
            let entry = 6 + 16 * ix;
            let width = match *icon.get(entry)? {
                0 => 256,
                width => u32::from(width),
            };
            Some((
                width,
                entry,
                u32_at(entry + 8)? as usize,
                u32_at(entry + 12)? as usize,
            ))
        })
        .collect();
    let &(_, entry, len, offset) = entries
        .iter()
        .filter(|frame| frame.0 >= size)
        .min_by_key(|frame| frame.0)
        .or_else(|| entries.iter().max_by_key(|frame| frame.0))?;
    let data = icon.get(offset..offset + len)?;
    let mut single = Vec::with_capacity(22 + len);
    single.extend_from_slice(&[0, 0, 1, 0, 1, 0]);
    single.extend_from_slice(icon.get(entry..entry + 12)?);
    single.extend_from_slice(&22u32.to_le_bytes());
    single.extend_from_slice(data);
    Some(single)
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
