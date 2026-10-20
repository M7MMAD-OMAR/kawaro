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
    /// the window centred, activating it with `token` when given. A token
    /// builds a fresh window component, because Slint hands it to winit only
    /// when a window adapter is created (S-013).
    pub fn present(&self, centre: &str, cells: Vec<HiveCell>, token: Option<String>) -> Result<(), slint::PlatformError>;
    /// Empty two-ring hive waiting for a dragged file (S-014, ADR 0003). On
    /// Wayland a drop receiver binds a new wl_data_device to each new hive
    /// surface, accepts text/uri-list over the cells only, copy only, and
    /// reads the files when the drag enters.
    pub fn present_drop(&self, centre: &str, token: Option<String>) -> Result<(), slint::PlatformError>;
    pub fn close(&self);
    pub fn on_choose(&self, f: impl Fn(String) + 'static);   // action id, hive already closed
    /// Drag entered the drop hive with these files: centre text and cells (none refuses the drop).
    pub fn on_drop_files(&self, f: impl Fn(Vec<PathBuf>) -> (String, Vec<HiveCell>) + 'static);
    pub fn on_dismiss(&self, f: impl Fn() + 'static);        // Esc, focus loss, click outside or on the centre
}
```

Rules from the spike (ADR 0001): software renderer, embedded IBM Plex Sans
Arabic, colours and sizes generated from `design/tokens.json` at build time,
theme follows the system colour scheme, Arabic mirrors the layout.

### Hive surface: layer path and window path (S-018, ADR 0004)

The hive is drawn on one of two surfaces, chosen again at every hive build
by `overlay::choose_surface` (`crates/ui/src/overlay.rs`, pure) from the
globals the compositor advertises, the screen reader state and the
`SBARHIVE_SURFACE` environment variable:

- **Layer path** (`crates/ui/src/layer.rs`): a Slint window adapter with the
  software renderer on its own wayland-client connection. Each show creates
  a catcher, a `zwlr_layer_shell_v1` surface on the overlay layer with the
  namespace `sbarhive`, anchored to every edge, exclusive zone -1 and
  exclusive keyboard (none in drop mode, whose input region is empty). Its
  buffer is one transparent pixel from `wp_single_pixel_buffer_manager_v1`
  stretched over the output by `wp_viewporter`; a click on it closes the
  hive. The hive itself is a `wl_subcompositor` subsurface of the catcher,
  centred by `overlay::place`, holding the Slint frame in a `wl_shm` buffer
  scaled by its own `wp_viewporter` (fractional scale when offered). An
  activation token goes to `xdg_activation_v1.activate` with the first
  frame. On hide every surface, the `zwlr_layer_surface_v1` and every
  buffer of the show not yet released are destroyed together, so no
  Wayland surface or buffer of a hidden hive stays alive (AC18).
- **Window path**: the winit backend, an ordinary `xdg_toplevel`
  (xdg-shell) window centred by the compositor, as before S-018. It is
  used on GNOME, X11 and Windows, and wherever the layer path cannot be.

Choice rule:

1. `SBARHIVE_SURFACE=window` always gives the window path.
2. `SBARHIVE_SURFACE=layer` gives the layer path, even with a screen reader,
   when the compositor has all four of `zwlr_layer_shell_v1`,
   `wl_subcompositor`, `wp_viewporter` and `wp_single_pixel_buffer` (the
   `wp_single_pixel_buffer_manager_v1` global); otherwise a window, with a
   log line naming what is missing.
3. With no override, the layer path needs Wayland, those four globals and no
   screen reader. A running screen reader (`ScreenReaderEnabled` on
   `org.a11y.Status`, watched on the session bus) makes the next show a
   window, since the overlay has no accessibility tree; when it stops, the
   next show is a layer again.

On Hyprland a layer rule for the `sbarhive` namespace can turn off its
animation; see `integrations/linux/hyprland/README.md`.

## Settings

`~/.config/sbarhive/config.toml`, model in `crates/core/src/config.rs`:
language, theme, output place (beside the original, or a subfolder whose
name the daemon puts in `OutputSpec.dir`), quality preset (shifts the
`quality` parameter by -15, 0 or +10, clamped to 1..100), notify, open the
folder when done. The daemon reads it at start, on `reload`, and when the
file or a user action file changes (polled every 2 s, `crates/daemon/src/watch.rs`);
the settings window (v0.2, S-011) writes it. Output naming is fixed: same
name with the new extension, " (1)", " (2)" on collision, never an overwrite.

## Daemon behaviour

- `open`: match actions; none gives a notification, otherwise show the hive.
  Choosing submits a job and closes the hive at once.
- `drop`: show the drop hive; when a drag enters, match actions for its files
  and keep them as pending, so a drop on a cell runs like a click. A drop off
  the cells or on the centre, a `leave`, or 10 s without a drag closes it.
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
