# S-047 Drag from Explorer onto a cell of the hive on Windows
Status: approved (manager, by delegation, 2026-10-24)

Builds on ADR 0005 section 5 (drop) and ADR 0003, and on S-046 (the hive window). For the Windows beta. The pure tracker is in `crates/ui/src/drop_track.rs` and tested on Linux; the Win32 timer and `GetCursorPos` are in `crates/ui/src/dnd/windows.rs`, excluded from mutation. One work day or less.

## Story
As a Windows user, I want to drag files from Explorer onto a cell of the hive and have that action run, with the original left in place, so that I can use the hive-first, drag-second flow.

## Files
`crates/ui/src/{drop_track.rs,dnd.rs,dnd/windows.rs,lib.rs}`, `crates/daemon/src/server.rs` (the S-021 gate input), `.cargo/mutants.toml`, `CHANGELOG.md`.

## Acceptance criteria
- AC1: `DropTracker` is pure. `on_hover(files: Vec<PathBuf>)` stores the files. `on_tick(&Layout, cursor: (f32, f32)) -> Option<i32>` returns `Some(cell)` only when the cell under the cursor (`Layout::pick`) differs from the last returned one, else `None`. Using the layout of the test fixtures: a tick over cell 3 gives `Some(3)`, a second tick over cell 3 gives `None`, a tick over empty space gives `Some(<the value pick gives for no cell>)`.
- AC2: `on_drop(&Layout, cursor) -> DropOutcome`: over cell 3 with 2 stored files it gives `Run { cell: 3, files: [a, b] }` (order kept); over empty space it gives `Ignored`; with no stored files it gives `Ignored`. After either outcome the stored files are cleared.
- AC3: `on_cancel()` clears the files, and a following `on_drop` gives `Ignored`.
- AC4: `TICK_MS` is `16` and the Windows file starts a timer with that value only between `HoveredFile` and `DroppedFile` or `HoveredFileCancelled`; a unit test pins the constant and a build test pins that the timer is killed on both end events (`KillTimer` or the winit timer cancel appears in both match arms of `dnd/windows.rs`).
- AC5: the copy rule: no code path in the story calls a delete, move or rename on a dropped file. A test runs `on_drop` with a real temporary file, executes the resulting action through the existing runner with the `copy` of an image to PNG, and checks that the original still exists with the same SHA-256 and the output is a new file next to it.
- AC6 (S-021 rule): `drop_hive_allowed(source: Source, left_button_down: bool) -> bool` is pure: for `Source::Shortcut` it equals `left_button_down`; for `Source::Menu` it is `true`. The Windows shortcut path reads `GetAsyncKeyState(VK_LBUTTON)` and passes it in.
- AC7: `HoveredFile` and `DroppedFile` events reach `DropTracker` through the existing `FocusWatcher::window_event`; `W` (`scripts/windows/check.sh`) exits 0 and clippy for the target has no warning. `C`, `L` clean with the baseline count plus new tests; `M` has no surviving mutant in `drop_track.rs`; one CHANGELOG line.

## Check commands
`W`, `L`, `C`, `M` as in S-045 (6G scope, `-j 2`).

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1, AC2, AC3 | Unit tests on `DropTracker` with a fixed `Layout`, asserting each returned value | unit |
| AC4 | Constant test plus a text test over `dnd/windows.rs` | unit |
| AC5 | Integration test with a temporary PNG, checking the hash and the new output file | integration |
| AC6 | Unit test with the four combinations | unit |
| AC7 | `W` exit code and the reviewer reads `FocusWatcher::window_event` | build, review |

## Live check deferred to S-052
- Drag from Explorer onto a cell highlights it, drop runs the action, the original stays; the shortcut during a drag (does `WM_HOTKEY` arrive during an OLE drag?).

## Out of scope
- Our own `IDropTarget` (only if the live check shows polling fails); drop position accuracy at mixed DPI.

## Review log
- 2026-10-24, planner: from ADR 0005 S-047; the cursor is polled because winit gives no drop position.
