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
2. drop on a transparent corner: no new file, the hive closes, the source sees
   the drop refused;
3. drop on the WEBP cell with Shift held: the source reports `deleted=False`
   and the original's sha256 is unchanged (`source.py` deletes the original
   when a move is taken, as a file manager would);
4. drop on the centre cell: as 2.

Then, from the daemon's protocol trace: each show requests a
`wl_data_device` after the new hive surface's `get_xdg_surface` (Slint
destroys the surface on hide), `set_actions` offers copy only, and `accept`
names `text/uri-list` only.

The WEBP cell is located from the shipped layout rule (`crates/ui/src/layout.rs`,
`geometry.rs`, `design/tokens.json`): first cell of ring 1, top right of the
centre in the English layout.

## Input

- KWin: `kwin-input/`, a small client of `org_kde_kwin_fake_input` (copied
  from the spike, outside the workspace). KWin allows it only with
  `KWIN_WAYLAND_NO_PERMISSION_CHECKS=1`, which the script sets for its own
  private compositor.
- GNOME: `input_gnome.py`, Mutter's RemoteDesktop API on the private bus, with
  `keydown`/`keyup` to hold Shift at the drop.
