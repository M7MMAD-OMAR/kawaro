# Architecture

Contracts between crates and tracks. Code comments repeat the parts that
matter locally; this file is the cross-crate view. English, because it is
read mostly by agents and contributors writing code.

## Processes

```
file manager entry ──▶ sbarhive open ──▶ (Unix socket, JSON lines) ──▶ sbarhived
  Dolphin / Nautilus      CLI, starts        $XDG_RUNTIME_DIR/            hive window (Slint)
  Nemo / Thunar           the daemon if      sbarhive.sock                job queue + engines
  Hyprland bind           it is not up                                    tray icon, notifications
```

- `sbarhived` (crates/daemon) is the only process that shows UI. Single
  instance: binding the socket fails if another daemon answers `ping`; a stale
  socket file is removed.
- `sbarhive` (crates/cli) is a thin client, plus `convert` which runs engines
  in-process with no daemon.
- Engines (crates/engines) run inside the daemon. Heavy external tools
  (FFmpeg, LibreOffice, Ghostscript, v0.2) run as child processes.

## Core contracts (crates/core)

- `Engine` trait, `Task`, `CancelToken`, `EngineError`: `engine.rs`.
- `ActionDef`, `Registry`, TOML schema: `action.rs`. Built-in actions live in
  the repository `actions/` folder and are embedded with `include_str!`. User
  actions load from `~/.config/sbarhive/actions/*.toml` and may override a
  built-in id.
- Output naming and atomic finalisation: `output.rs`. Engines write
  `task.output` (temp file in the destination directory); the queue renames.
- Queue and events: `queue.rs`. IPC messages: `ipc.rs`.

## Image engine operations (engine = "image")

| op | params | inputs | notes |
| --- | --- | --- | --- |
| `convert` | `quality` 1..100 (lossy formats) | 1 | target format from `task.output` extension: png jpg webp avif gif bmp tiff ico |
| `resize` | `scale` percent, or `max_side` px | 1 | keeps aspect ratio, Lanczos3 |
| `rotate` | `degrees` 90, 180, 270 | 1 | |
| `flip` | `axis` "horizontal" or "vertical" | 1 | |
| `grayscale` | none | 1 | |
| `compress` | `quality` | 1 | same format; PNG is re-encoded with best compression |
| `strip` | none | 1 | re-encode without metadata, same format |
| `to-pdf` | `page` "fit" | n | combined: one page per image, image size as page size |

All ops apply EXIF orientation before processing. Alpha flattens onto white
for formats without alpha (JPEG, BMP).

## UI contract (crates/ui)

```rust
/// `icon` names an SVG of the built-in set in crates/ui/icons/ (`icon_names()`);
/// empty or unknown shows the code alone.
pub struct HiveCell { pub id: String, pub code: String, pub label: String, pub icon: String, pub ring: u8 }

/// Selects winit with the software renderer, installs the activation-token
/// hook and the focus watcher. Call once, before creating any window.
pub fn init() -> Result<(), slint::PlatformError>;

pub struct Hive { .. }
impl Hive {
    pub fn new(lang: Lang) -> Result<Self, slint::PlatformError>;
    /// Lays out up to 6 cells in ring 1 and 12 in ring 2 (overflow from ring 1
    /// moves to ring 2, beyond 18 is dropped), sets the centre text, and shows
    /// the window centred, activating it with `token` when given.
    pub fn present(&self, centre: &str, cells: Vec<HiveCell>, token: Option<String>) -> Result<(), slint::PlatformError>;
    pub fn close(&self);
    pub fn on_choose(&self, f: impl Fn(String) + 'static);   // action id, hive already closed
    pub fn on_dismiss(&self, f: impl Fn() + 'static);        // Esc, focus loss, click outside or on the centre
}
```

Rules from the spike (ADR 0001): software renderer, embedded IBM Plex Sans
Arabic, colours and sizes generated from `design/tokens.json` at build time,
theme follows the system colour scheme, Arabic mirrors the layout.

## Settings

`~/.config/sbarhive/config.toml`, model in `crates/core/src/config.rs`:
language, theme, output place (beside the original, or a subfolder whose
name the daemon puts in `OutputSpec.dir`), quality preset (shifts the
`quality` parameter by -15, 0 or +10, clamped to 1..100), notify, open the
folder when done. The daemon reads it at start and on `reload`; the settings
window (v0.2, S-011) writes it.

## Daemon behaviour

- `open`: match actions; none gives a notification, otherwise show the hive.
  Choosing submits a job and closes the hive at once.
- Progress never takes focus: tray icon tooltip and status while jobs run, one
  desktop notification when a job ends (counts, first error, "Open folder").
- Tray menu: Cancel all, Settings (v0.2), Quit.
- Started by the systemd user unit `sbarhive.service`
  (`WantedBy=graphical-session.target`), or on demand by the CLI.

## CLI

```
sbarhive open [--token T] [--from-clipboard] [--] FILES...
sbarhive convert --to EXT [--quality N] FILES...      # in-process, no daemon
sbarhive run ACTION_ID [--wait] FILES...
sbarhive actions FILES...                             # id, ring, code, label
sbarhive daemon                                       # exec sbarhived
sbarhive ping | quit
```

`open` reads `XDG_ACTIVATION_TOKEN` and `DESKTOP_STARTUP_ID` when `--token`
is absent. `--from-clipboard` reads `text/uri-list` with `wl-paste` on
Wayland or `xclip` on X11. If the socket does not answer, `open` starts
`sbarhived` detached and retries for up to 2 seconds.

## File manager integrations (integrations/linux)

Each integration only collects the selected paths (and an activation token
where the file manager can give one) and runs `sbarhive open`. No logic.
- Dolphin: service menu with `X-KDE-Priority=TopLevel`.
- Nautilus: nautilus-python `MenuProvider`; token from
  `Gdk.AppLaunchContext.get_startup_notify_id`.
- Nemo: `.nemo_action`. Thunar: custom action merged into `uca.xml`.
- Hyprland: a bind that sends Ctrl+C to the active window, then
  `sbarhive open --from-clipboard`.
