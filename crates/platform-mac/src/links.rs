//! macOS counterpart of `loupe_platform_win::links`, with its signatures.
//! A stub for now: nothing here is implemented on macOS yet.

use loupe_core::{datetime::LocalTime, quicklink::Moment};
use std::path::PathBuf;

use std::{
    collections::hash_map::RandomState,
    hash::{BuildHasher, Hasher},
    time::{SystemTime, UNIX_EPOCH},
};

/// The local date and time, and a fresh random UUID.
pub fn moment() -> Moment {
    // UTC until the Mac's own clock and time zone are read.
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs() as i64);
    let (year, month, day) = civil_from_days(seconds.div_euclid(86_400));
    let of_day = seconds.rem_euclid(86_400);
    let random = || RandomState::new().build_hasher().finish();
    let (high, low) = (random(), random());
    Moment {
        now: LocalTime {
            year,
            month,
            day,
            hour: (of_day / 3600) as u8,
            minute: (of_day / 60 % 60) as u8,
            second: (of_day % 60) as u8,
        },
        // A version 4 UUID's shape.
        uuid: format!(
            "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
            high >> 32,
            (high >> 16) & 0xffff,
            high & 0x0fff,
            (low >> 48) & 0x3fff | 0x8000,
            low & 0xffff_ffff_ffff
        ),
    }
}

/// The date `days` after 1970-01-01 (Howard Hinnant's `civil_from_days`).
fn civil_from_days(days: i64) -> (i32, u8, u8) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u8;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
    let year = (yoe + era * 400 + i64::from(month <= 2)) as i32;
    (year, month, day)
}

/// The user's Downloads folder, where a save dialog starts; their profile
/// folder if Windows doesn't say.
pub fn downloads_folder() -> PathBuf {
    std::env::home_dir()
        .map(|home| home.join("Downloads"))
        .unwrap_or_default()
}

/// `%USERPROFILE%\Desktop` → `C:\Users\me\Desktop`. Unknown variables stay as
/// they are.
pub fn expand_env(text: &str) -> String {
    // `~/…` is what paths start with here; there are no `%VARIABLE%`s.
    match (text.strip_prefix('~'), std::env::home_dir()) {
        (Some(rest), Some(home)) if rest.is_empty() || rest.starts_with('/') => {
            format!("{}{rest}", home.display())
        }
        _ => text.to_owned(),
    }
}

/// An installed web browser.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Browser {
    pub name: String,
    pub exe: String,
}

/// The browsers registered on this machine, by name, each once. Blocking
/// (a few registry reads).
pub fn browsers() -> Vec<Browser> {
    Vec::new()
}
