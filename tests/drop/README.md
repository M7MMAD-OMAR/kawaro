# Drop tests (S-014)

A file dragged from a file manager, `sbarhive drop` run while the drag is
still going (what the keyboard shortcut does), and the file dropped on a hive
cell. ADR 0003 and the spike in `spikes/drop/` are the background.

Everything runs in a private headless compositor with a throwaway `HOME`, XDG
directories, `XDG_RUNTIME_DIR` and session bus, set before `dbus-run-session`,
as in `tests/focus/`. `DISPLAY`, `HYPRLAND_INSTANCE_SIGNATURE` and
`XDG_CURRENT_DESKTOP` are unset. Nothing touches the live desktop.

## Run

```
cargo build --release
cargo build --release --manifest-path tests/drop/kwin-input/Cargo.toml   # KWin only
tests/drop/kwin.sh             # KWin, focus stealing prevention Low (level=1)
tests/drop/gnome.sh            # Mutter, focus-new-windows=smart
tests/drop/kwin.sh shortcut    # S-014d AC11: Super+Z mid-drag, Esc
tests/drop/gnome.sh shortcut
tests/drop/budget.sh           # S-014d AC12: drop-mode first frame, 20 shows
tests/drop/docs.sh             # S-014d AC12: binding docs, no compositor
```

Each prints `PASS`/`FAIL` lines, ends with `RESULT PASS` or `RESULT FAIL (n)`,
and exits non-zero on any failure. Logs land in `tests/drop/out/` (ignored by
git; `DROP_OUT=dir` changes it).

## What a run does (S-014b: AC5, AC6, AC7)

The daemon starts with `WAYLAND_DEBUG=1` and `SBARHIVE_LANG=en`, with no
warm-up show. `source.py` (GTK 4, offers copy and move like Dolphin and
Nautilus) drags a copy of `tests/fixtures/images/alpha.png` in four shows:

1. drop on the WEBP cell: `photo one.webp` within 5 s, `file --mime-type` is
   `image/webp`;
2. over the WEBP cell, then drop on a transparent corner: the hive closes
   within 2 s of the release (not by the 10 s enter timeout), no new file,
   the source sees the drop refused;
3. drop on the WEBP cell with Shift held: the source reports `deleted=False`
   and the original's sha256 is unchanged (`source.py` deletes the original
   when a move is taken, as a file manager would);
4. drop on the centre cell: as 2.

Then, from the daemon's protocol trace: each show gets an `enter` on the
receiver's data device and requests a
`wl_data_device` after the new hive surface's `get_xdg_surface` (Slint
destroys the surface on hide), `set_actions` offers copy only, and `accept`
names `text/uri-list` only.

On KWin the hive is a layer surface (S-018): `kwin.sh` reads the hive's
rectangle from the daemon's `hive surface: layer X Y W H` line (the daemon
runs with `SBARHIVE_DEBUG_FOCUS=1`), counts it shown while there are more of
those lines than `zwlr_layer_surface_v1.destroy` requests, and takes the
show's new surface from `get_layer_surface`. It then runs the S-018d checks
in `layer.sh` (AC13, AC14) for each show: keyboard interactivity 0, an empty
input region on the catcher, one `get_data_device` after `get_layer_surface`
with every earlier data device released first, and the drag's `enter` on
this show's data device naming the hive subsurface. GNOME stays on the window
path (`get_xdg_surface`). `tests/overlay/kwin.sh drop` runs the same
`layer.sh` checks without a drag, plus a click at (100, 700) that must reach
the window below while the hive stays.

The WEBP cell is located from the shipped layout rule (`crates/ui/src/layout.rs`,
`geometry.rs`, `design/tokens.json`): first cell of ring 1, top right of the
centre in the English layout.

## S-014c: waiting for the drop (AC8, AC9, AC10)

Both scripts add a fifth show: `tests/fixtures/docs/one-page.pdf` (a one page
PDF written by hand) dragged in and dropped on a corner. Both run the daemon
with `SBARHIVE_DEBUG_FOCUS=1`.

