# SbarHive plan: releases

The original planning document was never exported, so this plan rebuilds it from `CLAUDE.md` and the hive spike (`docs/adr/0001-ui-toolkit.md`). Any later conflict is settled here with a written decision.

## Where we are (2026-10-22)

| Release | State |
| --- | --- |
| 0.1.0 "The hive" | Released 2026-10-20 |
| 0.2.0 "Engines", 0.2.1, 0.2.2 | Released 2026-10-20; RPM and DEB packages from 0.2.1 |
| 0.3.0 "Drop" | Released 2026-10-22, with RPM and DEB packages |
| 0.3.1 | Released 2026-10-22: the drop hive only during a real drag of files with an action (S-021) |
| 0.3.2 | Released 2026-10-22: the opening animation plays on the layer path, and drop follow-ups (S-022) |
| 0.3.3 | Released 2026-10-23: the drop shortcut registers through the global shortcuts portal on GNOME and KDE (S-014e), and the layer hive keeps off the keyboard at KDE Extreme without a token (S-027). S-014f, S-016 and S-017 remain for 0.4.0 |
| 0.4.0 "Portal and look" | Next: S-014e, S-014f, S-016, S-017 |
| 0.5.0 "X11" | Planned: S-015 |
| 0.6.0 "Windows" | Planned: starts with ADR 0005 |

Each release has a `vX.Y.Z` tag and a GitHub release page with its packages. Changes are in `CHANGELOG.md`, known limits in the repo's issues.

## 0.1.0 "The hive" (Linux, images)

Goal: a Linux user selects images in the file manager, opens the hive from the right click menu, picks a format, and finds the result next to the original.

| Component | Scope |
| --- | --- |
| core | Action registry from TOML, matching actions to the selection, conflict free output names, atomic writes, job queue with progress and cancel |
| engines/image | PNG, JPEG, WebP, AVIF, ICO, TIFF, BMP, GIF; resize, rotate, flip, greyscale, compress, strip metadata, merge images into one PDF |
| ui | Production quality hive from the spike, icons, a progress indicator that takes no focus, tokens generated from `design/tokens.json` |
| daemon | Background service on a Unix socket, tray icon (StatusNotifierItem), autostart with the session, xdg-activation token passing |
| cli | `sbarhive open`, `sbarhive convert --to`, `sbarhive actions`, `sbarhive daemon` |
| integrations | Dolphin first (a service menu with one item at the top), then Nautilus (nautilus-python), Nemo (action), Thunar (custom action) |
| Direct shortcut | `sbarhive open --from-clipboard` reads copied files (`text/uri-list`), with a Hyprland recipe binding Super plus right click to copy the selection and open the hive |
| Distribution | User install and uninstall script, systemd user unit, desktop file, bilingual README, GPLv3, CHANGELOG |

Out of scope: audio, video, documents, settings, Windows.

Decision (2026-10-20): the progress indicator becomes the tray icon tooltip plus one notification when each job finishes. On Wayland an ordinary window cannot appear without taking focus, and the only way around that (wlr-layer-shell) is missing on GNOME (KDE has it since Plasma 5.20, ADR 0004). "No work blocks the UI" outranks the indicator's form.

Decision: we do not replace the file manager's right click menu. The menu belongs to the file manager, there is no official API to replace it, and patching Dolphin means maintaining a fork every user installs and every update breaks. Instead: one item at the top of the menu, and a shortcut that opens the hive directly.

## 0.2.0 "Engines"

Goal: the same hive works with video, audio, documents and PDF, and shows only what the machine can run.

| Component | Scope |
| --- | --- |
| core::probe | Detect FFmpeg, LibreOffice and Ghostscript, hide unavailable actions, `sbarhive doctor` |
| engines/ffmpeg | Video to MP4, WebM, GIF; extract audio; convert audio (MP3, OGG, FLAC, WAV, M4A); compress video; real progress from `-progress` |
| engines/office | Office and ODF documents to PDF, DOCX and ODT both ways, through headless LibreOffice |
| engines/pdf | Compress PDF with Ghostscript, merge PDFs, split pages, pages to images |
| ui | Settings window: language, theme, output location, naming policy, quality mode |
| config | `~/.config/sbarhive/config.toml`, read by the service and reloaded on change |
| Identity | Logo and icon as SVG, landing page in `site/` |

Out of scope: Windows, macOS, Flatpak, the GlobalShortcuts portal.

Decision (2026-10-20): the naming policy is fixed, not a setting. The output takes the original name with the new extension; if that name exists, "(1)", "(2)" is added and nothing is ever overwritten. Every other option (overwrite, replace the original) breaks "the original is never modified" or risks the user's files, and a name suffix alone does not justify a field. Revisit if users ask.

Reload on change: the service polls `config.toml` and `actions/*.toml` every two seconds, so hand edits apply without `sbarhive reload`, and a language or theme change rebuilds the windows.

## 0.3.0 "Drop"

Goal: a GNOME or KDE user drags a file, presses a shortcut of their choice during the drag, the hive appears, and dropping the file on a cell runs its action. Reference: `docs/adr/0003-drop-hive.md`.

