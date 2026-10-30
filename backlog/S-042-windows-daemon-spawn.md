# S-042 Start the daemon from the command line on Windows and grant it the foreground right
Status: done (reviewer PASS 2026-10-22), on the windows/0.6.0 branch at the owner's request; ADR 0005 is still proposed; merged into main locally on 2026-10-23 (not pushed)
Renumbered on 2026-10-23: this story was S-024 on windows/0.6.0 before the merge into main (main had already used S-022 and S-027). Commit messages and review-log hashes keep the old number.

Builds on ADR 0005 sections 3 and 5 (focus), and on S-040b (`Stream::server_pid`). For release 0.6.0. Does not touch `crates/ui`. One work day or less, so no split. This is the first story that builds `kawaro.exe` for the Windows target.

## Story
As a Windows user, I want `kawaro open` to start the daemon if it is not running, without the daemon dying when I close the terminal, and I want the hive to take focus when it appears, so that I can use Kawaro from the terminal as on Linux.

## Files
`crates/cli/src/{client.rs,spawn.rs,spawn/windows.rs,main.rs}`, `crates/cli/Cargo.toml` (windows-sys under `cfg(windows)`), `Cargo.lock` (the cli dependency array only), `.cargo/mutants.toml`, `README.md`, `CHANGELOG.md`.

## Acceptance criteria
- AC1: `spawn::start_with(attempt)` is pure, and `attempt(flags: u32) -> io::Result<()>` is injected. The first attempt uses the flags `0x01000208` (`DETACHED_PROCESS` 0x8, `CREATE_NEW_PROCESS_GROUP` 0x200 and `CREATE_BREAKAWAY_FROM_JOB` 0x01000000). If it fails with `raw_os_error() == Some(5)`, it is retried once with `0x208`. The recorded flag sequence: first success `[0x01000208]`; refusal then success `[0x01000208, 0x208]`; two refusals `[0x01000208, 0x208]` and the second error is returned; a first `NotFound` `[0x01000208]`, returned without a retry.
- AC2: the three constants are defined in the shared `spawn.rs`, and `spawn/windows.rs` checks them at compile time against `windows_sys` (`const _: () = assert!(...)` for each constant), so the build for the target fails if a value differs. On Windows the start uses `CommandExt::creation_flags`; on Linux `process_group(0)` stays as today.
- AC3: `spawn::wait_exit(cmd) -> Result<i32, String>` is shared, and it is `exec_daemon` on Windows: start, then wait, then the daemon's exit code. On Linux `sh -c 'exit 3'` gives `Ok(3)`, and a program that does not exist gives an `Err` whose text starts with `cannot run kawarod:`. `exec_daemon` on Linux stays `exec` as today.
- AC4: `spawn::daemon_binary_in(dir, suffix)` is pure: a temporary directory holding a file `kawarod.exe` with `suffix = ".exe"` gives its path, and without it gives `PathBuf::from("kawarod.exe")`. With `suffix = ""` both results are as today (`kawarod`). `daemon_binary()` passes `std::env::consts::EXE_SUFFIX`.
- AC5: `spawn::foreground_grant(req, server_pid) -> Option<u32>` is pure: `Some(pid)` for `Open` and `Drop` (with `hold` true and false) when `server_pid` equals `Some(pid)`, and `None` for every other request (`Ping`, `DropRelease`, `Run`, `Actions`, `CancelAll`, `Settings`, `Reload` and `Quit`) and `None` when `server_pid` equals `None`. The client calls it after connecting and before writing the request, and on Windows passes the result to `AllowSetForegroundWindow` in `spawn/windows.rs`.
- AC6: `cargo check` and `cargo clippy -p kawaro-cli --bins` for the Windows target with no warning, and `cargo build --release -p kawaro-cli` for the target produces `kawaro.exe`. `x86_64-w64-mingw32-objdump -p kawaro.exe` lists `AllowSetForegroundWindow`, `GetNamedPipeServerProcessId`, `CancelIoEx` and `WaitNamedPipeW` among the imports.
- AC7: no `std::os::unix` outside `cfg(unix)` blocks in `crates/cli/src`: `cargo check -p kawaro-cli` for the target succeeds, and that alone proves it.
- AC8: no new package: the `[[package]]` count in `Cargo.lock` stays 671, and `X` lists `windows-sys v0.61.2` as the only new entry.
- AC9: `README.md` says in Arabic and English that `kawaro open` on Windows starts `kawarod.exe` from the program folder. One line in `CHANGELOG.md` under `[Unreleased]` in English with Arabic text in «». `git diff --stat main...HEAD -- crates/ui` is empty. `C` with no warning on Linux: in the cli binary, `start_with`, `wait_exit` and the three constants are called only by the Windows branch, so they carry `#[cfg_attr(not(windows), allow(dead_code))]`. `M` with no surviving mutant.

