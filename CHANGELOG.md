# Changelog

What changed in each release. The section for a version becomes its GitHub
release notes and what the app shows when it updates, so write it for people
using Loupe (called Spotlight up to 1.1.1). Versions follow [Semantic Versioning](https://semver.org).

## [Unreleased]

## [1.2.0] - 2026-11-07

- **Spotlight is now Loupe.** The new name avoids a clash with Apple's Spotlight. Loupe installs next to Spotlight as a separate app and starts with fresh settings and history: `Loupe-win-Setup.exe` installs it into `%LocalAppData%\Loupe`, and its data lives in `%APPDATA%\loupe`. Spotlight won't update to Loupe by itself; uninstall it from Windows Settings › Apps. Links now start with `loupe://`; `spotlight://` links no longer open the app.
- **Capture Screen Region:** the screen freezes and you drag out a rectangle, then pick **Copy Text** (T or Enter) to copy the text in it, **Scan QR** (Q) to open a QR code in it, or **Google Lens** (L) to look it up in your browser. Esc or a right click cancels. *Copy Text from Screen Region*, *Scan QR Code from Screen Region* and *Search Screen Region with Google Lens* do one of these straight away, and each command can have its own hotkey. Text is read in every language set up for Windows text recognition.

## [1.1.1] - 2026-11-06

- **Home screen:** Settings › Home Screen lists what root search shows before you type, in order. Move sections up or down, hide the ones you do not want, or reset both to the defaults.
- **Grid layout:** cards no longer overlap the rows below them.
- **Window:** the launcher clips to its own height, so clicks below it reach the windows beneath.

## [1.1.0] - 2026-11-05

- **Grid layout:** Settings › Appearance › Layout shows root search and every list as tiles instead of rows; Ctrl+Shift+G switches between them from anywhere in the launcher. Clipboard history and file search keep their rows beside the preview. Smooth scrolling works in the grid too.
- **New look:** the light and dark themes now follow Windows 11's own colours, and the switches in Settings look like Windows ones.
- **Simpler theme setting:** Settings › Appearance keeps System, Light and Dark. The Catppuccin themes are gone; if you used one, Spotlight switches to its own light or dark theme.

## [1.0.0] - 2026-10-27

The first release.

- **Installer and updates:** `Spotlight-win-Setup.exe` installs Spotlight for your user (no administrator prompt), adds it to the Start menu and starts it at login. New versions download in the background; *Restart to Update* in the tray menu or Settings › About switches to them, or the next start does. Uninstall from Windows Settings › Apps.
- **Root search** over apps, Windows Settings pages, Control Panel items, games, system commands, quicklinks, files, and the web, ranked by what you use.
- **Calculator** with units and currencies, **clipboard history**, **snippets** that expand as you type, **QR codes**, window layouts, and a settings window.
- Global hotkey (Alt+Space by default), a tray icon, and `spotlight://` links.
