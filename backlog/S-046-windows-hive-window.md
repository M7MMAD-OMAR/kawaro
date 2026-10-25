# S-046 The hive window on Windows: pointer position, focus, client mode, merged selections
Status: done

Builds on ADR 0005 sections 5 and 6 and on S-045 (the daemon builds for Windows) and S-042 (`AllowSetForegroundWindow`). For the Windows beta. Pure logic is in shared modules and tested on Linux; Win32 calls live in `crates/ui/src/win_window.rs` and `crates/daemon/src/shell_open.rs`, excluded from mutation. One work day or less; if it grows, split into the window (AC1 to AC4) and the client mode (AC5 to AC8).

## Story
As a Windows user, I want to select files in Explorer, choose Kawaro from the menu and see the hive at my pointer, inside the screen, taking keyboard focus, so that I can pick an action at once.

## Files
`crates/ui/src/{lib.rs,monitor.rs,win_window.rs}`, `crates/core/src/merge.rs`, `crates/daemon/src/{main.rs,args.rs,server.rs,shell_open.rs}`, `.cargo/mutants.toml`, `CHANGELOG.md`.

## Acceptance criteria
- AC1: `monitor::work_area_for(point: (i32, i32), monitors: &[Rect]) -> Rect` is pure. Given monitors A `(0,0,1920,1040)` and B `(-1280,0,1280,984)` as `(x, y, w, h)`: point `(100, 100)` gives A; `(-5, 10)` gives B; `(1919, 1039)` gives A; `(3000, 500)` (outside all) gives A (the nearest); an empty list returns a `Rect` of `(0,0,1920,1080)`.
- AC2: the Windows window uses `GetCursorPos` and the rect from AC1 as the input of the existing `layout::plan_at`; a test calls `plan_at` with the cursor `(1900, 20)` and area A and checks that every cell of the returned plan lies inside A (the shape adapts to the edge).
- AC3: the window is created undecorated, always on top, with no taskbar button, with `with_undecorated_shadow(false)`, and sets `DWMWA_WINDOW_CORNER_PREFERENCE` to `DWMWCP_DONOTROUND` (value 1). Constants are pinned against `windows_sys` with `const _: () = assert!(...)`. No `box-shadow`, gradient or `Acrylic` appears in the diff (design-lint clean).
- AC4: after show, the daemon calls `SetForegroundWindow` on the hive window; if the call returns 0 it writes the log line `kawarod: foreground refused` once (pure helper `focus_log(ok: bool) -> Option<&'static str>`: `Some(...)` for false, `None` for true).
- AC5: `args::parse(&[String]) -> Result<Mode, String>` is pure: `[]` gives `Mode::Daemon`; `["open", "a.png"]` gives `Mode::Open(vec!["a.png"])`; `["open", "a", "b"]` gives both paths in order; `["open"]` gives `Err` whose text contains `open needs at least one file`; `["bogus"]` gives `Err`.
- AC6: `kawarod open <file>` when a daemon answers on the pipe or socket acts as a client: it connects, sends exactly one line `{"cmd":"open","files":["<file>"],"token":null}`, reads the reply, and exits 0 (tested on Linux with a fake listener from `core::transport`). With no daemon it becomes the daemon and handles the same `Open` itself.
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
- 2026-10-24, tester: tests written from the criteria only. Failure output before implementation (`L`, 6G scope, -j 2):
  - `crates/core/tests/s046_merge.rs` (AC7, 8 tests): `error[E0432]: unresolved import kawaro_core::merge`.
  - `crates/ui/src/monitor_tests.rs` (AC1, AC2, 10 tests): `error[E0432]: unresolved import crate::monitor`.
  - `crates/daemon/src/args_tests.rs` (AC5, 6 tests) and `batch_tests.rs` (AC4, AC8, 5 tests): `unresolved import crate::args`, `crate::batch`, `crate::server::focus_log`.
  - `crates/daemon/tests/s046_client.rs` (AC6, AC5 exit codes): both FAILED; AC6 left `["{\"cmd\":\"ping\"}"]`, right the one `open` line.
  - `crates/daemon/tests/s046_sources.rs` (AC3, AC4, AC9): 6 of 6 FAILED (win_window.rs, shell_open.rs missing; mutants.toml and CHANGELOG lack S-046).
  - Contracts for the implementer (names the tests fix): `monitor::Rect {x,y,w,h: i32}`; `merge::{Collector::new, push(u64, PathBuf) -> Push::{First,Merged}, files()}`; `args::Mode::{Daemon, Open(Vec<String>)}`; `server::focus_log`; `batch::{OpenBatch::new(cells_for), push(u64, PathBuf) -> Step::{Show,Rebuild}{files, cells}}`. Not covered: the "no daemon, becomes the daemon" branch of AC6 (needs a display; S-052 live check), AC9 clippy for Windows and mutants (gates).
