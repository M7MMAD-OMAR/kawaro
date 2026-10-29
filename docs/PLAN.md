# Kawaro plan: releases

The original planning document was never exported, so this plan rebuilds it from `CLAUDE.md` and the hive spike (`docs/adr/0001-ui-toolkit.md`). Any later conflict is settled here with a written decision.

## Where we are (2026-10-27)

| Release | State |
| --- | --- |
| 0.1.0 "The hive" | Released 2026-10-20 |
| 0.2.0 "Engines", 0.2.1, 0.2.2 | Released 2026-10-20; RPM and DEB packages from 0.2.1 |
| 0.3.0 "Drop" | Released 2026-10-22, with RPM and DEB packages |
| 0.3.1 | Released 2026-10-22: the drop hive only during a real drag of files with an action (S-021) |
| 0.3.2 | Released 2026-10-22: the opening animation plays on the layer path, and drop follow-ups (S-022) |
| 0.3.3 | Released 2026-10-23: the drop shortcut registers through the global shortcuts portal on GNOME and KDE (S-014e), and the layer hive keeps off the keyboard at KDE Extreme without a token (S-027). S-016 and S-017 remain for 0.4.0; the portal follow-ups S-014f1 to S-014f4 shipped in the 0.4.0 line and are done |
| 0.4.0-beta.1 | Cut 2026-10-26: the first public beta for Linux and Windows, a GitHub prerelease with RPM, DEB, zip, MSI and `SHA256SUMS`, built by CI on a `v0.4.0-beta.1` tag (S-055). Updates from inside the app (ADR 0006); Windows unsigned; the Windows drop hive and notifications are built (S-047, S-049) but not yet run on a live machine; `kawaro update` from the terminal (S-054c). Superseded by 0.4.0 |
| 0.4.0 "Portal and look" | Released 2026-10-27: the first official release for Linux and Windows. In-app updates (S-053, S-054, S-054c), the Windows build, installer and Explorer menu (S-045 to S-051), the Windows beta feedback block (S-079 to S-081, S-083 to S-088), the portal shortcut (S-014e, S-014f) and the glass and motion look (S-017a to S-017c). The Windows binaries are test-signed with a self-signed certificate, so SmartScreen warns (More info, then Run anyway) and a trusted OV certificate is still needed; every Windows live result is owner pending until the S-079 checklist report exists. S-016, S-017d to S-017e and the open rows of the block below remain |
| 0.4.1 | Cut 2026-10-29: the settings app shell with header, page rail and footer (S-094), the Updates page with installed version, check, update now, automatic check, channel and install mode (S-090), a simpler website (one download section, MSI only) and README, and the CI skip of two wall clock drop tests. Known gaps: download percent not shown (S-091), tray About item (S-092), the introduction film is being redone (S-094c); the Windows build is still test-signed and the Windows screenshots of the new settings are owner pending |
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
| cli | `kawaro open`, `kawaro convert --to`, `kawaro actions`, `kawaro daemon` |
| integrations | Dolphin first (a service menu with one item at the top), then Nautilus (nautilus-python), Nemo (action), Thunar (custom action) |
| Direct shortcut | `kawaro open --from-clipboard` reads copied files (`text/uri-list`), with a Hyprland recipe binding Super plus right click to copy the selection and open the hive |
| Distribution | User install and uninstall script, systemd user unit, desktop file, bilingual README, GPLv3, CHANGELOG |

Out of scope: audio, video, documents, settings, Windows.

Decision (2026-10-20): the progress indicator becomes the tray icon tooltip plus one notification when each job finishes. On Wayland an ordinary window cannot appear without taking focus, and the only way around that (wlr-layer-shell) is missing on GNOME (KDE has it since Plasma 5.20, ADR 0004). "No work blocks the UI" outranks the indicator's form.

Decision: we do not replace the file manager's right click menu. The menu belongs to the file manager, there is no official API to replace it, and patching Dolphin means maintaining a fork every user installs and every update breaks. Instead: one item at the top of the menu, and a shortcut that opens the hive directly.

## 0.2.0 "Engines"

Goal: the same hive works with video, audio, documents and PDF, and shows only what the machine can run.

