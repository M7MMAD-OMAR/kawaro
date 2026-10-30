# S-105 Hyprland 0.57: a drop show without --at takes the first click
Status: approved (2026-10-31, manager)

## Story
As a Hyprland 0.57 user whose drop key has no `--at` (portal shortcut, a plain bind), I want a click made while the hive is armed to reach the file manager, so that the first click of a drag start is not swallowed.

## Problem
`kawaro drop` or `drop --hold` without `--at` maps a catcher whose first input region is the whole surface (the drag can start anywhere, so it must be). Hyprland 0.57 enters a layer surface at once under a still pointer and does not refocus when the region later shrinks to the hive rectangle (S-104). The pointer focus stays on the catcher and the first press is taken. The Alt bind passes `--at` and is not affected (S-104 hole). KWin and GNOME do not enter on map.

Measured (private Hyprland 0.57.0 harness): S-021 AC5 (b), pointer still at (40, 40), `kawaro drop`, a click 0.9 s later reaches keys.py 0 times (expected 1); 0.56.2 passed. S-022 AC4 (reduced motion, 3 attach in 200 ms) also fails; it has no pointer enter, is not this effect, and is an unrelated flake for a separate story.

## Options (reasoned from the code)
- Rejected before: a hole at the map time enter (no refocus on a region change), an empty region (the drag enter cannot land), a delayed full region (the growth enters again), `hyprctl cursorpos` as `at` (the hive anchors away from the drag's first `wl_data_device.enter`, Hyprland ends the drag without `drop`, decision 12 turns red).
- Rejected now: widening an empty region on the first real event (circular: an empty catcher gets no pointer, motion or drag event, and Wayland gives no pointer position outside our surfaces). `keyboard_interactivity` (keyboard only, it does not touch pointer focus). A re-injected click (needs a virtual pointer or uinput, privileged, not portable). A catcher sized to the hive at the screen centre (no first region of full size: drags are only seen inside the centre rectangle, so the hive can no longer appear at the drag position, S-019, and a pointer inside the rectangle still enters on map).
- Chosen: remap once at the map time enter position. The enter is the pointer position (`ProbeState` already reports it as `MapEnter`, S-104), and an enter proves no drag holds the pointer (S-021). So `Hive` destroys the catcher and maps a new one exactly as a show with `--at` at that position: first region is the hive rectangle minus the 6 px hole, no enter, the press reaches the window below. This reuses the measured S-104 path and the S-095 re-show. A drag that is already running holds the pointer, gets no map time enter, and never remaps, so the drag first order is untouched (the cursorpos failure came from anchoring a running drag). Cost: one extra transparent surface per still pointer show; assumed, to be measured first, that Hyprland refocuses the window below when the entered layer surface is destroyed.

## Acceptance criteria
- AC0 (spike first, half a day, no merge): in the private 0.57.0 harness, destroying the entered catcher and mapping the `--at` form at the enter position lets a click 0.9 s later reach keys.py 1 of 1 time, in 10 of 10 runs. If it is below 10 of 10 the story stops and the manager picks the documented known limit (as today).
- AC1 (decision, pure): `remap_at(drop, had_at, revealed, remapped, map_enter, origin)` returns `Some(global point)` only when drop is true, had_at false, revealed false, remapped false and a map time enter exists; the point is the enter position plus the output origin. Every other input returns `None`.
- AC2 (once): a show remaps at most once; a second map time enter on the new catcher cannot occur (its hole holds the pointer) and a forced one returns `None`. A real enter later than 250 ms hides as before (`hidden (pointer)`).
- AC3 (budget): from the map time enter to the new catcher's ready commit 30 ms or less; the existing 150 ms budget from the first `wl_data_device.enter` to the first hive attach holds (median of 20 in `tests/drop/budget.sh`, under 150 ms).
- AC4 (output): per drop show without `--at` on a still pointer: exactly 1 `hive probe:` or `hive reveal:` line, 2 `get_layer_surface` calls (3 or more is a fail), the first destroyed before the second maps.
- AC5 (harness, private Hyprland 0.57.0): S-021 AC5 (b) asserts as on 0.56.2 with the S-105 SKIP removed: keys.py clicks 1, catcher still bound, then a move into the rectangle gives `hidden (pointer)`. `repro.sh --altdrag layer` passes in full with 0 SKIP for AC5 (b); S-022 AC1 second drag and mid-drag drop on the WEBP cell still write the file; alt-first and drag-first stay 20 of 20 each.
- AC6: S-019 decision 12 holds on the surviving catcher: its first region is one add equal to the hive rectangle, within 400 ms of its ready commit.
- AC7: fmt, clippy -D warnings, tests of kawaro-ui, design-lint, no long dash, one CHANGELOG line in English and Arabic.

## Test cases
| AC | Test | Type |
| --- | --- | --- |
| AC1 | table in `layer.rs` tests: drop+no at+map enter (40, 40), origin (1920, 0) gives `Some((1960, 40))`; non drop, had_at, revealed, remapped, no enter each give `None` | unit |
| AC2 | sequence on `ProbeState` plus the remap flag: enter at 0 ms gives remap once; enter at 0 ms again gives none; enter at 300 ms gives `PointerEnter` and no remap; drag enter without a pointer enter gives none | unit |
| AC0, AC5 | `tests/hyprland/repro.sh` S-021 AC5 (b) and `--altdrag layer`: keys.py click count, `bstay`, probe line text | harness |
| AC3, AC4 | trace timestamps and counts of `get_layer_surface` and probe lines per show; `tests/drop/budget.sh` median | harness, perf |
| AC6 | `still_region` reads the last catcher in the trace; asserts `1 $sreg` and within_ms 400 | harness |

The harness runs are a manual release step on a private compositor, not a fast-loop gate. Before release also run `tests/overlay/kwin.sh drop` and `tests/drop/kwin.sh level=1` (shared pointer code; same pass counts as before).

## Out of scope
- S-022 AC4 flake. KWin, GNOME, X11, Windows behaviour. Multi screen precision (the point is used as output coordinates, as `--at` does before an output is known). Asking upstream for an opt out of the enter on map.

## Review log
- 2026-10-31 AC0 spike result: PASS. Private nested Hyprland 0.57.0 harness (the S-021 AC5 (b) steps of `tests/hyprland/repro.sh`, looped 10 times: pointer still at (40, 40), `kawaro drop` without `--at`, a click 0.9 s later). Baseline on the unpatched daemon: keys.py received the click 0 of 10 times, one layer surface per show. With a temporary local patch behind an env switch (on the catcher's map time enter in a drop show with no `at`, unmap the catcher and map a new one with `at` set to the enter position plus the output origin, so the first region is the hive rectangle minus the 6 px hole, as the `--at` path does; once per show): the click reached keys.py 10 of 10 times, the catcher was still bound after the click in 10 of 10, and each show made exactly 2 `get_layer_surface` calls. The patch was not committed and has been reverted. The assumption holds, so the story can go ahead.
- 2026-10-31 tester: tests written from AC1 and AC2 in `crates/ui/src/s105_tests.rs` (registered in `crates/ui/src/lib.rs`): 7 tests (`ac1_*`, `ac2_*`: the five-input None table, the 32 combination sweep with exactly one Some, the origin addition, remap once, late enter at 300 ms, drag with no pointer enter, a move after the remap). `rustfmt --check` on the file is clean. Clippy cannot run before the hooks exist (the file does not compile); the implementer runs it.
  Failing output (`cargo test -p kawaro-ui --lib s105`, fast loop scope, nice, 1 job): `error[E0432]: unresolved import crate::layer::remap_at`, `error[E0603]: enum Probe is private`, `error[E0603]: struct ProbeState is private`, then `could not compile kawaro-ui (lib test) due to 3 previous errors`. Right reason: the decision function does not exist and the probe types are private.
  Hooks the implementer must create in `crates/ui/src/layer.rs`: `pub(crate) fn remap_at(drop: bool, had_at: bool, revealed: bool, remapped: bool, map_enter: Option<(f64, f64)>, origin: (i32, i32)) -> Option<(f64, f64)>` (Some(enter + origin) only for drop, no at, not revealed, not remapped, enter present); make `Probe`, `ProbeState` and `ProbeState::{enter, motion, leave}` `pub(crate)` (Probe keeps Debug and PartialEq; ProbeState keeps Default). The adapter keeps the `remapped` flag per show and calls `remap_at` on `Probe::MapEnter`.
  Harness edits (shell, not run, manual release step): `tests/hyprland/repro.sh` S-021 AC5 (b) lost the S-105 SKIP and version gate, the check asserts as on 0.56.2; `tests/drop/layer.sh` `still_region` now reads the last catcher of the segment (cut at the last `get_layer_surface` line), which is the AC6 check; with one catcher the output is unchanged. `bash -n` passes on both. AC3, AC4, AC5, AC6 stay harness steps; AC7 scans (CHANGELOG line, design-lint) are the implementer's.