- AC8: the daemon prints one stderr line per enter of a drag into a drop-mode
  hive, once the cells for the entered files are known and before any drop:
  `drop cells: ID ID ...` (cell ids in cell order, one space apart). In every
  show each such line must equal `sbarhive actions --json FILE` for that
  show's file, run in the same sandbox, and there must be at least one. The
  PNG and PDF lists must be non-empty and differ.
- AC9, AC10 (KWin only; the window path draws the same component, S-018 AC7):
  the daemon runs with `SBARHIVE_COLOR_SCHEME=light` and
  `SBARHIVE_REDUCED_MOTION=1`. In show 1, a ScreenShot2 shot is taken while
  the hive waits (before enter) and one while the drag is over the WEBP cell
  (`tests/overlay/shot.py`, started in the background because it writes its
  PNG only after 5 s). `pixels.py` checks the centre shows text-primary ink
  before enter (the waiting text), the WEBP cell is `color.light.accent` and
  the ring 1 cell across the centre is `surface-2` with no accent. The hive's
  box of each shot is written to `out/drop-en-light-waiting.png` and
  `out/drop-en-light-over.png` and compared (0.1 % of pixels or less, 8
  levels) with `snapshots/drop-en-light-waiting.png` and
  `snapshots/drop-en-light-over.png`. A missing reference is a FAIL: the owner
  approves a candidate from `out/` and it is copied to `snapshots/`.
- `source.py` uses a one pixel transparent drag icon, so the shots show the
  hive only.

## S-014d: the shortcut mid-drag (AC11, AC12)

`kwin.sh shortcut` and `gnome.sh shortcut` never run `sbarhive drop`
themselves. Super+Z is bound inside the private compositor to a wrapper that
writes one time line and execs `sbarhive drop`:

- KWin: a KDE custom command shortcut, as the docs describe it: a `.desktop`
  entry with `X-KDE-GlobalAccel-CommandShortcut=true` in the sandbox
  `applications` folder and `[services][net.local.sbarhive-drop.desktop]
  _launch=Meta+Z` in `kglobalshortcutsrc`, launched by the kglobalacceld in
  KWin. Keys go through `org_kde_kwin_fake_input` (evdev 125 + 44, Esc 1).
- GNOME: `global.display.grab_accelerator('<Super>z')` in the normal action
  mode through `Shell.Eval`, which spawns the wrapper (ADR 0003, result 4).
  gsd-media-keys custom shortcuts are out of scope (S-014). The overview must
  stay closed after Super+Z.

Three drags (one session) each press Super+Z mid-drag and drop a PNG on the
WEBP cell: the binding ran once, the hive appeared, `<name>.webp` is written,
`deleted=False`. Then Esc mid-drag twice, no file written in either:

- over the WEBP cell after `enter`: the source sees `drag-cancel`, and the
  daemon's trace shows the hive surface destroyed within 500 ms of the Esc
  (the trace stamps are UTC wall time, compared with `date -u` at the Esc);
- before the drag enters the hive: the compositor cancels the drag at the
  source and nothing reaches the hive (on the KWin layer path the hive takes
  no keyboard in drop mode, S-018 AC13; on GNOME Mutter's drag grab takes
  Esc), so it closes by its 10 s timeout (checked within 12 s).

On KWin no `wl_keyboard` enter or key may reach the daemon in those shows.

`budget.sh` runs `crates/ui/tests/measure.sh 20 1` with `DROP=1`
(`hive_bench show --drop`): every show line must say `mode=drop`, every show
is a layer surface that takes no keyboard (no `hive focus: true`), and the
median warm first frame is under 150 ms. `docs.sh` checks the GNOME, KDE and
Hyprland binding sections in `docs/`, in Arabic and in English.

## Input

- KWin: `kwin-input/`, a small client of `org_kde_kwin_fake_input` (copied
  from the spike, outside the workspace). KWin allows it only with
  `KWIN_WAYLAND_NO_PERMISSION_CHECKS=1`, which the script sets for its own
  private compositor.
- GNOME: `input_gnome.py`, Mutter's RemoteDesktop API on the private bus, with
  `keydown`/`keyup` to hold Shift at the drop.
