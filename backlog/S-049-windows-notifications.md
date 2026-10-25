# S-049 Notifications on Windows: app identity, Open folder, log counting
Status: review

Builds on ADR 0005 section 5 (notifications) and on S-045 (stderr goes to the log file on Windows). For the Windows beta. Pure helpers are in `crates/daemon/src/notify_logic.rs` (Linux tests); the daemon keeps `notify-rust`, which uses WinRT toasts on Windows. Half a work day.

## Story
As a Windows user, I want one toast when a job ends, with an Open folder button that selects the result in Explorer, so that I find the output without a window taking focus.

## Files
`crates/daemon/src/{notify.rs,notify_logic.rs,jobs.rs}`, `crates/core/src/appid.rs`, `.cargo/mutants.toml`, `CHANGELOG.md`.

## Acceptance criteria
- AC1: `appid::APP_ID` equals `io.github.m7mmad_omar.Kawaro`, and it is the same string the pipe name in `core::transport` uses as its prefix (one constant, one definition). On Windows the notification is built with `.app_id(APP_ID)`; on Linux nothing changes.
- AC2: `notify_logic::open_folder_command(os, folder: &Path, file: Option<&Path>) -> (String, Vec<String>)` is pure. `Os::Windows` with `file = Some("C:\\out dir\\a b.webp")` gives program `explorer.exe` and exactly one argument `/select,C:\out dir\a b.webp` (a single argument, spaces kept). `Os::Windows` with `file = None` and folder `C:\out` gives `explorer.exe` and `["C:\out"]`. `Os::Linux` gives `xdg-open` and `[folder]` as today.
- AC3: both call sites that spawn `xdg-open` (`notify.rs:59` and `jobs.rs:75`) use `open_folder_command`, and a text test finds no literal `xdg-open` in `crates/daemon/src` outside `notify_logic.rs`.
- AC4: `notify_logic::log_line(body: &str) -> String` returns `kawarod: notify: <body>`, the format the e2e tests count today; `count_notify_lines(log: &str) -> usize` counts the lines starting with that prefix. A text with 3 such lines among 5 gives 3; an empty text gives 0; a line that only contains the prefix in the middle does not count.
- AC5: one job that ends produces exactly one `notify:` line (existing behaviour); a regression test runs a job with the in-process daemon and calls `count_notify_lines` on the captured log, expecting 1.
- AC6: `W` (`scripts/windows/check.sh`) exits 0; `C`, `L` clean with the baseline count plus new tests; `M` has no surviving mutant in `notify_logic.rs` and `appid.rs`; one CHANGELOG line.

## Check commands
`W`, `L`, `C`, `M` as in S-045 (6G scope, `-j 2`).

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | Unit test on the constant and on the transport name prefix | unit |
| AC2, AC4 | Unit tests with the exact strings above | unit |
| AC3 | Text test over `crates/daemon/src` | unit |
| AC5 | Integration test with the in-process daemon | integration |

## Live check deferred to S-052
- The toast shows as Kawaro (needs the AUMID from S-050 or S-051; without it Windows may label it PowerShell), and Open folder selects the file in Explorer.

## Out of scope
- Registering the AUMID in the registry and the Start menu shortcut: S-050 and S-051.

