# Changelog

What changed in each release. The section for a version becomes its GitHub
release notes and what the app shows when it updates, so write it for people
using Spotlight. Versions follow [Semantic Versioning](https://semver.org).

## [Unreleased]

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
