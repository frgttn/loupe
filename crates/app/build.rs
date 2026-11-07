//! Works out the version the app shows and, for a Windows build, embeds the
//! Windows resources: the exe's icon (Explorer, taskbar, Alt+Tab) and its
//! version (Properties › Details).
//!
//! `LOUPE_VERSION` is the version in the workspace `Cargo.toml` for a
//! release build (`--release`), `0.1.0`, and that plus the day it was built
//! for a dev build, `0.1.0.20261024`, so it's clear which build of the working
//! branch is running. The date is the build machine's local one.

use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    // The version is set in the workspace manifest.
    println!("cargo:rerun-if-changed=../../Cargo.toml");

    let semver = env::var("CARGO_PKG_VERSION").unwrap();
    let version = if env::var("PROFILE").as_deref() == Ok("release") {
        semver.clone()
    } else {
        // A path that never exists makes Cargo run this on every build;
        // otherwise the date would stay at the day of the first one.
        println!("cargo:rerun-if-changed=rerun-every-build");
        format!("{semver}.{}", today())
    };
    println!("cargo:rustc-env=LOUPE_VERSION={version}");

    // Resources are a Windows exe's; other targets get theirs from the bundle.
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let rc = out_dir.join("app.rc");
    fs::write(&rc, resource_script(&semver, &version)).unwrap();
    embed_resource::compile(&rc, embed_resource::NONE)
        .manifest_optional()
        .unwrap();
}

/// The icon and a version block. Windows wants the numeric version as four
/// 16-bit parts, so it carries only the semver; the text shows all of it.
fn resource_script(semver: &str, version: &str) -> String {
    let icon = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("assets/icon.ico");
    let icon = icon.display().to_string().replace('\\', "\\\\");
    let mut parts = semver
        .split(['.', '-', '+'])
        .map(|part| part.parse::<u16>().unwrap_or(0));
    let numeric = [0; 4]
        .map(|_| parts.next().unwrap_or(0).to_string())
        .join(",");
    format!(
        r#"1 ICON "{icon}"

1 VERSIONINFO
FILEVERSION {numeric}
PRODUCTVERSION {numeric}
FILEOS 0x40004
FILETYPE 0x1
BEGIN
    BLOCK "StringFileInfo"
    BEGIN
        BLOCK "040904B0"
        BEGIN
            VALUE "FileDescription", "Loupe"
            VALUE "FileVersion", "{version}"
            VALUE "InternalName", "loupe"
            VALUE "OriginalFilename", "loupe.exe"
            VALUE "ProductName", "Loupe"
            VALUE "ProductVersion", "{version}"
        END
    END
    BLOCK "VarFileInfo"
    BEGIN
        VALUE "Translation", 0x409, 1200
    END
END
"#
    )
}

/// Today's local date as `YYYYMMDD`, on the machine that builds.
#[cfg(windows)]
fn today() -> String {
    // SAFETY: GetLocalTime only fills in the struct it is given.
    let now = unsafe {
        let mut now = std::mem::zeroed();
        windows_sys::Win32::System::SystemInformation::GetLocalTime(&mut now);
        now
    };
    format!("{:04}{:02}{:02}", now.wYear, now.wMonth, now.wDay)
}

/// Today's local date as `YYYYMMDD`, on the machine that builds.
#[cfg(not(windows))]
fn today() -> String {
    let date = std::process::Command::new("date")
        .arg("+%Y%m%d")
        .output()
        .expect("`date` to tell the date");
    String::from_utf8_lossy(&date.stdout).trim().to_owned()
}