## Check commands
Read-only tools (`grep` and `objdump`) run outside the scope.
- `C`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% sh -c 'cargo fmt --check && cargo clippy -j 2 --workspace --all-targets -- -D warnings'`
- `L`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% cargo test -j 2 -p kawaro-cli`
- `W`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_TARGET_DIR=$HOME/.cache/kawaro-win-target cargo clippy -j 2 --target x86_64-pc-windows-gnu -p kawaro-core -p kawaro-engines --lib -p kawaro-cli --bins -- -D warnings`
- `B`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_TARGET_DIR=$HOME/.cache/kawaro-win-target cargo build -j 2 --release --target x86_64-pc-windows-gnu -p kawaro-cli`, then `x86_64-w64-mingw32-objdump -p $HOME/.cache/kawaro-win-target/x86_64-pc-windows-gnu/release/kawaro.exe`
- `M`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_BUILD_JOBS=2 cargo mutants --jobs 1 --in-diff <diff>` (the changed files only, including `main.rs`; `spawn/windows.rs` is excluded)
- `X`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_TARGET_DIR=$HOME/.cache/kawaro-win-target cargo tree --target x86_64-pc-windows-gnu -p kawaro-cli -e normal --depth 1`

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | Unit test with four fake `attempt` functions, asserting the flag sequence and the returned error; under `M` | unit |
| AC2, AC6, AC7 | `W` and `B`, and `grep` on the `objdump` output for the four names | build |
| AC3 | Unit test on Linux with `sh -c 'exit 3'` and with a path that does not exist | unit |
| AC4 | Unit test in a temporary directory with both suffixes | unit |
| AC5 | Unit test with all ten `Request` variants, with `Some(42)` and with `None` | unit |
| AC8 | `grep -c '^\[\[package\]\]' Cargo.lock` gives 671, and `X` | review |
| AC9 | The reviewer reads README and CHANGELOG.md | review |

These stay green: `cargo test --workspace`, `crates/cli/tests/cli.rs` (starting the daemon on Linux) and `tests/e2e/run.sh`.

## Live check deferred to S-052
- `kawaro open` from a Windows Terminal, then closing it: the daemon stays.
- A terminal inside a job that forbids breaking away from it: the retry without `CREATE_BREAKAWAY_FROM_JOB` succeeds.
- No console window for the daemon (depends on `windows_subsystem` in S-045).
- The hive takes focus after `AllowSetForegroundWindow` (depends on S-046).

## Out of scope
- Client mode in `kawarod.exe open "%1"` from Explorer, and `SetForegroundWindow` in the daemon: S-046.
- `Settings` does not grant the foreground right, as the ADR specifies ("before Open and Drop"); revisit in S-052 if the settings window appears behind others.

