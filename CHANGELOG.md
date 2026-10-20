# Changelog

All notable changes. Format: one line per finished story, newest first under each version.

## [Unreleased]

### Added
- SbarHive shows in app launchers (search «sbar» or «صبار»); opening it starts the service and opens the settings. New command `sbarhive settings`.

## [0.2.1] - 2026-10-20

### Added
- RPM and DEB packages, built on Ubuntu 22.04 and test-installed on Fedora 41, current Fedora, Ubuntu 22.04 and 24.04 and Debian 12, attached to each GitHub release.

## [0.2.0] - 2026-10-20 «المحركات»

### Added
- Focus tests (S-013): `tests/focus/` measures hive focus in private headless GNOME and KWin with no, real and made-up activation tokens under default and strict focus policies.
- Settings window (S-011): language, theme, output place and folder, quality, notifications, built from the vendored Slint Material 3 components with token colours and no shadows, Arabic and English with full mirroring; opened from the tray menu.
- Office and PDF engines (S-009, S-010): documents to PDF, DOCX, ODT, XLSX, ODS, PPTX, ODP through LibreOffice; PDF merge, rotate, split (ZIP) in pure Rust; compress and pages to PNG (ZIP) through Ghostscript.
- FFmpeg engine (S-008): video to MP4, WebM, MKV, MOV and GIF, audio extraction and conversion, video compression with real progress.
- `sbarhive doctor` and `sbarhive reload`; the service also reloads settings and user actions by itself when their files change.
- Brand: hexagon and cactus app icon, tray icon, wordmark, PNG exports; bilingual landing page in `site/`.

### Fixed
- The xdg-activation token now reaches the compositor (a token builds a fresh hive window, since Slint applies it only when a window adapter is created), so the hive gets focus under GNOME strict and KWin Extreme.
- The hive's corners are transparent on Wayland with the software renderer (vendored softbuffer 0.4.8 with `Argb8888` buffers).
- Only the hive window's own focus events can close the hive.
- No empty toplevel window at daemon start: the settings window is created on first open.

## [0.1.0] - 2026-10-20 «الخلية»

### Added
- Hive (S-001): 19 hexagonal cells with icons and format codes, one compact ring for six or fewer actions, Arabic mirroring, keyboard and screen reader access, reduced motion, colours and sizes generated from `design/tokens.json`.
- Image engine (S-004): PNG, JPEG, WebP, AVIF, GIF, BMP, TIFF, ICO, resize, rotate, flip, grayscale, compress, strip metadata, images to one PDF.
- Core (S-002, S-003): action registry from TOML, collision-free output naming, atomic finalisation, job queue with progress and cancel; settings file `~/.config/sbarhive/config.toml`.
- Service and command line (S-012): `sbarhived` with Unix socket protocol, tray icon and end-of-job notifications; `sbarhive open`, `convert`, `run`, `actions`, clipboard input.
- File manager integrations and installer (S-006, S-007): Dolphin, Nautilus, Nemo, Thunar, Hyprland helper, systemd user unit, user-local install and uninstall.
- Hive spike and ADR 0001: Slint with the software renderer, measured on headless KWin and GNOME and live on Hyprland.