| Component | Scope |
| --- | --- |
| ui | Wayland drop receiver (a private `wl_data_device` on winit's display, bound on every show, copy only, `text/uri-list`) and a "waiting for drop" mode with cell highlight |
| cli and daemon | `sbarhive drop` and its IPC message, bound by the user to a custom system shortcut |
| Docs | Binding the shortcut on GNOME and KDE, and a `bind` line for Hyprland |
| Layer and preview | The hive as a `zwlr_layer_shell_v1` overlay (a full screen catcher with the hive as a software rendered subsurface) where the compositor offers it, an xdg_toplevel window on GNOME, X11 and with a screen reader; the receiver on the layer connection; the file itself in the centre cell from shared thumbnails or its type icon. Based on `docs/adr/0004-hive-overlay.md`, ahead of S-017b and S-017c. Story: S-018 (done) |
| Drop on Hyprland | In drop mode the catcher takes the drag with an input region the size of the hive, because Hyprland sends no drag motion to a subsurface of a layer surface; outside the hive drags and clicks pass through. Ahead of S-019. Story: S-020 (done) |
| Position, hold, preview | On the layer path the hive appears at the pointer or drag position and adapts to screen edges, the window path stays centred; the screen reader rule needs a running reader, not the flag alone; the drop hive stays while the key is held (`drop --hold`, `--release`); the file's image in the centre after the drag enters. Amends ADR 0003 and 0004. Story: S-019 (done) |

Stories: `backlog/S-014-drop-hive.md` (split into S-014a to S-014f), `backlog/S-018-hive-overlay.md` (S-018a to S-018h), `backlog/S-020-hyprland-layer-drop.md` (ahead of S-019), `backlog/S-019-hive-at-pointer.md` (S-019a to S-019f). Follow-ups shipped in 0.3.1 (`backlog/S-021-drop-only-on-drag.md`) and 0.3.2 (`backlog/S-022-drop-followups.md`).

Out of scope: the X11 path ("hive first, then drag", story S-015), Windows, running Hyprland live.

> Manager decision (2026-10-22): 0.3.0 shipped what was finished and verified. Its goal is met with a custom shortcut the user binds on GNOME, KDE and Hyprland (`docs/drop-shortcut.md`). The portal (S-014e, S-014f), signed distribution (S-016, needs accounts the maintainer opens) and the glass look (S-017) moved to 0.4.0.

## 0.4.0 "Portal and look"

Goal: the shortcut registers through the portal with no manual setup, install and update come from signed repositories, and the hive is glass and animated.

| Component | Scope |
| --- | --- |
| Portal | Register the shortcut through GlobalShortcuts with ashpd, changeable from GNOME and KDE settings, with the activation token on GNOME |
| Distribution | A COPR repo for Fedora and a signed APT repo for Ubuntu and Debian, so install and update are plain `dnf` or `apt`, with packages signed by an OpenPGP key (today dnf warns "skipped OpenPGP checks"). Story: S-016 |
| Look | Glass hive: a transparent token surface over the compositor's blur (KDE, Hyprland), opaque on GNOME, and token driven motion: open ring by ring, a highlight that slides between cells, press, close, respecting reduced motion. Based on `docs/adr/0004-hive-overlay.md`. Story: S-017 (approved) |

## 0.5.0 "X11"

Goal: the hive and drop work in an X11 session, with the hive at the pointer as `CLAUDE.md` says.

| Component | Scope |
| --- | --- |
| ui | The hive as an ordinary window at the pointer, adapting to screen edges, and the "hive first, then drag" drop order with an XDND receiver |
| Testing | A headless X server and a temporary HOME, never a live desktop |

Story: S-015 (draft). Out of scope: Windows, macOS.

## 0.6.0 "Windows"

Goal: a Windows 10 or 11 user selects a file in Explorer, picks SbarHive from the right click menu, the hive appears at the pointer, and the result is saved next to the original, in Arabic and English, with no file leaving the machine.

Before any code: ADR 0005 (`docs/adr/0005-windows.md`) settles the installer and its format, package signing, tray icon and notifications, where external engines live, and how to test on real Windows or a virtual machine.

| Component | Scope |
| --- | --- |
| core and daemon | IPC on a named pipe instead of a Unix socket, config and temp paths in `%APPDATA%` and `%TEMP%`, tray icon and notifications on Windows |
| ui | The hive as a frameless window at the pointer that takes focus, drop through an OLE receiver, the global shortcut |
| engines | The image engine as is; FFmpeg, LibreOffice and Ghostscript from Windows paths or downloaded on demand; kill the process tree on cancel (start in `crates/engines/src/office/non_unix.rs`) |
| integrations/windows | The legacy registry menu first (unsigned), then the Windows 11 menu with IExplorerCommand and a signed sparse package |
| Distribution | An installer attached to the GitHub release, built in CI on a Windows runner |

Needs from the maintainer: a code signing certificate for the sparse package and the installer (without it only the legacy menu works and SmartScreen warns), and a Windows machine or VM for live checks.

Stories: written after ADR 0005 is accepted, numbered from the next free story number. Out of scope: macOS.

## Work order

1. Shared contracts in `crates/core` (action, job, engine and IPC types), then the workspace skeleton.
2. Parallel tracks that share no files: core, image engine, UI, integrations, identity and site.
3. Wire the service and CLI, independent review, tag `v0.1.0`.
4. 0.2 engines in parallel, then settings, review, tag `v0.2.0`.
5. 0.3.0: drop, layer and hive at the pointer, review, tag `v0.3.0`; follow-ups in 0.3.1 and 0.3.2.
6. 0.4.0: portal, signed packages and glass; then 0.5.0 for X11; then ADR 0005 and Windows stories for 0.6.0.
