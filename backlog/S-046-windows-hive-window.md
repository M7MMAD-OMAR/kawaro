# S-046 The hive window on Windows: pointer position, focus, client mode, merged selections
Status: approved (manager, by delegation, 2026-10-24)

Builds on ADR 0005 sections 5 and 6 and on S-045 (the daemon builds for Windows) and S-042 (`AllowSetForegroundWindow`). For the Windows beta. Pure logic is in shared modules and tested on Linux; Win32 calls live in `crates/ui/src/win_window.rs` and `crates/daemon/src/shell_open.rs`, excluded from mutation. One work day or less; if it grows, split into the window (AC1 to AC4) and the client mode (AC5 to AC8).

## Story
As a Windows user, I want to select files in Explorer, choose SbarHive from the menu and see the hive at my pointer, inside the screen, taking keyboard focus, so that I can pick an action at once.

## Files
`crates/ui/src/{lib.rs,monitor.rs,win_window.rs}`, `crates/core/src/merge.rs`, `crates/daemon/src/{main.rs,args.rs,server.rs,shell_open.rs}`, `.cargo/mutants.toml`, `CHANGELOG.md`.

## Acceptance criteria
- AC1: `monitor::work_area_for(point: (i32, i32), monitors: &[Rect]) -> Rect` is pure. Given monitors A `(0,0,1920,1040)` and B `(-1280,0,1280,984)` as `(x, y, w, h)`: point `(100, 100)` gives A; `(-5, 10)` gives B; `(1919, 1039)` gives A; `(3000, 500)` (outside all) gives A (the nearest); an empty list returns a `Rect` of `(0,0,1920,1080)`.
- AC2: the Windows window uses `GetCursorPos` and the rect from AC1 as the input of the existing `layout::plan_at`; a test calls `plan_at` with the cursor `(1900, 20)` and area A and checks that every cell of the returned plan lies inside A (the shape adapts to the edge).
- AC3: the window is created undecorated, always on top, with no taskbar button, with `with_undecorated_shadow(false)`, and sets `DWMWA_WINDOW_CORNER_PREFERENCE` to `DWMWCP_DONOTROUND` (value 1). Constants are pinned against `windows_sys` with `const _: () = assert!(...)`. No `box-shadow`, gradient or `Acrylic` appears in the diff (design-lint clean).
- AC4: after show, the daemon calls `SetForegroundWindow` on the hive window; if the call returns 0 it writes the log line `sbarhived: foreground refused` once (pure helper `focus_log(ok: bool) -> Option<&'static str>`: `Some(...)` for false, `None` for true).
- AC5: `args::parse(&[String]) -> Result<Mode, String>` is pure: `[]` gives `Mode::Daemon`; `["open", "a.png"]` gives `Mode::Open(vec!["a.png"])`; `["open", "a", "b"]` gives both paths in order; `["open"]` gives `Err` whose text contains `open needs at least one file`; `["bogus"]` gives `Err`.
- AC6: `sbarhived open <file>` when a daemon answers on the pipe or socket acts as a client: it connects, sends exactly one line `{"cmd":"open","files":["<file>"],"token":null}`, reads the reply, and exits 0 (tested on Linux with a fake listener from `core::transport`). With no daemon it becomes the daemon and handles the same `Open` itself.
- AC7: `merge::Collector` is pure and takes the clock as an argument. `push(at_ms, path) -> Push` returns `Push::First` for the first path, and `Push::Merged` for any later path when `at_ms - last_push_ms < 300`. Pushes at 0, 100, 250 give `First, Merged, Merged` (gaps are measured from the last push). A push at 250 then 550 gives `Merged` then `First` (gap exactly 300 starts a new batch). `files()` returns paths in first-seen order without duplicates: pushes `a, b, a` give `[a, b]`.
- AC8: in the daemon, the first `Open` shows the hive at once (no 300 ms wait), and later merged paths rebuild the cells for the common type of all files. Test with a fake clock and the files `a.png`, `b.png`, `c.pdf` pushed at 0, 100 and 200: the show count is 1, and the final cell ids equal the ids of the `Actions` response for the three files together (not those for `a.png` alone).
- AC9: `cargo clippy --target x86_64-pc-windows-gnu -p <ui> -p <daemon> --lib --bins -- -D warnings` exits 0; `C`, `L` clean on Linux with the same test count plus the new tests; `M` has no surviving mutant in `monitor.rs`, `merge.rs`, `args.rs`; one CHANGELOG line (English, Arabic in «»).

## Check commands
`W`: `scripts/windows/check.sh`. `L`, `C`, `M` as in S-045, run inside the 6G scope with `-j 2`.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1, AC2 | Unit tests with the values above | unit |
| AC3 | `W`, plus a test that reads `win_window.rs` for the three `assert!` constants | build |
| AC4, AC5, AC7 | Unit tests with the values above | unit |
| AC6 | Integration test: fake listener records the received line; second run with no listener starts the in-process server | integration |
| AC8 | Unit test with the fake clock on the daemon's batching function | unit |

## Live check deferred to S-052
- Corners are transparent (not black), position at the pointer and at the screen edges, 150% scale, focus and Esc closing, ten files selected in Explorer give one hive.

## Out of scope
- Drop (S-047), tray and shortcut (S-048), the transparency fallbacks `UpdateLayeredWindow` and `SetWindowRgn`: opened as a new story only if the live check shows black corners.

## Review log
- 2026-10-24, planner: written from ADR 0005 S-046; client mode and merging are pure so Linux tests cover them.
