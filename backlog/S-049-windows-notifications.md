# S-049 Notifications on Windows: app identity, Open folder, log counting
Status: approved (manager, by delegation, 2026-10-24)

Builds on ADR 0005 section 5 (notifications) and on S-045 (stderr goes to the log file on Windows). For the Windows beta. Pure helpers are in `crates/daemon/src/notify_logic.rs` (Linux tests); the daemon keeps `notify-rust`, which uses WinRT toasts on Windows. Half a work day.

## Story
As a Windows user, I want one toast when a job ends, with an Open folder button that selects the result in Explorer, so that I find the output without a window taking focus.

## Files
`crates/daemon/src/{notify.rs,notify_logic.rs,jobs.rs}`, `crates/core/src/appid.rs`, `.cargo/mutants.toml`, `CHANGELOG.md`.

## Acceptance criteria
- AC1: `appid::APP_ID` equals `io.github.m7mmad_omar.SbarHive`, and it is the same string the pipe name in `core::transport` uses as its prefix (one constant, one definition). On Windows the notification is built with `.app_id(APP_ID)`; on Linux nothing changes.
- AC2: `notify_logic::open_folder_command(os, folder: &Path, file: Option<&Path>) -> (String, Vec<String>)` is pure. `Os::Windows` with `file = Some("C:\\out dir\\a b.webp")` gives program `explorer.exe` and exactly one argument `/select,C:\out dir\a b.webp` (a single argument, spaces kept). `Os::Windows` with `file = None` and folder `C:\out` gives `explorer.exe` and `["C:\out"]`. `Os::Linux` gives `xdg-open` and `[folder]` as today.
- AC3: both call sites that spawn `xdg-open` (`notify.rs:59` and `jobs.rs:75`) use `open_folder_command`, and a text test finds no literal `xdg-open` in `crates/daemon/src` outside `notify_logic.rs`.
- AC4: `notify_logic::log_line(body: &str) -> String` returns `sbarhived: notify: <body>`, the format the e2e tests count today; `count_notify_lines(log: &str) -> usize` counts the lines starting with that prefix. A text with 3 such lines among 5 gives 3; an empty text gives 0; a line that only contains the prefix in the middle does not count.
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
- The toast shows as SbarHive (needs the AUMID from S-050 or S-051; without it Windows may label it PowerShell), and Open folder selects the file in Explorer.

## Out of scope
- Registering the AUMID in the registry and the Start menu shortcut: S-050 and S-051.

## Review log
- 2026-10-24, planner: the toast name depends on the AUMID key; this story only sends the id.
