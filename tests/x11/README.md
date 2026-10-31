# X11 session checks (S-114)

`tests/x11/run.sh` starts a private X session and measures the hive in it, so the X11 path is checked for real without touching the desktop you work in. It is a manual release step: it is run by hand before the 0.5.0 tag, and its output is kept in the release notes. `cargo test` and CI never call it.

## Run it

```
cargo build --release
systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=100% nice -n 15 tests/x11/run.sh
```

When `Xvfb` or `xfwm4` is not on `PATH` the script prints `SKIP: Xvfb not found` (or `SKIP: xfwm4 not found`), exits 0 and starts nothing.

## What it starts

- `Xvfb` (1280 x 800 x 24) with `-displayfd`, and `xfwm4 --compositor=off`, inside `dbus-run-session`.
- A temporary HOME and `XDG_RUNTIME_DIR` under `$HOME/.cache`, removed on exit. `WAYLAND_DISPLAY` and `HYPRLAND_INSTANCE_SIGNATURE` are unset and `GDK_BACKEND=x11` is exported for everything it starts (xfwm4 does not start under a Wayland `GDK_BACKEND`).
- The `kawarod` and `kawaro` binaries of `target/release` (override with `KAWARO_BIN_DIR`), and `tests/x11/x11-tool`, a standalone crate built into `$HOME/.cache/kawaro-x11-tool-target`.
- On exit it ends only the process ids it started and prints whether any is still alive.

## What it checks

| Check | Measured |
| --- | --- |
| AC4 placement | the `hive surface: window X Y W H` log line against the geometry read from the X server, for three pointer positions, and that the window does not move |
| AC5 choice line | exactly one `hive surface choice: window, not a Wayland session` line after 20 shows |
| AC6 focus | `hive focus: true` within 300 ms of the first frame, with and without `DESKTOP_STARTUP_ID` |
| AC7 dismissal | Esc and a focus move each close the hive within 500 ms, with no new file and one `on_dismiss` |
| AC8 plate | the pixel at the window corner is `surface-0`, not the root colour, and a click there writes no file |
| AC9 budget | median first frame over 20 shows below 150 ms for open and drop (`crates/ui/tests/measure.sh` with `X11=1`), service resident memory at most 40 MB |

The pure helpers are in `tests/x11/lib.sh` and are tested by `tests/x11/s114.sh`, which needs no display.

## x11-tool

`tests/x11/x11-tool` has its own `Cargo.toml` and `[workspace]`, outside the Kawaro workspace. Subcommands: `move X Y` (XTEST, reads the pointer back), `key NAME` (XTEST; `button1` clicks), `geometry WINDOW_NAME` (prints `X Y W H` in root coordinates), `pixel X Y` (prints `RRGGBB`), `stacking` (window names, top to bottom).

## Drop check (S-115)

`tests/x11/drop.sh` starts the same private session and drags a PNG from the GTK 4 source `tests/drop/source.py` onto a hive cell after `kawaro drop`, with Shift held, 3 times. Each run checks that the hive stays above the raised source window, that the hovered cell turns accent within 100 ms, that `sample.webp` appears within 5 s, that the original's sha256 does not change and that the source reports `deleted=False`. It is also a manual release step.
