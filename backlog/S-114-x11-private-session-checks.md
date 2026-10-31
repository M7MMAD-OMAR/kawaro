# S-114 A private X11 session for Kawaro: harness, live geometry, focus, budget
Status: draft

## Story
As the maintainer, I want one script that starts a private X session and measures the hive in it, so that the X11 path is verified for real without touching my desktop. Slice 2 of S-015, after S-113. The script is optional: it is a manual release step, and skips when Xvfb is missing.

## Acceptance criteria
- AC1 (skip): when `Xvfb` or `xfwm4` is not on `PATH`, `tests/x11/run.sh` prints `SKIP: Xvfb not found` (or `xfwm4`), exits 0 and starts no process. `cargo test` never calls it.
- AC2 (isolation): the script starts `Xvfb -displayfd` (1280 x 800 x 24) and xfwm4 with `--compositor=off`, inside `dbus-run-session`, with a temporary HOME and `XDG_RUNTIME_DIR` under `$HOME/.cache`, and with `WAYLAND_DISPLAY` and `HYPRLAND_INSTANCE_SIGNATURE` unset. No `wayland-*` file exists in that runtime dir at any 1 s check. On exit the script ends only the process ids it started; 0 of them are alive afterwards, and the temporary directory is gone.
- AC3 (tool): `tests/x11/x11-tool` is a standalone Rust crate outside the workspace (own `[workspace]` table, x11rb only, own target dir under `$HOME/.cache`) with subcommands `move X Y` (XTEST), `key NAME`, `geometry WINDOW_NAME`, `pixel X Y`, `stacking`. `move 400 500` followed by `XQueryPointer` reads back (400, 500).
- AC4 (placement live): after `move X Y` and `kawaro open FILE`, the log line `hive surface: window X Y W H` equals the geometry the tool reads from the X server (`TranslateCoordinates` to the root) within 1 px, for pointers (400, 500), (0, 0), (1279, 799). The window box is inside 1280 x 800 each time and the window does not move after its first frame (2 reads 500 ms apart equal). On X11 the line reports the real position, not the assumed centre; Wayland output is unchanged.
- AC5 (choice line): the service log has exactly 1 `hive surface choice: window, not a Wayland session` after 20 shows.
- AC6 (focus): `hive focus: true` appears within 300 ms of the first frame, with no token and with `DESKTOP_STARTUP_ID` set, 3 of 3 runs each.
- AC7 (dismiss): `Esc` through the tool closes the hive within 500 ms; moving focus to a test window closes it within 500 ms; both leave 0 new files and exactly 1 `on_dismiss`.
- AC8 (plate): with xfwm4 `--compositor=off`, `xsetroot -solid` set to a colour that is not `surface-0`, the pixel at the window box corner equals `surface-0`, not the root colour, and a click there reaches no cell (0 files written).
- AC9 (budget): `crates/ui/tests/measure.sh` with `X11=1` reports a median over 20 shows below 150 ms from the request to the first frame for `open` and for `drop`; service RSS after those 20 shows is 40 MB or less (read from `/proc`).
- AC10 (manual step): `tests/x11/README.md` and `docs/PLAN.md` (0.5.0 table, one line) state that `tests/x11/run.sh` is run by hand before the 0.5.0 tag and that its output is kept in the release notes; a test checks the README names the command and the SKIP rule.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | Run the script with `PATH` holding no Xvfb; assert stdout line, exit 0, 0 children | integration |
| AC2 | Script polls the runtime dir each second, counts started pids after exit, checks the temp dir | integration |
| AC3 | Tool moves the pointer and reads it back | integration |
| AC4 | Three pointers; compare the log line to the tool geometry, then read twice | integration |
| AC5 | Count the line in the service log | integration |
| AC6, AC7 | Read `hive focus:` with its timestamp; `Esc` and a focus move through the tool, 3 runs | integration |
| AC8 | Tool `pixel` at the box corner against the `surface-0` value; a click and a file count | integration, snapshot |
| AC9 | `measure.sh X11=1`; RSS from `/proc` | performance |
| AC10 | Test reads the README and the PLAN line | unit |

## Out of scope
The drop cell (S-115); window managers other than xfwm4; a live desktop; CI (this script is not in CI).

## Risks to check first
Whether xfwm4 honours the requested position of an undecorated window, and whether winit under Xvfb reports `hive focus: true`. If either fails the tester stops and reports to the manager before AC4 or AC6 are written.

## Review log
- 2026-10-31, planner: split from S-015 (old AC1, AC4 to AC8). The script is a manual release step and optional by design. Draft awaiting the manager.