## Review log
- 2026-10-22, planner: the flags are numbers in a shared file that is mutated, and their correctness against windows-sys is proven at compile time for the target, because windows-sys is not available on Linux.
- 2026-10-22, tester:
  - The contract pinned at the top of `crates/cli/src/spawn.rs` (the file is new and holds only the test module; the implementation is written above it): the constants `DETACHED_PROCESS`, `CREATE_NEW_PROCESS_GROUP` and `CREATE_BREAKAWAY_FROM_JOB` of type `u32`; `start_with(attempt: impl FnMut(u32) -> io::Result<()>) -> io::Result<()>`, and the tests pass `&mut` to a closure, so `&mut dyn FnMut` also works; `wait_exit(cmd: &mut Command) -> Result<i32, String>`; `daemon_binary_in(dir: &Path, suffix: &str) -> PathBuf`; `foreground_grant(req: &Request, server_pid: Option<u32>) -> Option<u32>`. `mod spawn;` was added to `main.rs` so the tests build, and nothing else changed there.
  - Tests: in `crates/cli/src/spawn.rs` (the `kawaro` binary): AC1 six (the constant values; first success; refusal then success; two refusals; refusal then a different error, which is the one returned; `NotFound` with no retry), and the refusal is `from_raw_os_error(5)` because that is EIO on Linux, so an implementation that relies on `kind()` does not pass. AC3 three (`exit 3` and `exit 0` under `cfg(unix)`, and a program that does not exist). AC4 six (both suffixes with and without the file, a lone `kawarod` file with `.exe` gives `kawarod.exe`, and a directory named `kawarod` is not the binary, as today). AC5 three (the eleven requests with `Some(42)` and with `None`, and a `match` with no catch-all arm so the build fails on a new variant). And in `crates/cli/tests/spawn_windows.rs` four for the build rows: `spawn/windows.rs` has three `const _: () = assert!(`, each naming a constant and `windows_sys`, plus `creation_flags` and `AllowSetForegroundWindow`; `windows-sys` only under `[target.'cfg(windows)'.dependencies]` in `crates/cli/Cargo.toml`; `spawn/windows.rs` in `exclude_globs`, and `spawn.rs`, `client.rs` and `main.rs` not in it.
  - Ambiguities and choices: (1) the criterion does not define the shape of `cmd` in `wait_exit`, so `&mut Command` was chosen. (2) Calling `foreground_grant` in the client "after connecting and before writing" has no row in the table, so it is for the reviewer to read in `client.rs`. (3) AC7 has no test other than `W`, as the criterion says. (4) AC8 is review only: the count is already 671 today because `windows-sys 0.61.2` is in the lock file from core, so a test for it would pass before the implementation. (5) Note for the reviewer: if the flags are built with `|` from separate bits, cargo-mutants may produce an equivalent `|` to `^` mutant that survives in `M`.
  - For the reviewer, not tests: `W` and `B` with `objdump` for the four names, `X`, the `[[package]]` count, README and CHANGELOG.md, `git diff --stat main...HEAD -- crates/ui`, `C` and `M`.
  - The inline tests are not seen by the `crates/*/tests/*` check: the test module in `spawn.rs` and the existing test module in `main.rs` must not change after this commit (an empty diff is required; the tester's commit is a6bd519):
    `diff <(git show a6bd519:crates/cli/src/spawn.rs | sed -n '/^#\[cfg(test)\]/,$p') <(sed -n '/^#\[cfg(test)\]/,$p' crates/cli/src/spawn.rs)`
    `diff <(git show a6bd519:crates/cli/src/main.rs | sed -n '/^#\[cfg(test)\]/,$p') <(sed -n '/^#\[cfg(test)\]/,$p' crates/cli/src/main.rs)`
  - Note: the `kawaro` test binary does not build until the implementation lands, so the existing unit tests in it (`clipboard` and `resolve_token`) are red until then. `W` is not blocked by `crates/ui`: all the errors are in `client.rs`.
  - Check of the test logic: a temporary copy in the session folder (deleted, not merged) with a short implementation of the contract passed the binary's twenty-three tests and the four `spawn_windows` tests, and `cargo clippy -p kawaro-cli --tests -D warnings` is clean on it. Here `cargo fmt --check -p kawaro-cli` is clean, and `cli` (32), `settings` and `no_unix_net` are green.
  - Failing output (each command inside `systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200%`):
```
$ cargo test -j 2 -p kawaro-cli --bin kawaro
      6 error[E0425]: cannot find function `daemon_binary_in` in this scope
      4 error[E0425]: cannot find function `foreground_grant` in this scope
      5 error[E0425]: cannot find function `start_with` in this scope
      3 error[E0425]: cannot find function `wait_exit` in this scope
      1 error[E0425]: cannot find value `CREATE_BREAKAWAY_FROM_JOB` in this scope
      1 error[E0425]: cannot find value `CREATE_NEW_PROCESS_GROUP` in this scope
      1 error[E0425]: cannot find value `DETACHED_PROCESS` in this scope
error: could not compile `kawaro-cli` (bin "kawaro" test) due to 21 previous errors; 1 warning emitted

$ cargo test -j 2 -p kawaro-cli --test spawn_windows
test ac2_windows_spawn_checks_each_flag_against_windows_sys_at_compile_time ... FAILED
test ac2_ac5_windows_spawn_uses_creation_flags_and_allow_set_foreground_window ... FAILED
test ac2_windows_spawn_is_excluded_from_mutants_and_the_shared_files_are_not ... FAILED
test ac6_windows_sys_is_a_windows_only_dependency_of_the_cli ... FAILED
.../crates/cli/src/spawn/windows.rs must exist: No such file or directory (os error 2)
crates/cli/src/spawn/windows.rs must be in exclude_globs (story command M)
crates/cli/Cargo.toml needs [target.'cfg(windows)'.dependencies] (AC6)
test result: FAILED. 0 passed; 4 failed

$ env CARGO_TARGET_DIR=$HOME/.cache/kawaro-win-target cargo clippy -j 2 --target x86_64-pc-windows-gnu -p kawaro-core -p kawaro-engines --lib -p kawaro-cli --bins -- -D warnings   # W, AC6 and AC7
error[E0433]: cannot find `unix` in `os`        --> crates/cli/src/client.rs:79:18
error[E0433]: cannot find `unix` in `os`        --> crates/cli/src/client.rs:101:18
error[E0599]: no method named `process_group` found for mutable reference `&mut std::process::Command`  --> crates/cli/src/client.rs:87:10
error[E0599]: no method named `exec` found for struct `std::process::Command`  --> crates/cli/src/client.rs:102:45
error: could not compile `kawaro-cli` (bin "kawaro") due to 4 previous errors
```
- 2026-10-22, implementer:
  - `crates/cli/src/spawn.rs`: the three constants as `u32`; `FIRST_FLAGS = 0x0100_0208` and `RETRY_FLAGS = 0x208` as two literal values pinned by `const _: () = assert!(...)` against the constants (so there is no equivalent `|` to `^` mutant); `start_with` with a `match` and a `raw_os_error() == Some(5)` guard; `wait_exit` with `status()` and the exit code (`1` when there is no code, that is, a signal on Linux); `daemon_binary_in`; and `foreground_grant` (`Open` and `Drop` pass `server_pid`, the rest `None`). `start_with`, `wait_exit` and the constants carry `#[cfg_attr(not(windows), allow(dead_code))]`. The test module did not change: the two diffs from the tester's log are empty.
  - `crates/cli/src/spawn/windows.rs` (new): three `const _: () = assert!` against `windows_sys::Win32::System::Threading`, `start_detached` (`start_with` with `creation_flags(flags).spawn()`), and `allow_foreground` (`AllowSetForegroundWindow`; the result is ignored because its failure only means a hive without focus).
  - `crates/cli/src/client.rs`: `send` calls `foreground_grant(req, stream.server_pid())` first thing, that is, after `connect` in `request` and `stream` and before `writeln!`, and on Windows passes the result to `allow_foreground`. `ensure_daemon` builds the command once, then `process_group(0)` under `cfg(unix)` or `start_detached` under `cfg(windows)`. `exec_daemon` on Linux keeps its body as is under `cfg(unix)`, and on Windows it is `wait_exit` then `process::exit` with the code. `daemon_binary()` passes `EXE_SUFFIX` to `daemon_binary_in`.
  - `crates/cli/Cargo.toml`: `windows-sys 0.61` under `[target.'cfg(windows)'.dependencies]` with the two features `Win32_System_Threading` and `Win32_UI_WindowsAndMessaging`. `Cargo.lock`: the line `"windows-sys 0.61.2"` in the cli dependencies only. `.cargo/mutants.toml`: `spawn/windows.rs` in `exclude_globs`, and two patterns in `exclude_re` (deviation below). README in Arabic and English, and one CHANGELOG line.
  - Deviation: the first `M` round (16 mutants) left two surviving mutants in `client.rs`: `replace daemon_binary -> PathBuf with Default::default()` and `replace exec_daemon -> Result<(), String> with Ok(())`. The first is glue that reads `current_exe`, the decision is in the mutated `daemon_binary_in`, and `tests/e2e/run.sh` ("open starts the daemon") covers it; the second is `exec`, which replaces the process so no test returns from it, and its body did not change: it only entered the diff through the added `#[cfg(unix)]`. I added two narrow patterns for them in `exclude_re` with a comment, following the precedent of `serve` and `run`. If the manager thinks an integration test for `kawaro daemon` is better than the exclusion, it is for the tester; the difficulty is that `target/debug/kawarod` usually sits next to the binary, so it really gets started.
  - Gates (all inside `systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200%` with `-j 2`):
    - `C`: `cargo fmt --check` and `cargo clippy -j 2 --workspace --all-targets -- -D warnings` are clean.
    - `cargo test -j 2 --all`: 664 passed, 0 failed, 1 ignored, in 44 binaries. In cli: the `kawaro` binary 23, `cli` 32, `settings` 2, `no_unix_net` 1 and `spawn_windows` 4.
    - `W` is clean, and `cargo check -p kawaro-cli --target x86_64-pc-windows-gnu` is clean. `B`: `kawaro.exe` was built (release), and `objdump -p` lists `AllowSetForegroundWindow`, `GetNamedPipeServerProcessId`, `CancelIoEx` and `WaitNamedPipeW`.
    - `X`: clap, kawaro-core, kawaro-engines, serde_json and `windows-sys v0.61.2` (the only new one). `[[package]]` in `Cargo.lock`: 671.
    - `M` in a copy of the tree in the session folder, `CARGO_BUILD_JOBS=2 cargo mutants --jobs 1 --in-diff` on `git diff 34a3288 -- crates/cli/src/{client.rs,spawn.rs,main.rs}` with `-p kawaro-cli`: after the two exclusions, 14 mutants, 13 caught, 1 timeout (`replace send -> Result<(), String> with Ok(())`: the client waits for a reply to a request that was never written, so the timeout is a catch), 0 surviving.
    - `scripts/design-lint.sh` is clean. `tests/e2e/run.sh`: all pass (including S-019 AC7). `git diff --stat main...HEAD -- crates/ui` is empty. No long dash in the diff.
  - For the live check in S-052: the daemon surviving after the terminal closes (README and CHANGELOG state it as the design), the retry inside the job, and focus. `Settings` does not grant the foreground right, as the ADR specifies.
- 2026-10-22, reviewer: **PASS**. Every gate was re-run inside the scope (`MemoryMax=6G`, `-j 2`) from the `windows/0.6.0` tree at `2da2eab`:
  - `cargo fmt --check` is clean. `cargo clippy -j 2 --workspace --all-targets -- -D warnings` with no warning. `cargo test -j 2 --all`: 664 passed, 0 failed, 1 ignored, in 44 binaries; in cli: the `kawaro` binary 23 (including the story's eighteen tests), `spawn_windows` 4, `cli` 32, `settings` 2 and `no_unix_net` 1.
  - `W` (the separate `CARGO_TARGET_DIR`): `cargo clippy` for the `x86_64-pc-windows-gnu` target for core and engines `--lib` and cli `--bins` with no warning, and `cargo check -p kawaro-cli` and `cargo check -p kawaro-core -p kawaro-engines --lib` for the target are clean (AC7). `B`: `kawaro.exe` (release, 8786944 bytes); `objdump -p` lists the four names (`AllowSetForegroundWindow`, `GetNamedPipeServerProcessId`, `CancelIoEx` and `WaitNamedPipeW`), and the imported libraries all come from the system (bcryptprimitives, advapi32, api-ms-win-core-synch, combase, kernel32, ntdll, shell32, user32 and msvcrt), so no libgcc_s, no libwinpthread and no libstdc++.
  - `X`: clap, kawaro-core, kawaro-engines, serde_json and `windows-sys v0.61.2` as the only new one. `[[package]]` in `Cargo.lock` 671, and the diff is one line in the cli dependencies.
  - `M`: in a copy of the tree, not `--in-place`, `CARGO_BUILD_JOBS=2 cargo mutants --jobs 1 --in-diff` on `git diff c059a4d..HEAD -- crates/cli/src` with `-p kawaro-cli`: 14 mutants, 13 caught, 1 timeout (`send` with `Ok(())`: the client waits for a reply that was never requested, so the timeout is a catch), **0 surviving**. `--list --no-config` proves that the two new exclusions remove only `client.rs:74` (`daemon_binary`) and `client.rs:122` (`exec_daemon` with `Ok(())`), and that the `daemon_binary_in` mutant remains and is caught.
  - `scripts/design-lint.sh` clean. No long dash in `git diff a6bd519~1..HEAD`. `git diff --stat main...HEAD -- crates/ui` is empty. `git diff c059a4d..HEAD -- 'crates/*/tests/*'` is empty, the two inline test diffs in `spawn.rs` and `main.rs` from the tester's log are empty, and `main.rs` has not changed since `c059a4d`. `tests/e2e/run.sh` (after a release build): 37 PASS, 0 FAIL, including "open starts the daemon".
  - The criteria against the code: AC1: `start_with` retries once with `0x208` only when `raw_os_error() == Some(5)`, and any other error is returned without a retry; `FIRST_FLAGS` and `RETRY_FLAGS` are literals pinned by a `const assert` against the constants. AC2: three `const _: () = assert!` in `spawn/windows.rs` against `windows_sys::Win32::System::Threading`; Linux stays on `process_group(0).spawn()` as before. AC3: `wait_exit` uses `status()`; `exec_daemon` on Linux keeps its body literally as before under `cfg(unix)` (`exec`). AC4: `daemon_binary()` passes `EXE_SUFFIX`, and the empty directory when `current_exe` fails gives the bare name as before. AC5: `send` is called in `request` and `stream` only after `connect()`, `foreground_grant` is its first line, before `serde_json` and `writeln!`, and the grant goes to `allow_foreground` only under `cfg(windows)`; `server_pid()` on Linux is a constant `None`. No leak: `spawn().map(drop)` closes the process and thread handles on Windows, the retry happens only after `spawn` fails so there is no child before it, and Linux drops the `Child` as before. AC9: README in Arabic and English, and one CHANGELOG line from the implementer, so no new line from the reviewer.
  - The deviations are accepted: (1) `exclude_re` for `daemon_binary`: glue that reads `current_exe`, the decision is in the mutated `daemon_binary_in`, and the e2e "open starts the daemon" covers it; the pattern does not match `daemon_binary_in`. (2) `exclude_re` for `exec_daemon` with `Ok(())` only: `exec` does not return, the body did not change, and the Windows twin `wait_exit` is mutated; following the precedent of `serve` and `run`. (3) `-p kawaro-cli` in `M`: the whole diff is in cli, and no other crate depends on its binary, so no test is lost. (4) The CHANGELOG wording: accepted, the line describes the intent; the live check is in S-052.
  - Non-blocking notes, for S-052:
    - Low, `README.md:76`, `README.md:190-192` and `CHANGELOG.md:8`: the daemon surviving after the terminal closes is written as fact, and has not yet been checked live on Windows (deferred to S-052). The retry with `0x208` keeps the daemon inside the terminal's job by design, so if the job kills its processes on close (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`), the daemon dies with it; S-052 tests this exact case, and the text is qualified with it if it holds.
    - Info, `crates/cli/tests/settings.rs:6` and `crates/cli/tests/cli.rs`: `cargo clippy --target x86_64-pc-windows-gnu -p kawaro-cli --all-targets` fails because both use `std::os::unix`; this predates the story (`git show 34a3288` shows `std::os::unix` in both files) and is outside `W` (`--bins`), and it needs a story for cli tests on Windows.
    - Info, `crates/cli/src/spawn.rs:60`: cargo-mutants does not mutate `unwrap_or(1)` (a literal); on Windows `code()` is always `Some`, so there is no effect.
