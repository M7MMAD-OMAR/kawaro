# S-116 The X11 hive is placed where the plan says
Status: approved (2026-10-31, manager)

## Story
As a user on X11, I want the hive to open at my pointer, not at the screen centre. S-114 found that under xfwm4 the hive opens at the centre for every pointer while S-113 plans a different position per pointer, and that the `hive surface: window` line printed an assumed centre.

## Acceptance criteria
- AC1: after `show` on an X11 session with a pointer source, `window().position()` equals `Hive::x11_position()` and equals the S-113 plan, for pointers (400, 500), (0, 0), (1279, 799); the three positions differ.
- AC2: on X11 the line `hive surface: window X Y W H` is built from the planned or real window position (`x11::surface_line`), never from the assumed centre of the monitor; `Hive::x11_surface_line()` gives the planned line. Wayland and Windows output is unchanged.
- AC3: a window that the window manager placed elsewhere is asked once (`set_outer_position`) to move to the plan after it is mapped; the line is printed only once the window is at the plan, or with whatever position the manager chose after the move request (`x11::PlaceCheck`). Linux X11 only, cfg-gated.
- AC4 (live, `tests/x11/run.sh`): the three pointers give three different server geometries within 1 px of the plan, the line equals the server geometry within 1 px, and the window does not move after its first report.
- S-074, S-105, S-110, S-113 tests stay green.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | `crates/ui/src/s116_tests.rs` the window position equals the plan at three pointers | unit |
| AC2 | line equals `surface_line` of the planned position; none off X11 | unit |
| AC3 | `PlaceCheck` sequences | unit |
| AC4 | `tests/x11/run.sh` AC4 | live |

## Out of scope
Window managers other than xfwm4; the X11 drop path (S-115).

## Review log
- 2026-10-31, manager: found by S-114. Implementer: test first (`s116_tests.rs`), then the fix.
- 2026-10-31, implementer diagnosis (live, private Xvfb and xfwm4): `xprop WM_NORMAL_HINTS` of the hive window shows only `user specified size` and no position hint, so xfwm4 centres the window; the position set before `show()` never reaches the window creation hints. The old line used `report_window`, which assumes a centred window.