- 2026-10-24, implementer: all S-046 tests pass on Linux, `scripts/windows/check.sh` and design-lint are clean. Deviations: (1) a Rebuild step presents the hive again (the window is replaced) instead of editing the cells in place; (2) DPI comes from `GetDpiForSystem`, not per monitor, to avoid the shcore import (S-052 checks 150% scale); (3) `#[allow(clippy::doc_lazy_continuation)]` on the `batch_tests` module line in `main.rs`, because the tester's header comment trips clippy and tests are not edited; (4) `OpenBatch::new` also requires `Send` so it can live in a static.
- 2026-10-24, reviewer: REWORK. Gates run in the 6G scope with -j 2: `cargo fmt --check` clean; `cargo clippy --all-targets -- -D warnings` clean; `cargo test --all` all green; `scripts/design-lint.sh` clean; `scripts/windows/check.sh` exit 0. I fixed the tester comment in `batch_tests.rs` ("+ 'static" started a list item, now "and 'static") and removed the `#[allow(clippy::doc_lazy_continuation)]` in `main.rs`; clippy stays clean (commit "S-046 review: fix the tester comment, drop the clippy allow"). Linux behaviour: `server::open` extraction keeps the old `Open` path (same notify, error text, `pending`, `present`), existing tests and e2e-facing commands unchanged; no change to the hive show path, so the 150 ms budget is untouched. ADR 0005 section 5/6 items present in `win_window.rs` (undecorated, always on top, skip taskbar, no undecorated shadow, DWMWCP_DONOTROUND pinned, work-area placement); exclusion of `win_window.rs` and `shell_open.rs` is honest, except that `shell_open::selection` holds a little glue (static batch, clock) that is acceptable. Mutants (`cargo mutants --in-place --in-diff` over `git diff 339a299~1 HEAD`, stopped after 33 of 47 mutants once 7 had survived; 21 caught, 5 unviable):
  1. `crates/daemon/src/main.rs:170`: delete match arm `(Ok(_), Ok(Response::Error{message}))` in `send_open` survives. No test makes a daemon answer an error to `open` and checks exit code 1 (AC6 only covers the OK reply).
  2. `crates/daemon/src/args.rs:17`: replace match guard `cmd == "open"` with `true` in `parse` survives. `["bogus"]` still gives `Err` (the "needs a file" one); a test must assert the text of the bogus error (`unknown argument`) and that `["bogus", "x"]` is `Err`, not `Open`.
  3. `crates/daemon/src/server.rs:42`: replace `open -> Result<(), String>` with `Ok(())` survives. Nothing checks that `open` returns `Err(no_actions text)` for a file with no matching action (the empty-actions branch runs before `ui::present`, so it is testable on Linux).
  4. `crates/ui/src/lib.rs:859`: four mutants of `Hive::hwnd` (None, Some(0), Some(1), Some(-1)) survive. The function is `#[cfg(windows)]`, so Linux never compiles the mutant and no test can kill it. Fix: add an exclude_re entry in `.cargo/mutants.toml` with a pointer to S-052 ("replace Hive::hwnd -> Option<isize> with"), like the other Windows glue; the decision stays in `win_window.rs`/`monitor.rs`.
  The mutants run was stopped early, so after the fixes rerun it fully: more survivors may be behind the 14 not yet run (monitor.rs, merge.rs, batch.rs, server.rs focus_log).
  Required for rework (new tests for 1 to 3 by the tester, config edit for 4 by the implementer); CHANGELOG already has the S-046 line. Status stays `review`.
- 2026-10-24, reviewer (second review): PASS. Gates in the 6G scope with -j 2: `cargo fmt --check` clean; `cargo clippy --all-targets -- -D warnings` clean; `cargo test --all` all green, no failures; `scripts/design-lint.sh` clean; `scripts/windows/check.sh` exit 0. `cargo mutants --in-place --in-diff` over the whole `git diff 339a299~1 HEAD`, no filter, no -j: 43 mutants, 37 caught, 6 unviable, 0 missed, 0 timeout (working tree clean afterwards). Linux e2e: `tests/e2e/run.sh` (private headless KWin, throwaway HOME) after a fresh `cargo build --release`: "e2e: all passed" and "e2e S-019 AC7: all passed", idle memory 38 MB, so Linux behaviour is unchanged by the main.rs argument handling and the `server::open` extraction. Every AC is tied to a test (table above); the Windows live check stays with S-052. The three review points of REWORK 1 are covered by the new tests and the `Hive::hwnd` exclusion.