## Review log
- 2026-10-24, planner: the toast name depends on the AUMID key; this story only sends the id.
- 2026-10-26, tester: tests in `crates/core/tests/s049_appid.rs` (AC1), `crates/daemon/tests/s049_notify_logic.rs` (AC2, AC4, module included by path), `crates/daemon/tests/s049_sources.rs` (AC3) and `crates/daemon/src/s049_notify_tests.rs` (AC5, `#[cfg(test)] mod s049_notify_tests;` added to main.rs). Contracts are in each file's header; AC5 needs `crate::notify::captured_log()` in test builds (the thread's `notify:` lines). AC5 runs a real `image.to-png` job on `tests/fixtures/images/orient6.jpg` with `run_job` and feeds its events to `jobs::pump` with `open_folder_when_done = false`. All fail before implementation:
  - AC1: `error[E0432]: unresolved import kawaro_core::appid --> crates/core/tests/s049_appid.rs:13:20`
  - AC2, AC4: `error: couldn't find file crates/daemon/tests/../src/notify_logic.rs --> crates/daemon/tests/s049_notify_logic.rs:19:1`
  - AC3: `test ac3_no_xdg_open_in_daemon_sources_outside_notify_logic_and_both_call_sites_use_the_helper ... FAILED`, `crates/daemon/src/notify_logic.rs must exist`
  - AC5: `error[E0432]: unresolved import crate::notify_logic --> crates/daemon/src/s049_notify_tests.rs:16:12` and `error[E0432]: unresolved import crate::notify::captured_log --> crates/daemon/src/s049_notify_tests.rs:15:5`
  - AC6 (W, C, L, M, CHANGELOG) is a reviewer gate, no test.
- 2026-10-26, implementer: `core::appid::APP_ID` is the one definition and `transport::pipe_name` formats with it; `daemon/src/notify_logic.rs` holds `Os` (a `pub use` of `core::paths::Os`), `open_folder_command`, `log_line` and `count_notify_lines`; `notify.rs` builds every notification through one `notification()` helper that adds `.app_id(APP_ID)` under `cfg(windows)`, and writes its `notify:` lines through `write_log` (stderr as before, plus a thread local buffer read by `captured_log()` in test builds); both open folder spawns (`notify.rs` finished, `jobs.rs` pump) use `open_folder_command(CURRENT, ...)` with the first output as the file. The update notification line keeps its literal `kawarod: notify:` format string because `update_tests::ac6_update_notifications_never_open_a_window` requires the literal in `notify.rs`. Gates: `cargo fmt --all --check` exit 0; `cargo clippy --workspace --all-targets -- -D warnings` exit 0; `cargo test -p kawaro-core -p kawaro-daemon` exit 0 (682 passed, 0 failed, 4 ignored); `cargo test --workspace` exit 0 (1197 passed, 0 failed, 5 ignored); `KAWARO_WIN_SCOPE=1 scripts/windows/check.sh` exit 0; `scripts/design-lint.sh` exit 0. M (`--in-diff` over notify_logic.rs, appid.rs, notify.rs, jobs.rs, transport.rs): 17 mutants, 14 caught, 3 missed, none in `notify_logic.rs` or `appid.rs`. Two missed ones reach only the desktop and are now excluded in `.cargo/mutants.toml` with a reason (`replace notification -> Notification with Default::default()`, `replace == with != in finished` on the Open folder click); `--list` after the exclusion shows the remaining 15. One survivor stays open: `notify.rs: replace info with ()`; `info` now writes through `write_log`, so a test that calls `notify::info` and reads `captured_log()` would kill it, which is tester work (no test calls `info` today). Not proven here: Explorer's handling of the quoted `/select,` argument that std builds for a path with spaces, and the toast label, both for S-052; paths that are not UTF-8 are passed lossily to `xdg-open`.
- 2026-10-26, tester (follow-up): added `ac5_info_writes_exactly_one_notify_line_with_its_body` to `crates/daemon/src/s049_notify_tests.rs`. It calls `notify::info(Lang::En, "s049 info body, صبار")` and asserts, on the part of `captured_log()` it added, that `count_notify_lines` is 1 and the text is exactly `kawarod: notify: s049 info body, صبار\n` (also checked against `notify_logic::log_line`). With the body of `info` temporarily removed (file restored after, `git diff --stat` showed only the test file) it fails: `test s049_notify_tests::ac5_info_writes_exactly_one_notify_line_with_its_body ... FAILED`, `panicked at crates/daemon/src/s049_notify_tests.rs:100:5: left: 0 right: 1`, `test result: FAILED. 1 passed; 1 failed`. Against the real code: `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 131 filtered out`. M (`--in-diff` of `notify.rs` against 503ac71, `-F 'replace info with'`): `1 mutant tested in 70s: 1 caught`.
