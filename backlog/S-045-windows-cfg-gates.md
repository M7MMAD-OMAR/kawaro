# S-045 Build the whole workspace for Windows: cfg gates, daemon subsystem, log file
Status: approved (manager, by delegation, 2026-10-24)

Builds on ADR 0005 sections 1 and 5 and on S-040 to S-044. For the Windows beta (order today: S-045, S-046, S-047, S-048, S-049, S-050, S-051, then S-052). First story that touches `crates/ui` and `crates/daemon` for the Windows target. One work day or less.

## Story
As the maintainer, I want `sbarhived.exe` and everything it depends on to build for `x86_64-pc-windows-gnu`, with no console window and a log file, so that the next stories can add the Windows behaviour on top of a build that already works.

## Files
`crates/ui/src/lib.rs`, `crates/daemon/{Cargo.toml,src/main.rs,src/tray.rs,src/tray/*.rs,src/notify.rs,src/logfile.rs}`, `crates/core/src/paths.rs`, `scripts/windows/{check.sh,build.sh}`, `docs/engines.md`, `CHANGELOG.md`.

## Acceptance criteria
- AC1: `cargo check --workspace --target x86_64-pc-windows-gnu` exits 0 with no error (today it fails in `crates/ui/src/lib.rs:244` on `winit::platform::startup_notify`).
- AC2: `startup_notify` and `token_hook` appear in `crates/ui/src/lib.rs` only inside items gated by `cfg(unix)` (X11 and Wayland). On Linux `cargo test --workspace` passes the same number of tests as before the story (record the number in the review log).
- AC3: `ksni` is declared in `crates/daemon/Cargo.toml` only under `[target.'cfg(target_os = "linux")'.dependencies]`.
- AC4: on Windows the daemon `tray` module exposes `start(lang, queue, state) -> TrayHandle`, `set_active(&TrayHandle, Vec<(String, f32)>)` and `set_lang(Lang)` with the same signatures as on Linux, and bodies that do nothing (they return `None` or `()`). AC1 proves they compile.
- AC5: `crates/daemon/src/main.rs` starts with `#![cfg_attr(windows, windows_subsystem = "windows")]`. `x86_64-w64-mingw32-objdump -p sbarhived.exe` prints `Subsystem` with value `00000002` (GUI), and `sbarhive.exe` prints `00000003` (console).
- AC6: `paths::log_file_with(os, env, known) -> Option<PathBuf>` is pure and equals `log_dir_with(...)` joined with `sbarhived.log`. For `Os::Windows` with `LOCALAPPDATA=C:\Users\u\AppData\Local` it gives `C:\Users\u\AppData\Local\SbarHive\logs\sbarhived.log`; with the variable missing it falls back to the `known` function, and `None` when both fail.
- AC7: `logfile::open_append(path) -> io::Result<File>` creates missing parent directories and appends. Calling it twice on a temporary path and writing one line each leaves a file with exactly 2 lines. On Windows the daemon redirects its stderr to this file at start, so the existing `sbarhived: notify:` lines land in it; on Linux stderr is unchanged.
- AC8: `scripts/windows/check.sh` also runs `cargo clippy -j 2 --target x86_64-pc-windows-gnu -p <daemon> -p <ui> --lib --bins -- -D warnings` and exits 0. `scripts/windows/build.sh` also builds `sbarhived.exe` in release, and `scripts/windows/check-dlls.sh` prints `dll check: ok sbarhived.exe <N> imports` with exit 0 (no `lib*.dll`).
- AC9: `[[package]]` count in `Cargo.lock` is unchanged (record it), and `git diff --stat -- crates/ui` shows only `lib.rs`. One line in `CHANGELOG.md` under `[Unreleased]`, English with Arabic in «». `scripts/design-lint.sh` clean.

## Check commands (inside `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200%`, `-j 2`)
- `W`: `scripts/windows/check.sh`, `scripts/windows/build.sh`, then `scripts/windows/check-dlls.sh <target>/x86_64-pc-windows-gnu/release/sbarhived.exe`.
- `L`: `cargo test -j 2 --workspace`. `C`: `cargo fmt --check && cargo clippy -j 2 --workspace --all-targets -- -D warnings`.
- `M`: `cargo mutants --in-place --in-diff <diff>`; `tray/windows.rs` stays out of mutation (`exclude_globs`).

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1, AC4, AC8 | `W` exits 0; the `sbarhived.exe` file exists and `file` says `PE32+ executable` | build |
| AC2, AC3 | Test reads `lib.rs` and `daemon/Cargo.toml` and checks the gate text; `L` count equals baseline | unit |
| AC5 | `objdump -p` output contains `Subsystem` followed by `00000002` for the daemon and `00000003` for the cli | build |
| AC6 | Unit test with the three environments above, a missing variable, and a failing `known` | unit |
| AC7 | Unit test in a temporary directory with a nested missing path, two appends, count lines | unit |
| AC9 | Reviewer reads CHANGELOG and runs `grep -c '^\[\[package\]\]' Cargo.lock` | review |

## Live check deferred to S-052
- Starting `sbarhived.exe` opens no console window, and the log file appears under `%LOCALAPPDATA%\SbarHive\logs\`.

## Out of scope
- Any working tray, notification, window or pipe behaviour on Windows (S-046 to S-049).

## Review log
- 2026-10-24, planner: story written from ADR 0005 "Proposed stories" S-045 with the beta order.