| Component | Scope |
| --- | --- |
| core::probe | Detect FFmpeg, LibreOffice and Ghostscript, hide unavailable actions, `kawaro doctor` |
| engines/ffmpeg | Video to MP4, WebM, GIF; extract audio; convert audio (MP3, OGG, FLAC, WAV, M4A); compress video; real progress from `-progress` |
| engines/office | Office and ODF documents to PDF, DOCX and ODT both ways, through headless LibreOffice |
| engines/pdf | Compress PDF with Ghostscript, merge PDFs, split pages, pages to images |
| ui | Settings window: language, theme, output location, naming policy, quality mode |
| config | `~/.config/kawaro/config.toml`, read by the service and reloaded on change |
| Identity | Logo and icon as SVG, landing page in `site/` |

Out of scope: Windows, macOS, Flatpak, the GlobalShortcuts portal.

Decision (2026-10-20): the naming policy is fixed, not a setting. The output takes the original name with the new extension; if that name exists, "(1)", "(2)" is added and nothing is ever overwritten. Every other option (overwrite, replace the original) breaks "the original is never modified" or risks the user's files, and a name suffix alone does not justify a field. Revisit if users ask.

Reload on change: the service polls `config.toml` and `actions/*.toml` every two seconds, so hand edits apply without `kawaro reload`, and a language or theme change rebuilds the windows.

## 0.3.0 "Drop"

Goal: a GNOME or KDE user drags a file, presses a shortcut of their choice during the drag, the hive appears, and dropping the file on a cell runs its action. Reference: `docs/adr/0003-drop-hive.md`.

