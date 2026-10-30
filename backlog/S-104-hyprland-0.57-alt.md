# S-104 Hyprland 0.57: a map time pointer enter killed the Alt hive
Status: review

## Story
As a Hyprland 0.57 user holding Alt, I want the hive to stay up until my drag arrives, so that Alt first and then a drag works as it did on 0.56.

## Finding (measured 2026-10-30)
- The S-021 and S-095 harness checks that passed on ffa916c in the morning fail on a clean ffa916c build after `dnf upgrade` (Hyprland 0.56.2 to 0.57.0, built 2026-10-28). The harness config loads without error on 0.57.
- Cause: Hyprland 0.57 sends `wl_pointer.enter` to a layer surface the moment it maps under a still pointer (0.56.2 did not until the pointer moved). Daemon log, one failing show: first catcher commit at .113685, `wl_pointer.enter` at (39.75, 39.75) at .113891, `hive probe: hidden (pointer)` at .113977. `dnd::Machine` treats `PointerEnter` in `Probing` as "no drag, hide", so every show with a still pointer died at map.
- Effect on the real desktop: Alt held first and the drag beginning after `hold_ms` shows nothing (the S-095 "does nothing" case is back, the 4 re-shows of S-095 each die at map: 5 `hive surface` lines in under a second, also for Esc and for Alt chords). Drag first and then Alt still works, because during a drag the pointer is grabbed and the catcher gets the data device enter.
- Not an upstream bug: entering the surface under the cursor on map is ordinary Wayland compositor behaviour. The assumption "an enter means the pointer moved" was the defect.
- A second effect of the same mechanism, which the harness cannot see: a catcher that took pointer focus on map also takes the next button press, so the file manager would never start the drag.

## Acceptance criteria
- AC1: the first `wl_pointer.enter` on the catcher within 250 ms of its map is a map time enter: it does not raise `PointerEnter`; the position is the baseline. A later or second enter is a real pointer and hides as before (KWin and 0.56 behave as today).
- AC2: motion on the catcher 3 px or more from the baseline raises `PointerEnter` once (a 5 px move still hides, S-021 AC9 (1)); less than 3 px does not (39.75 against 40 at fractional scale).
- AC3: a re-show of the same press (S-095) leaves its hole at the last pointer position the catcher saw, not at the position of the first press. The drop show ignores the map time enter for placement (as on 0.56); a normal show still anchors on it.
- AC3b: an unrevealed `--at` catcher leaves a 6 px square around the anchor out of its input region, so a still pointer never takes focus there and the press reaches the file manager; the hole is cut to the hive rectangle and goes away at reveal.
- AC4: harness `tests/hyprland/repro.sh --altdrag layer` passes in full on Hyprland 0.57.0 with no check weakened.
- AC5: fmt, clippy -D warnings, tests of kawaro-ui, design-lint, no long dash, one CHANGELOG line in English and Arabic.

## Tests
Unit, in `crates/ui/src/layer.rs` (`pointer_probe_tests`): the three pure helpers `is_map_time_enter`, `moved_from`, `probe_hole`. Harness: the existing checks (they were the red before the fix: 8 FAIL).

## Out of scope
KDE, GNOME, X11, Windows. The hole is not proven against a real file manager drag (the harness has no drag source); that is a test for the owner on the live desktop.

## Review log
- 2026-10-30, one pass (planner, tester, implementer by one agent, the chain was not split): measured and fixed as above. The red was the existing harness (8 FAIL on HEAD), the helper tests were written with the code.
- 2026-10-30, result (private Hyprland 0.57.0 harness, `repro.sh --altdrag layer`, release build, CPUQuota=100%, load about 7 from unrelated jobs): before the fix RESULT FAIL (8): S-021 AC5 (b), AC5 (c) x2, AC9 (1) x2, decision 12, S-095 AC2 alt-first (gap 600: 0 of 10), Esc (5 surface lines). After: RESULT FAIL (1). alt-first 20 of 20 (gap 50 and gap 600), drag-first 20 of 20, 0 hangs, Esc 1 surface line, 2 s motion 4 lines, Alt+a 0 lines and `hold: chord`.
  - Still failing: S-021 AC5 (b) (`kawaro drop` with no `--at`, pointer still at (40, 40) outside the centred hive, a click 0.9 s later must reach keys.py). On 0.57 the catcher took pointer focus at map, with the full input region of the unswitched show, and Hyprland does not refocus when the region later shrinks to the hive rectangle, so the first click is taken by the catcher. The Alt bind passes `--at` and is not affected (its hole keeps focus off the catcher). The portal shortcut path without `--at` loses at most the first click in the first second of a still pointer. No fix without an empty region, which would stop the drag from being detected anywhere; left for the manager.
  - The Lua hook `hl.on("input.keyboard.key")` of S-102 loads on 0.57 with no error in the Hyprland log, and fires (`hold: chord` after Alt+a).
  - Not run: the KWin drop scripts (`tests/overlay/kwin.sh`, shared pointer code); the real file manager drag with the hole (harness has no drag source).
