# S-115 The drop cell on X11: show the hive, then drag the file onto it
Status: draft

## Story
As an X11 user, I want to run `kawaro drop`, drag a file onto a cell and get the result next to the original, so that "hive first, then drag" works as on Windows. Slice 3 of S-015, after S-113 and S-114 (decisions 2 and 3 there).

## Acceptance criteria
- AC1 (drop position): on X11 `Hive::present_drop_at` ignores `at` and places the drop hive with `overlay::place` (centred on the 1280 x 800 root); the pure choice function returns the centred placement for `at = Some(p)` and `None`, both equal. A normal hive show (S-113) is unchanged.
- AC2 (always on top): the X11 window spec of a drop show has level always on top; the spec of a normal show keeps S-113 values. After a source window is raised by a click, the hive stays above it in `_NET_CLIENT_LIST_STACKING` (live, S-114 tool).
- AC3 (event adapter): a new `dnd/x11.rs` mirrors `dnd/windows.rs`, feeding the existing `DropTracker`; the pointer comes from the S-113 `PointerSource` trait. With a fake source and the headless Slint platform (as in `drop_hive_tests.rs`), `HoveredFile` x 3 gives exactly 1 `drop cells:` line equal to the cells for those 3 files (S-014c contract), and the centre shows `3`. `HoveredFileCancelled` closes the hive as `leave` does; `DroppedFile` with the fake pointer over cell k runs cell k once; over a gap or the centre it runs 0 actions and closes the hive within 500 ms.
- AC4 (highlight): with the fake pointer moved onto cell k, a tick at `TICK_MS` sets the highlight to k; with `reduced_motion` the highlight changes with no animation (duration 0). The live check is the `accent` pixel under the pointer within 100 ms (tool `pixel`).
- AC5 (copy only): the XDND reply is copy. The test reads winit 0.30's X11 `xdnd.rs` for the accepted action and records it in the story log. A live drop of a PNG with a move-capable source (`tests/drop/source.py`, GTK 4 with `GDK_BACKEND=x11`) and Shift held reports `deleted=False`, the original's sha256 is equal before and after, and `<name>.webp` exists within 5 s, 3 of 3 runs.
- AC6 (no drop on non-drop show): `DroppedFile` on a normal hive runs 0 actions (assert on the fake).
- AC7 (docs parity): `README.md` and `README.ar.md` change the X11 row to the verified state (platform plus tested or not, no personal details) in both files; `docs/drop-shortcut.md` and `docs/drop-shortcut.ar.md` each gain an X11 section with the same 3 steps (run `kawaro drop`, drag, drop on a cell); both start with the language switch line. A test asserts both pairs mention `X11` and the same step count.
- AC8 (records): `CHANGELOG.md` has one line naming S-113 to S-115 in English and Arabic; the X11 item under "still open" in ADR 0003 is closed with the measured result; `scripts/design-lint.sh` finds 0 long dashes in touched files.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | Table `(at, X11)`; assert equal centred placement | unit |
| AC2 | Spec values; live stacking list | unit, integration |
| AC3 | Headless platform with the fake pointer: event sequences, count log lines, count actions run | unit |
| AC4 | Fake pointer on cell k, tick, read highlight; live pixel check | unit, snapshot |
| AC5 | Source read of winit; `tests/x11/drop.sh` with sha256 and `file --mime-type` | unit, integration |
| AC6 | Fake sequence on a normal hive | unit |
| AC7 | Read the four docs, compare step counts and the first line | unit |
| AC8 | Reviewer reads CHANGELOG and ADR 0003; lint | review |

## Out of scope
`drop --hold`, `--release` on X11; the global shortcut during a drag; several monitors; Shape clipping; other window managers.

## Risks to check first
That Slint 1.18.1's winit loop gives `HoveredFile` and `DroppedFile` to the custom handler on X11 as it does on Windows (`FocusWatcher::window_event`), and that GTK 4 runs under `GDK_BACKEND=x11` in Xvfb. If not, report to the manager.

## Review log
- 2026-10-31, planner: split from S-015 (old AC9 to AC14). Reuses `DropTracker` and the Windows handler shape. Draft awaiting the manager.
