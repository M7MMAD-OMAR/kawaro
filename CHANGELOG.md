# Changelog

All notable changes. Format: one line per finished story, newest first under each version.

## [Unreleased]

### Added
- Hive cell icons (S-001): 21 monoline SVG icons above the format code, chosen per action with `icon` in `actions/*.toml`, coloured like the code and embedded in the binary.
- Focus tests (S-013): `tests/focus/` measures hive focus in private headless GNOME and KWin with no, real and made-up activation tokens; the hive now really sends the xdg-activation token (a token builds a fresh hive window, since Slint applies it only when a window adapter is created).
- Settings window (S-011): language, theme, output place and folder, quality, notifications, built from the vendored Slint Material 3 components with token colours and no shadows, Arabic and English with full mirroring.
- Office and PDF engines (S-009, S-010): documents to PDF, DOCX, ODT, XLSX, ODS, PPTX, ODP through LibreOffice; PDF merge, rotate, split (ZIP) in pure Rust; compress and pages to PNG (ZIP) through Ghostscript.
- FFmpeg engine (S-008): video to MP4, WebM, MKV, MOV and GIF, audio extraction and conversion, video compression with real progress.
- Image engine (S-004): PNG, JPEG, WebP, AVIF, GIF, BMP, TIFF, ICO, resize, rotate, flip, grayscale, compress, strip metadata, images to one PDF.
- Core (S-002, S-003): action registry from TOML, collision-free output naming, atomic finalisation, job queue with progress and cancel.
- File manager integrations and installer (S-006, S-007): Dolphin, Nautilus, Nemo, Thunar, Hyprland helper, systemd user unit.
- Brand: hexagon and cactus app icon, tray icon, wordmark, PNG exports; bilingual landing page in site/.
- Core settings file `~/.config/sbarhive/config.toml`: language, theme, output place, quality preset.
- `sbarhived` service: Unix socket protocol, job pump, tray icon, end-of-job notifications.
- `sbarhive` command line: open, convert, run, actions, daemon control, clipboard input.
- Hive spike and ADR 0001: Slint with the software renderer, measured on headless KWin and GNOME.
