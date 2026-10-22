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
  instance, through `sbarhive_core::transport::Listener::bind`:
  - Unix: binding the socket gives `AlreadyRunning` if another daemon answers
    `ping` within 500 ms; any other socket file there is stale, removed and
    bound.
  - Windows: the pipe `\\.\pipe\io.github.m7mmad_omar.SbarHive.<SID>.<session>`.
    The first instance is created with `FILE_FLAG_FIRST_PIPE_INSTANCE`; when
    it fails with `ERROR_ACCESS_DENIED` a daemon already owns the name
    (`AlreadyRunning`). Later instances, one per accepted client, omit the
    flag. Every instance has a DACL for the current user alone and
    `PIPE_REJECT_REMOTE_CLIENTS`; the client checks that the server process
    runs as the same user before it writes, waits up to 2000 ms for a busy
    pipe, and a read timeout is a watcher thread that calls `CancelIoEx`.
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
  exclusive keyboard (none in drop mode). Its
  buffer is one transparent pixel from `wp_single_pixel_buffer_manager_v1`
  stretched over the output by `wp_viewporter`; a click on it closes the
  hive. The hive itself is a `wl_subcompositor` subsurface of the catcher,
  centred by `overlay::place`, holding the Slint frame in a `wl_shm` buffer
  scaled by its own `wp_viewporter` (fractional scale when offered). An
  In drop mode the catcher's input region is empty until the first
  configure, then exactly the hive rectangle, and the hive subsurface takes
  no input: the drop receiver binds to the catcher and subtracts the hive
  position before picking a cell, because Hyprland sends no drag motion to
  a subsurface of a layer surface (S-020). Outside the hive drags and
  clicks pass through. An
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
3. `hive_surface = "window"` in config.toml gives the window path, and
   `hive_surface = "layer"` the layer path when Wayland and those four
   globals are there. Absent means `auto`.
4. With `auto`, the layer path needs Wayland, those four globals and no
   running screen reader. A reader runs when `ScreenReaderEnabled` on
   `org.a11y.Status` is set and `org.gnome.Orca.Service` has an owner on the
   session bus, both watched there (S-019a): the flag alone is often left
   over from a GNOME setting. A running reader makes the next show a window,
   since the overlay has no accessibility tree; when it stops, the next show
   is a layer again.

Every change of the choice prints one `hive surface choice: KIND, REASON`
line, with no environment variable.

On Hyprland a layer rule for the `sbarhive` namespace can turn off its
animation; see `integrations/linux/hyprland/README.md`.

### Hive at the pointer (S-019, ADR 0004)

On the layer path the hive opens at an anchor instead of the output centre:

- **Anchor and wait.** The anchor is the first `wl_pointer.enter` or
  `motion` on the catcher or the hive before the first frame, waited for at
  most 50 ms from the catcher's first `configure`; with none the hive is
  centred. `sbarhive drop --at X,Y` (global logical coordinates, for
  example `$(hyprctl cursorpos)`) gives the anchor at once, on the output
  that holds the point. After the first frame the centre does not follow
  the pointer. With `SBARHIVE_DEBUG_FOCUS` every show prints
  `hive anchor: X Y` or `hive anchor: none`, then `hive cells: N CX CY`.
- **Adaptive shape.** `layout::plan_at` (pure, `crates/ui/src/layout.rs`)
  clamps the anchor so the centre cell stays on screen, then fills the slots
  of rings 1 to `MAX_RINGS` (4) in spiral order that fit on screen, ring 1
  cells first and their overflow ahead of ring 2 cells; the hive surface is
  the box around the centre and every accepted slot. Without an anchor it
  is `plan_rings` and `place` as before. Near a corner fewer than 18 cells
  may show.
- **Catcher phase of a drop show.** A drop show without `--at` takes its
  anchor from the drag: the catcher takes the input over the whole output
  until the first position (`wl_data_device.enter` or `motion`, or
  `wl_pointer` with no drag). This anchor phase never reaches the mode
  machine: no enter, leave or drop, and no offer is accepted. With no
  position within 50 ms the hive shows centred; a first position within
  300 ms of the first frame moves it once under the drag, after that it
  stays centred (manager decision 12). Then the catcher's region becomes
  the hive rectangle, so a still pointer is never held, and the keyboard is
  never taken.
- **Window path.** GNOME and a running screen reader keep the window,
  centred by the compositor, with no anchor line.

### Holding the drop key (S-019e, ADR 0003)

`sbarhive drop --hold` is the press of a held key and
`sbarhive drop --release` its release; both send one request and return at
once. The daemon feeds them to the long press gate (`sbarhive_core::hold`,
pure, with an injected clock), on the event loop thread:

- A press starts a timer of `hold_ms` (config.toml, 100 to 2000, default
  400), read from the file at every press so a change applies at the next
  one. Repeated presses keep the first press's timer and its `--at`
  position. The timer is ticked every 5 ms while it runs.
- When the timer ends with the key still held the gate shows the drop hive
  once, in hold mode (`Hive::present_drop_hold`); neither the press nor the
  release shows it. A release before the end cancels the timer, so a tap
  shows nothing.
- A release after the show closes a waiting hold hive, or one with the drag
  over a cell, and runs nothing (`Hive::key_released`); after a drop it does
  nothing. In a hold show the drag leaving the hive does not close it
  (manager decision 3), and the 10 s enter timeout still counts from the
  show.
- Whenever the hive closes for any reason (a drop, the timeout, Esc, a
  click), the daemon calls `Gate::hive_closed`, so a lost release never
  swallows the next press (manager decision 14).
- `Request::Drop` carries `hold`, false by default, and the release is
  `Request::DropRelease` (`{"cmd":"drop-release"}`). `--release` never
  starts the daemon; `sbarhive drop` without `--hold` stays the immediate
  show of S-014. A GlobalShortcuts portal (a later story) maps `Activated`
  and `Deactivated` to the same two gate inputs (`hold::from_portal`).
- Budget: on the hold path the 150 ms budget to the first frame is timed
  from the end of the threshold (`hive_bench show --drop --hold`,
  `tests/drop/budget.sh 20 1 hold`); the threshold is a user setting, not a
  response time. The immediate paths keep the request to first frame
  budget.

After a drag enters a drop hive (either kind) its centre shows the dragged
files by the S-018h rules (S-019f): one file's cached thumbnail or the kind
icon with the format code, the count for several; before enter the waiting
text and no preview. With `SBARHIVE_DEBUG_FOCUS` every enter prints one
`hive preview: thumb W H` or `hive preview: icon KIND` line.

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
sbarhive drop [--token T] [--at X,Y] [--hold | --release]
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
