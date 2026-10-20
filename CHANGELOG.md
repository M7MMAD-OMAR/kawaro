# Changelog

All notable changes. Format: one line per finished story, newest first under each version.

## [Unreleased]

### Added
- Drop hive waiting text (S-014c): the hive shown by `sbarhive drop` now reads "Drop files here" («أفلت الملف هنا») in its centre until a drag enters, and with `SBARHIVE_DEBUG_FOCUS` the service prints one `drop cells: ID ...` line per enter with the cells built for the dragged files.
- File preview in the centre cell (S-018h): with no cell hovered the centre shows the selected file's cached thumbnail, decoded row by row and downscaled to the square inscribed in the centre hexagon before Slint and freed on every hide, else the kind icon with the format code; several files add two stacked hexagons and a count badge, mirrored in Arabic; the centre is read as the file name or the selection text, and the layout is unchanged.
- Overlay buffers, docs and suite (S-018f): hiding the overlay now frees every buffer of that show at once, the catcher's pixel included, instead of waiting for the compositor to release them; `docs/architecture.md` describes the layer and window paths and their choice rule, the Hyprland README gives an unverified layer rule that turns off the overlay animation, and `tests/overlay/run.sh` runs the whole overlay suite.
- Overlay colour scheme and cursor (S-018e): `SBARHIVE_COLOR_SCHEME` given to the service now decides the overlay colours ahead of the portal when `theme = "auto"`, and the overlay asks for the default arrow cursor (`wp_cursor_shape_v1`) on every pointer enter.
- Drop on the hive overlay (S-018d): `sbarhive drop` on the layer-shell overlay takes dragged files again, its drop receiver now bound on the overlay's connection to the hive surface, with clicks and drags outside the hive passing through.
- Hive overlay input and focus (S-018c): the activation token is sent once the overlay is mapped, `SBARHIVE_DEBUG_FOCUS` prints one `hive surface: layer|window X Y W H` line per show, and while a screen reader runs (`org.a11y.Status.ScreenReaderEnabled`) the next show is a window, back to the overlay when it stops.
- Hive overlay (S-018b): on compositors that offer wlr-layer-shell (checked on headless KWin) every hive show is a full-output overlay with the hive centred in it, no window frame or tiling, closed with Esc; GNOME and `SBARHIVE_SURFACE=window` keep the window, `SBARHIVE_SURFACE=layer|window` forces the choice.
- Centre picture lookups (S-018g): `sbarhive_ui::thumb` finds a valid cached freedesktop thumbnail for a file (GLib and Qt URIs, symlink targets, read only), its MIME type from shared-mime-info `globs2`, its kind icon (new monoline `file` icon) and format code.
- Overlay groundwork (S-018a): pure surface choice between a layer-shell overlay and a window, hive placement and buffer size per scale, evdev keys, colour scheme and cursor shape decisions in `sbarhive_ui::overlay`.
- Drop receiver (S-014b): on Wayland the hive shown by `sbarhive drop` takes a file dragged onto it, reads the files when the drag enters, builds their cells and runs the action of the cell it is dropped on, as a copy only; a drop elsewhere closes it and runs nothing.
- Drop groundwork (S-014a): `sbarhive drop [--token T]` asks the service for a hive that waits for dragged files, with a shared `text/uri-list` parser, drop point cell picking and the drop hive state machine.
- Fixed: `scripts/install.sh` stops a running daemon before replacing it, so an upgrade never leaves the old one running (the new one used to exit with "already running").

## [0.2.2] - 2026-10-20

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
