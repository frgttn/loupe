//! Shows the "update ready" notification, to check how it looks and that
//! clicking it opens Settings › About in a running Loupe.
//!
//! `cargo run -p loupe-platform-win --example toast`

fn main() {
    let shown = loupe_platform_win::toast::show(
        "Loupe update ready",
        "Version 9.9.9 is ready. Click to open Settings and update.",
        "loupe://settings/about",
        include_bytes!("../../app/assets/icon.ico"),
    );
    match shown {
        Ok(()) => println!("shown"),
        Err(err) => eprintln!("{err}"),
    }
}