| Component | Scope |
| --- | --- |
| ui | Wayland drop receiver (a private `wl_data_device` on winit's display, bound on every show, copy only, `text/uri-list`) and a "waiting for drop" mode with cell highlight |
| cli and daemon | `kawaro drop` and its IPC message, bound by the user to a custom system shortcut |
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
| Portal | Register the shortcut through GlobalShortcuts with ashpd, changeable from GNOME and KDE settings, with the activation token on GNOME. Done: S-014e (registration) and S-014f1 to S-014f4 (the `portal_shortcut` switch, live apply and docs). Remaining portal work is the fallback role in the S-075 per-desktop contract, see the block below |
| Distribution | A COPR repo for Fedora and a signed APT repo for Ubuntu and Debian, so install and update are plain `dnf` or `apt`, with packages signed by an OpenPGP key (today dnf warns "skipped OpenPGP checks"). Story: S-016 |
| Look | Glass hive: a transparent token surface over the compositor's blur (KDE, Hyprland), opaque on GNOME, and token driven motion: open ring by ring, a highlight that slides between cells, press, close, respecting reduced motion. Based on `docs/adr/0004-hive-overlay.md`. Story: S-017 (approved) |

## Linux UX, distribution and Windows readiness (S-072 to S-082)

Decision (2026-10-26, after the specification audit of the stable roadmap): this block is the current work alongside S-016 and S-017. It replaces the earlier assumption that Alt-drag would come from the GlobalShortcuts portal alone, which the measured spike disproved.

| Story | Scope | State |
| --- | --- | --- |
| S-074 | The hive shows only its hexagonal cells, no background plate | Done |
| S-075 | Alt then drag out of the box, as a per-desktop evidence contract with substories S-075a to S-075f: a measured Hyprland compositor bind for Alt (S-075a), native KDE (S-075b spike as prerequisite, then S-075c) and native GNOME (S-075d spike as prerequisite, then S-075e) paths only on measured APIs, and a named non-Alt portal fallback (Super+Z) with the owner priority and conflict semantics recorded (S-075f). The Alt-only GlobalShortcuts mechanism is disproven and removed from every contract | Re-baselined 2026-10-26, top priority |
| S-076 | Settings without technical words, accessible drag-key picker | Approved |
| S-077 | A normal open can use the pointer position on the layer path; reproduction and a KDE pointer-source spike first | Approved, spike first |
| S-078 | AppStream metadata, AUR and winget (substories S-078a to S-078f); publication waits for owner accounts and credentials (exclusion R4-accounts) | In progress (2026-10-27): the winget half (S-078d) is submitted and passes all twelve Microsoft checks, waiting on one approving review from a moderator; the AppStream half (S-078a) and the AUR half (S-078b to S-078c) are not started, and the PR automation (S-078e) is not built |
| S-079 | Windows beta live readiness checklist (documentation and static tests only); the owner runs the live steps (exclusion R5-live) | Done (2026-10-26); the live steps stay owner run |
| S-080 | Windows `kawaro doctor` engine discovery matches engine lookup, a confirmed S-079 defect | Done (2026-10-26) |
| S-081 | Windows output placement: long paths and directory-name output candidates, a confirmed S-079 defect | Done (2026-10-26) |
| S-082 | Focus and activation with real Nautilus and Dolphin tokens in a virtual machine, closing the phase-0 gap recorded in S-013 | Draft |
| S-089 | First run and settings recovery from the owner Windows report: the language choice before the introduction, one settings window, and long-job notifications | Implemented on main (2026-10-27, e32ce3a); the live verification and the owner review are still pending |
| S-090 | Settings updates section: the installed version, a Check for updates button, Update now, and the three update keys (ADR 0006 and S-054b deferred these, and no story covered them). Owner priority: very important | Draft (2026-10-28) |
| S-091 | The daemon reports the real download percent and size in `Response::Update` (backward compatible JSON) so the S-090 state line stops showing 0%. After S-090 review, before S-092 | Draft (2026-10-28) |
| S-092 | Tray About item: the installed version (S-090 `about_line`), Check for updates and Settings, on the Linux tray and the Windows tray menu. After S-091 | Draft (2026-10-28) |
| S-093 | Windows settings window: default size, wide layout and layout jumps (live test findings of the S-090 build) | Superseded by S-094 (2026-10-29) |
| S-094 | Settings and welcome window as a real app shell: header with logo, name and version, page rail (General, Conversion, Notifications and integration, Updates, About), centred content column, fixed Save row, RTL, bounded Introduction film. Absorbs S-093. Owner priority: above S-091 and S-092 | Draft (2026-10-29) |
| S-094b | About page actions: website and source links through a safe `open_url`, copy version, check for updates shortcut. After S-094 | Draft (2026-10-29) |
| S-094c | Introduction film rebuilt: sharp at any scale, faithful four step flow, poster, play and pause, reduced motion fallback. After S-094 | Draft (2026-10-29) |

Measured facts behind S-075 (recorded 2026-10-26): Alt-only GlobalShortcuts is accepted by the BindShortcuts exchange but never activates on GNOME 50.0 or KDE 6.7.5 portals, so it cannot be the KDE or GNOME default; the Super+Z (`LOGO+z`) portal trigger passed the full real-drag path on both. Evidence: `/home/sbarah/.hermes/checkpoint-runs/kawaro-stable-20261026/alt-portal-spike.md`. Hyprland compositor binds are outside that measurement and count only when the private Hyprland harness records them.

Checkpoint "phase-0 closure" (S-082): the real Nautilus and Dolphin token focus validation needs a virtual machine and remains open until its report exists. It does not block the stories above, and it is never marked done by private compositor evidence alone.

S-072 "Launch assets and rename announcement" stays planned, owned by the site track (the owner of the S-056 to S-071 site stories). It is explicitly non-blocking for the stable code release and every story above unless the maintainer asks otherwise.

## Windows beta feedback (S-083 to S-088)

From the owner's first Windows beta run (2026-10-26): the SmartScreen warning, the missing first-run experience, the Alt conflict with Explorer's create-shortcut modifier, the missing app icon and bilingual name, and the silent Start menu entry. Written as stories on 2026-10-26 with a goldfish review (design, readiness and zoom-out) resolved into them.

| Story | Scope | Status |
| --- | --- | --- |
| S-083 | Signing pipeline and honest SmartScreen guidance; needs the maintainer's OV certificate | Done (2026-10-27); the 0.4.0 build is test-signed with the self-signed certificate, public signing waits for the OV certificate |
| S-084 | Windows drag trigger without the Explorer drag modifiers; spike first, default named by manager decision | Done (2026-10-27); the default is the right-button drag |
| S-085 | Windows app identity: icon in the exes, the name "Kawaro \| كوارو" everywhere | Done (2026-10-27) |
| S-086 | Clicking the app icon opens a window instead of nothing (the Start shortcut runs `kawarod settings`) | Done (2026-10-27) |
| S-087 | First-run welcome: demo, Try on an image, Settings entry | Done (2026-10-27) |
| S-088 | Download page: demo film, a sample to try, the honest warning | Done (2026-10-27) |

Landing order from the review: S-084 before or with S-076b (its D1 and D2 amend S-076 AC5 and AC6); S-087 and S-088 name no drag key before the S-084 manager decision line exists. The MSI `StartShortcut` row of `installer/windows/kawaro.wxs` is edited by S-085 (name and icon), S-086 (target command) and S-087 (behavior after the click): they land in that order and each updates the s051 table assertions in its own commit, so the parity test never breaks silently.

## 0.5.0 "X11"

Goal: the hive and drop work in an X11 session, with the hive at the pointer as `CLAUDE.md` says.

| Component | Scope |
| --- | --- |
| ui | The hive as an ordinary window at the pointer, adapting to screen edges, and the "hive first, then drag" drop order with an XDND receiver |
| Testing | A headless X server and a temporary HOME, never a live desktop |

Story: S-015 (draft). Out of scope: Windows, macOS.

## 0.6.0 "Windows"

Goal: a Windows 10 or 11 user selects a file in Explorer, picks Kawaro from the right click menu, the hive appears at the pointer, and the result is saved next to the original, in Arabic and English, with no file leaving the machine.

Before any code: ADR 0005 (`docs/adr/0005-windows.md`) settles the installer and its format, package signing, tray icon and notifications, where external engines live, and how to test on real Windows or a virtual machine.

| Component | Scope |
| --- | --- |
| core and daemon | IPC on a named pipe instead of a Unix socket, config and temp paths in `%APPDATA%` and `%TEMP%`, tray icon and notifications on Windows |
| ui | The hive as a frameless window at the pointer that takes focus, drop through an OLE receiver, the global shortcut |
| engines | The image engine as is; FFmpeg, LibreOffice and Ghostscript from Windows paths or downloaded on demand; kill the process tree on cancel (`crates/engines/src/process.rs` since S-030) |
| integrations/windows | The legacy registry menu first (unsigned), then the Windows 11 menu with IExplorerCommand and a signed sparse package |
| Distribution | An installer attached to the GitHub release, built in CI on a Windows runner |

Needs from the maintainer: a code signing certificate for the sparse package and the installer (without it only the legacy menu works and SmartScreen warns), and a Windows machine or VM for live checks.

Stories: numbered S-040 to S-052 (a block reserved on 2026-10-23, because main had already used S-022 and S-027), per ADR 0005 (proposed): S-040 (IPC over a named pipe), S-041 (paths), S-042 (service start), S-043 (engines), S-044 (build and CI), all done on Linux and the Windows cross build; S-045 (first ui story: cfg gates and the full build), S-046 (hive window and focus), S-047 (drop), S-048 (tray and shortcut), S-049 (notifications), S-050 (Explorer integration), S-051 (installer), S-052 (live checks and the 0.6.0 tag). The beta readiness block adds S-079 (the refreshed owner checklist for what actually shipped) and the two confirmed defect fixes S-080 and S-081, which land before the owner live run so the checklist steps they touch are trustworthy. Out of scope: macOS.

## Work order

1. Shared contracts in `crates/core` (action, job, engine and IPC types), then the workspace skeleton.
2. Parallel tracks that share no files: core, image engine, UI, integrations, identity and site.
3. Wire the service and CLI, independent review, tag `v0.1.0`.
4. 0.2 engines in parallel, then settings, review, tag `v0.2.0`.
5. 0.3.0: drop, layer and hive at the pointer, review, tag `v0.3.0`; follow-ups in 0.3.1 and 0.3.2.
6. 0.4.0: portal, signed packages and glass; then 0.5.0 for X11; then ADR 0005 and Windows stories for 0.6.0.
7. The S-072 to S-082 block runs alongside 0.4.0: S-075b and S-075d (the native spikes) come before any native registration, S-076 and S-077 in the Linux UX track, S-078 after the owner accounts exist (R4-accounts), S-080 and S-081 before the owner runs the S-079 live steps (R5-live), and S-082 in a virtual machine. S-072 never blocks this order unless the maintainer asks.
