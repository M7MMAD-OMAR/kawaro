# S-107 Verify the foreground result and repeat it after the window shows (Windows)
Status: review

## Story
As a Windows user, I want `kawaro settings` and the welcome window to appear in front of the app I am using, so that I do not have to hunt for a window that opened behind my browser.

## Evidence and causes
- Found by the S-106 A8 VM run (Windows 11, build 0.4.5-pre on main). With Edge as the foreground window, `kawaro.exe settings` (daemon already running, started by the user or by the MSI, both reproduce) opens the settings window BEHIND Edge. The daemon stderr shows no `kawarod: foreground refused` line. Screenshot: `docs/screenshots/s106/windows/settings-behind-edge-before-fix.png`. With a console as the previous foreground window the window appeared in front, which hid the bug.
- Cause 1: `shell_open::foreground` (`crates/daemon/src/shell_open.rs`) trusts the return value of `SetForegroundWindow`. Windows can return nonzero without the window becoming foreground, so the ladder in `foreground_logic.rs` stops at step 1 and never tries the topmost toggle or the flash.
- Cause 2 (suspected): the call runs before the Slint/winit window is really shown (`show_settings` then `raise_welcome` in `crates/daemon/src/ui.rs`, and `present`). A show that completes after the call maps the window without activation.
- Design note: `Step` keeps its three variants and `foreground_ladder` keeps its signature, so the five tests in `crates/daemon/tests/s106_foreground.rs` stay valid. `AttachThreadInput` to the foreground window's thread, if used, lives inside the `Topmost` step in the Windows glue, not as a new variant (the call count stays at most 3).

## Acceptance criteria
- A1 (pure): `foreground_ladder(try_step)` is unchanged in signature and behaviour: the closure is the already verified result, steps in order Plain, Topmost, Flash, at most 3 calls, returns the step that worked or `Flash`.
- A2 (Windows glue): in `shell_open::foreground` each of the Plain and Topmost steps returns `GetForegroundWindow() == hwnd` read after the call (and a 50 ms settle at most), never the return value of `SetForegroundWindow`. `kawarod: foreground refused` is logged when the verified result of step 1 is false.
- A3 (pure): `should_retry(verified: bool, attempts: u32) -> bool` in `foreground_logic.rs` returns true only when `verified` is false and `attempts < 1`. Table: `(false, 0)` true, `(false, 1)` false, `(true, 0)` false, `(true, 1)` false, `(false, 5)` false.
- A4 (wiring): `show_settings`, the welcome path (`raise_welcome`) and `present` in `crates/daemon/src/ui.rs` schedule one repeat of `foreground` about 150 ms (between 100 and 200 ms) after the window is shown, and the repeat is skipped when `should_retry` says false. The repeat runs on the UI thread timer and never blocks it.
- A5 (scan): `shell_open.rs` contains `GetForegroundWindow` inside `foreground`; `ui.rs` contains the repeat scheduling and a call to `should_retry`.
- A6 (cross build): `scripts/windows/check.sh` exits 0 (fmt, clippy `-D warnings` for the Windows target).
- A7 (VM, release step, not a test): on Windows 11 with Edge in front, `kawaro.exe settings` leaves the settings window as the foreground window within 3 s, with the daemon started by the user and again with the daemon started by the MSI. The same for the welcome window. Record which step won (plain, topmost, flash) and the screenshot in `docs/screenshots/s107/windows/`. If the window is still behind, the story reopens.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| A1 | Existing `s106_foreground` (5 tests) pass with no edit | unit (daemon) |
| A2 | Scan of `foreground` in `shell_open.rs`: `GetForegroundWindow` is compared with `hwnd` after each of the Plain and Topmost calls, and no `SetForegroundWindow(hwnd) != 0` is returned directly | source scan |
| A3 | The five table rows above give exactly true, false, false, false, false | unit (daemon) |
| A4 | Scan of `ui.rs`: in each of `show_settings`, `raise_welcome`, `present` a timer of 100 to 200 ms is started that calls `foreground`, gated by `should_retry` | source scan |
| A5 | Scan finds `GetForegroundWindow` in `shell_open.rs`, and the timer plus `should_retry` in `ui.rs` | source scan |
| A6 | `scripts/windows/check.sh` exit code 0 | cross build |
| A7 | The VM run above, recorded in the review log | manual, release step |

## Out of scope
Linux and macOS focus (S-082), the update helper's foreground right (S-106 A2), more than one repeat, changing the Flash fallback, the hive overlay focus path, signing.

## Review log
Tester, 2026-10-30. New tests (not edited elsewhere): `crates/daemon/tests/s107_should_retry.rs` (A3, 4 tests) and `crates/daemon/tests/s107_foreground_scans.rs` (A2, A4, A5, 7 tests). A1 is the untouched `s106_foreground`. A6 is the cross build script, run by the reviewer (it passes today, so no test is written). A7 is manual. Command: `~/.cache/s107-tests.sh` inside `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=100%`, nice 15, CARGO_BUILD_JOBS=1, `--test-threads=2`.
- A3: fails to compile, `error[E0432]: unresolved import foreground_logic::should_retry` (the hook does not exist yet).
- A2: `foreground must read GetForegroundWindow after the Plain and the Topmost call`; `SetForegroundWindow return value used directly: SetForegroundWindow(hwnd)!=0`; `foreground must settle before reading GetForegroundWindow`.
- A4: `show_settings`, `raise_welcome`, `present` each fail with `the repeat must be gated by should_retry`.
- A5: `assertion failed: code_of("shell_open.rs").contains("GetForegroundWindow")`.
- Result: 7 failed, 0 passed in the scans file; the A3 file does not compile. A first draft had a refusal-log test that already passed; it was removed (covered by s106_foreground).
- Pinning notes for the implementer: the scans read non-comment lines; `foreground`, `show_settings`, `raise_welcome`, `present` must be item-level `fn`s whose body ends at the next `fn`; the timer must be named `Timer`/`timer` with `Duration::from_millis(N)` N in 100..=200 inside each of the three fns, and the settle is `from_millis(N)` N <= 50 inside `foreground`.

Reviewer, 2026-10-30. Verdict: REWORK.
- A7 (done by the manager, Windows 11 VM, 0.4.5 build, Edge in front): the settings window opens in front. Screenshots: `docs/screenshots/s107/windows/edge-foreground-before.png` and `settings-in-front-after-fix.png`. Which ladder step won was not recorded; the welcome window and the MSI-started daemon case were not shown.
- Passed: clippy `-D warnings` (daemon), `cargo test -p kawaro-daemon` (205 unit plus all integration files, including s106_foreground 5, s107_should_retry 4, s107_foreground_scans 7), `scripts/windows/check.sh` exit 0, `scripts/design-lint.sh` clean, CHANGELOG line present in English and Arabic, no long dashes found.
- Failed: `cargo fmt --check` exits 1: rustfmt diffs in `crates/daemon/tests/s107_foreground_scans.rs` (lines 23 and 33, the `.iter().filter_map` marker list). Fix: run `cargo fmt` and commit that file.
- Note (not blocking): `show_settings` calls `raise_welcome(win)`, which already schedules the 150 ms repeat, and then schedules a second identical timer, so two repeats can run there (criteria say one). Consider removing the extra timer in `show_settings`, or confirm the scan test still passes.
- Note: `should_retry(verified, 0)` is always called with a literal 0, so the attempts guard is never exercised by the wiring; harmless because the repeat runs once.
