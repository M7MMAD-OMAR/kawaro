# S-015 The hive on X11: an ordinary window at the pointer, and "hive first, then drag" drop
Status: split: S-113 (window at the pointer), S-114 (private X session harness and live checks), S-115 (drop cell, docs). Draft, awaiting the manager's approval (decisions below were given by the manager in the planning request). Release 0.5.0 "X11" (`docs/PLAN.md`).

This story is the parent plan. It carries no acceptance criteria of its own; each slice does. It builds on ADR 0003 (finding 7), ADR 0004 (X11 stays a winit window, the layer path is Wayland only), S-019 (`layout::plan_at`), S-046 and S-047 (the Windows window and drop, the pattern to copy).

## Story
As a user on an X11 session, I want the hive to appear at my pointer, adapt to the screen edges and take focus, and I want to show the drop cell and then drag a file onto it, so that Kawaro works as on Wayland and Windows.

## What the code does today (read, never run on X11)
- `layer::start()` returns false when `Connection::connect_to_env` fails, before `report_choice`, so no `hive surface choice:` line is printed on X11 although `overlay::surface_choice` already yields `not a Wayland session`.
- `report_window` assumes the compositor centres the window, `present_drop_at` ignores `at` on the window path, `attach_receiver` returns for a non Wayland handle. Nothing receives a drop on X11.
- Windows is the pattern: `win_window.rs` (cursor, `plan_at`, `set_position`), `dnd/windows.rs` (winit `HoveredFile` and `DroppedFile`, a `TICK_MS` pointer poll feeding the pure `DropTracker`). winit 0.30 has XDND on X11, so the X11 drop is the Windows drop with a different pointer source.
- `x11rb` 0.13.2 is in `Cargo.lock` through winit; `crates/ui` does not depend on it directly. softbuffer cannot draw transparency on X11.
- On this machine `/usr/bin` has Xvfb, xfwm4, xprop, xsetroot and dbus-run-session; there is no xdotool, Xephyr or xwininfo, so input and geometry come from a small x11rb tool (S-114), as `tests/drop/kwin-input` does for KWin.

## Decisions (settled by the manager, 2026-10-31)
1. **Corners: an opaque rectangle in `surface-0`, not the X Shape extension.** Reason: smallest change that fits ADR 0004 and the CLAUDE.md rule "where there is no blur surfaces stay opaque"; the window path on X11 has no compositor to blend. Shape (clip to the union of hexagons) needs rectangle decomposition and more live checks; it becomes a separate story only if the plate looks wrong in the S-114 screenshot.
2. **Drop cell position: centred on the screen**, as the layer path and GNOME do for a drop show. Reason: at the pointer it covers the file being dragged (S-019d decision), and centring reuses `overlay::place` with no new code. A normal hive (`open`, menu) is at the pointer.
3. **Highlight and cell choice during XDND: poll the pointer at `TICK_MS` through the existing `DropTracker`**, the same as S-047. Reason: winit gives no position during a drag, the poll is needed anyway to know the drop cell, so the highlight is free. No new mechanism.
4. **Session detection order: Wayland, then X11, then Windows**, from the environment (`WAYLAND_DISPLAY`, then `DISPLAY`) as a pure function. `DISPLAY` alone means X11; both set (XWayland inside Wayland) means Wayland, so Wayland behaviour never changes.
5. **Pointer on X11: `XQueryPointer` on the root window through x11rb (pure Rust, MIT or Apache 2.0, already in the lock) behind a small trait**, with a fake in unit tests. The real path is verified only in the private Xvfb session (S-114), run by hand at release, never on the live desktop.

## Slices (each fits one working session, in order, each ends with the fast loop)
| Story | Content | Needs a display |
| --- | --- | --- |
| S-113 | Session detection, pointer trait, pure placement at the pointer, opaque plate, the surface choice line on X11, x11rb as a direct dependency | no |
| S-114 | `tests/x11/run.sh` (Xvfb, xfwm4, skips when Xvfb is missing), the x11rb tool, live geometry, focus, Esc, budget, the manual release step | yes (private) |
| S-115 | Drop cell on X11 (winit XDND, copy only, always on top), README and drop-shortcut parity, CHANGELOG, ADR 0003 item | partly |

## Out of scope
Windows and macOS; XWayland inside a Wayland session; `drop --hold` on X11 (hold needs the key before the drag, the shortcut during a drag uses a passive grab later); several monitors and scale other than 1 on X11; window managers other than xfwm4 in tests (Mutter and KWin on X11); a real file manager as drag source; Shape clipping.

## Review log
- 2026-10-22, planner: first draft, three slices, effort about 3 days, four open decisions.
- 2026-10-31, planner: reworked after reading the Windows window and drop code. The Windows pattern removes the highlight and position work from the open decisions. Three decisions settled by the manager, plus detection order and pointer source. Slices re-cut so that S-113 needs no display. Draft awaiting approval of the manager.
