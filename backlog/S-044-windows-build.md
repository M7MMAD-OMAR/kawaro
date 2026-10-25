# S-044 The Windows cross build, its gate, CI and the test image
Status: S-044a: done (reviewer: PASS, 2026-10-22); S-044b: done (reviewer: PASS, 2026-10-22, windows/0.6.0 branch at the owner's request); ADR 0005 is still proposed; merged into main locally on 2026-10-23 (not pushed)
Renumbered on 2026-10-23: this story was S-026 on windows/0.6.0 before the merge into main (main had already used S-022 and S-027). Commit messages and review-log hashes keep the old number.

Builds on ADR 0005 sections 1 and 9, and on S-040 to S-043 (before them `kawaro.exe` does not build for the target). For release 0.6.0. Does not touch `crates/ui`.

## Story
As a developer on Fedora without Windows, I want one command that builds `kawaro.exe` and the test files for Windows inside a bounded memory scope and rejects any shipped mingw library, and I want CI to check the code on real Windows, so that only a clean build reaches the machine.

## Split (in order, each substory one work day or less)
The ADR makes it one story, and it is larger than a day, so it was split:
| Story | Content | Criteria | Files |
| --- | --- | --- | --- |
| S-044a | The build script, the `objdump` check, the clippy gate for the target with all targets, and confining the Unix tests | AC1 to AC7 | `scripts/windows/{build.sh,check-dlls.sh,check.sh,README.md}`, `tests/windows/fixtures/{objdump-ok.txt,objdump-mingw.txt}`, `crates/cli/tests/{cli.rs,settings.rs}`, `crates/core/tests/{output.rs,contracts.rs}`, `crates/engines/src/office.rs` (the test module), `crates/engines/tests/{office_engine.rs,pdf_ghostscript.rs}`, and maybe `.cargo/config.toml` (AC2), `docs/engines.md`, `CHANGELOG.md` |
| S-044b | The two CI jobs, the ISO script, and `run-tests.cmd` | AC8 to AC12 | `.github/workflows/ci.yml`, `scripts/windows/{make-iso.sh,run-tests.cmd,README.md}`, `CHANGELOG.md` |

## Acceptance criteria
S-044a:
- AC1: `scripts/windows/build.sh` outside a scope re-runs itself inside `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200%`, and inside CI (`CI=true`) it runs directly. Its first printed line is `cgroup: <path>`, and the `memory.max` value of that path equals `6442450944`. It builds with `-j 2` and `CARGO_TARGET_DIR=/home/sbarah/.cache/kawaro-win-target` by default, produces `x86_64-pc-windows-gnu/release/kawaro.exe`, and exits with 0.
- AC2: `scripts/windows/check-dlls.sh <exe>` reads the `DLL Name` lines from `x86_64-w64-mingw32-objdump -p`, and accepts `--text <file>` for testing. `objdump-ok.txt` (system libraries only) gives 0 and the line `dll check: ok <name> <N> imports`. `objdump-mingw.txt` (with `libwinpthread-1.dll` and `libgcc_s_seh-1.dll`) gives 1 and lists both names. Any name that matches `^lib.*\.dll$`, case-insensitive, is rejected. On the real `kawaro.exe` it gives 0, and if it does not, a `.cargo/config.toml` with `-C target-feature=+crt-static` is added for the target alone, and that is recorded here.
- AC3: `scripts/windows/check.sh`, inside the same scope, runs `cargo clippy -j 2 --target x86_64-pc-windows-gnu -p kawaro-core -p kawaro-engines -p kawaro-cli --all-targets -- -D warnings` and exits with 0.
- AC4: the Unix-only tests are confined with `cfg(unix)`, as configuration, not as a change to the assertions: at file level `crates/cli/tests/cli.rs` and `settings.rs` (`UnixListener` servers and `PermissionsExt`), and at function level the dangling link test in `core/tests/output.rs:108`, `socket_path_lives_in_the_runtime_dir` in `core/tests/contracts.rs:228`, the `PermissionsExt` test in `engines/src/office.rs`, and the fake program tests with `sh` in `office_engine.rs` and `pdf_ghostscript.rs`. Every added line in the `git diff` of these files is a `cfg` line, and no line is deleted. The number of passing tests in `cargo test --workspace` on Linux before and after the change is equal.
- AC5: `cargo test --no-run -j 2 --target x86_64-pc-windows-gnu -p kawaro-core -p kawaro-engines -p kawaro-cli` inside the scope succeeds, and the number of `.exe` test files it lists equals the number of test targets in the three packages (the number is recorded here).
- AC6: no new package: the `[[package]]` count in `Cargo.lock` stays 671, and there is no change to `Cargo.lock` in this story (`git diff --quiet -- Cargo.lock`).
- AC7: `docs/engines.md` records the build tools that are not shipped: mingw64-gcc 16.2.1 and genisoimage (both installed), and their licences. `scripts/windows/README.md` explains the commands in Arabic and English.

S-044b:
- AC8: `ci.yml` adds two jobs and does not change the `check` job: `git diff -U0 main...HEAD -- .github/workflows/ci.yml` has only `+` lines. `windows-cross` on `ubuntu-24.04` installs `mingw-w64` and the `x86_64-pc-windows-gnu` target, then runs `check.sh`, `build.sh` and `check-dlls.sh`. `windows-native` on `windows-latest` installs FFmpeg (`choco install ffmpeg -y`), because `ffmpeg_engine.rs:123` asserts it is present and does not skip, then runs `cargo test -p kawaro-core -p kawaro-engines -p kawaro-cli`. The LibreOffice and Ghostscript tests skip themselves when the program is missing (`office_engine.rs:37`, `pdf_ghostscript.rs:91`), so they are not installed.
- AC9: `scripts/windows/make-iso.sh` builds with `genisoimage -J -R` the image `/home/sbarah/.cache/kawaro-win-target/kawaro-win-tests.iso`, holding `bin/kawaro.exe`, the test files in `tests/`, `run-tests.cmd` and `SHA256SUMS`. `isoinfo -J -f -i <iso>` lists all of these, the number of lines in `SHA256SUMS` equals the number of executables, and the `sha256sum` of `run-tests.cmd` extracted with `isoinfo -J -x` equals the original.
- AC10: `run-tests.cmd` has CRLF line endings (`file` says `with CRLF line terminators`). It runs every `tests\*.exe` with `--test-threads=1`, and writes to `%USERPROFILE%\kawaro-tests\results.txt` one line `<exe> <errorlevel>` per file, with the last line `total N failed F`, and the full output in `log.txt` next to it. It does not write to the disc.
- AC11: `scripts/windows/README.md` adds, in Arabic and English, the attach command `virsh -c qemu:///session attach-disk win11 <iso> sdb --type cdrom --mode readonly`, without running it in this story.
- AC12 (added by the manager on 2026-10-22 from the S-044a reviewer notes): `scripts/windows/check-dlls.sh` exits with 2 for every input error (a `--text` file that does not exist, or input with no `DLL Name` line at all), and keeps 1 for rejection alone. The `windows-cross` job sets `CARGO_TARGET_DIR` inside the workspace. `docs/engines.md` attributes `x86_64-w64-mingw32-objdump` to mingw64-binutils with its version and licence, and records the `mingw-w64` package in Ubuntu. The files `check-dlls.sh` and `docs/engines.md` are among the S-044b files.

For each substory:
- One line for the whole story in `CHANGELOG.md` under `[Unreleased]`, in English with Arabic text in «»: S-044a adds it, and S-044b edits the same line.
- `git diff --stat main...HEAD -- crates/ui` is empty, and `C` with no warning on Linux.
- `scripts/design-lint.sh` is clean on the new files (no long dash).
- No new production Rust code here (`cfg` lines in the tests only), so no mutants.

## Check commands
Read-only tools (`grep`, `file`, `objdump`, `isoinfo` and `sha256sum`) run outside the scope, and `make-iso.sh` builds nothing.
- `C`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% sh -c 'cargo fmt --check && cargo clippy -j 2 --workspace --all-targets -- -D warnings'`
- `L`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% cargo test -j 2 --workspace`
- `S`: `scripts/windows/build.sh`, then `scripts/windows/check-dlls.sh /home/sbarah/.cache/kawaro-win-target/x86_64-pc-windows-gnu/release/kawaro.exe`, then `scripts/windows/check.sh` (each enters the scope by itself)
- `T`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_TARGET_DIR=/home/sbarah/.cache/kawaro-win-target cargo test --no-run -j 2 --target x86_64-pc-windows-gnu -p kawaro-core -p kawaro-engines -p kawaro-cli`

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | `S`: the `cgroup:` line, `cat /sys/fs/cgroup<path>/memory.max` gives 6442450944, and `kawaro.exe` exists | build |
| AC2 | `check-dlls.sh --text` on the two planted files, with exit codes 0 and 1 and their text, then on `kawaro.exe` | integration |
| AC3 | `check.sh` exits with 0 | build |
| AC4 | The `git diff` of the seven files has added `cfg` lines only; and the number of `test result: ok` in `L` before and after | review, unit |
| AC5 | `T` and counting the `.exe` files | build |
| AC6 | `grep -c '^\[\[package\]\]' Cargo.lock` gives 671, and `git diff --quiet -- Cargo.lock` | review |
| AC8 | The `git diff -U0` command above, and the reviewer reads the two jobs | review |
| AC9 | `make-iso.sh`, then `isoinfo -J -f`, `isoinfo -J -x` and `sha256sum` | integration |
| AC10 | `file scripts/windows/run-tests.cmd`, and the reviewer reads the file | review |
| AC7, AC11 | The reviewer reads engines.md, README and CHANGELOG.md | review |
| AC12 | Running `check-dlls.sh` on a missing file and on empty input, and reading ci.yml and engines.md | unit |

## Live check deferred to S-052
- The first success of the two jobs in CI: `ci.yml` runs only on a push to main or a pull request, and pushing and merging are outside this story and need the owner's decision.
- `run-tests.cmd` in the machine: the file and its last line, and the number of failures.
- Attaching the ISO to the machine and reading it from the disc drive.

## Out of scope
- `kawarod.exe` and checking its libraries with `check-dlls.sh`: S-045 (the daemon depends on ui).
- The installer and the portable archive: S-051.
- wine, and the guest tool `qemu-guest-agent`: they need the owner's approval.

## Review log
- 2026-10-22, planner: the ADR asks for an `objdump` check on `kawarod.exe` in the first build story, but it does not link before S-045, so `kawaro.exe` is checked here and `kawarod.exe` is added in S-045. The "what the owner needs" list asks for permission for xorriso or genisoimage, but genisoimage and isoinfo are installed on the host, so this story needs no permission.
- 2026-10-22, tester (S-044a):
  - Tests, all shell, outside the cargo workspace so `cargo test` on real Windows (S-044b) never runs bash and the reviewer's `L` starts no nested build:
    - `tests/windows/test_scripts.sh` (light, no real build, a few seconds): `ac1_build_sh_reexecs_in_a_6g_scope_with_j2_and_prints_its_cgroup`, `ac2_check_dlls_accepts_system_dlls_and_rejects_lib_dlls`, `ac3_check_sh_runs_windows_clippy_all_targets_in_a_6g_scope`, `ac4_unix_only_tests_carry_cfg_unix_and_nothing_else_changes`, `ac7_engines_md_and_readme_document_the_build_tools`, `no_long_dash_and_shell_lint_for_the_new_scripts` (`bash -n`, `shellcheck`, no em or en dash in the three scripts and the README).
    - `tests/windows/test_heavy.sh` (the real `S` and `T`; refuses to start while `pgrep` sees cargo or cargo-mutants): `ac1_real_build_sh_runs_in_a_6g_scope_and_produces_kawaro_exe`, `ac2_check_dlls_passes_on_the_real_kawaro_exe`, `ac3_real_check_sh_exits_0`, `ac5_windows_test_binaries_build_one_exe_per_test_target`.
    - `tests/windows/lib.sh` (helpers, sources `tests/integrations/lib.sh` for the asserts), fixtures `tests/windows/fixtures/objdump-ok.txt` and `objdump-mingw.txt`, trimmed from the real `x86_64-w64-mingw32-objdump -p` of the current `kawaro.exe` (header, Import Tables, the trailing empty entry; the path on the first line is cut to `kawaro.exe`). `objdump-ok.txt` has 9 distinct system DLLs; `objdump-mingw.txt` is the same plus `libgcc_s_seh-1.dll` and `libwinpthread-1.dll`.
    - Both scripts must run from outside any memory scope (a plain terminal or the Claude app scope, whose `memory.max` is `max`).
  - Contract pinned by the light tests (the implementer reads this, not the tests, to know what is checked):
    - AC1 and AC3: the scripts call `cargo` from `PATH` (a fake `cargo` first on `PATH` logs each call's argv, its cgroup, that cgroup's `memory.max`, `CARGO_TARGET_DIR` and `CARGO_BUILD_JOBS`). The scope must keep `PATH` (a `systemd-run --scope` child inherits the environment). Two jobs are accepted as `-j 2`, `-j2`, `--jobs 2`, `--jobs=2` or `CARGO_BUILD_JOBS=2`. The build call has `build`, `--release` and `--target x86_64-pc-windows-gnu` (or `--target=`). A `CARGO_TARGET_DIR` from the caller is kept, and with it unset the build uses `/home/sbarah/.cache/kawaro-win-target` (env or `--target-dir`, both read). The `cgroup:` line names the cgroup the cargo call runs in, with `memory.max` 6442450944, and it is not the caller's cgroup. With `CI=true`, `systemd-run` is never called (a fake that exits 99 is first on `PATH`) and the `cgroup:` line is the caller's own. AC3 checks `clippy` with `-p kawaro-core -p kawaro-engines -p kawaro-cli --all-targets` and `-- -D warnings` in the 6G cgroup.
    - AC2: `--text FILE` on `objdump-ok.txt` exits 0 and prints the line `dll check: ok <name> 9 imports`, where `<name>` may be `kawaro.exe` (from the objdump header), `objdump-ok.txt` or the path given, and N counts `DLL Name` lines (DLLs, not functions). `objdump-mingw.txt` exits 1, names both DLLs, prints no `dll check: ok`, and does not name `kernel32` or `msvcrt`. A generated copy with `DLL Name: LIBSTDC++-6.DLL` exits 1 and names it (case-insensitive rule). Without `--text`, the script runs `x86_64-w64-mingw32-objdump -p <exe>` from `PATH` (a fake that prints `objdump-mingw.txt` gives exit 1). The heavy test on the real exe asks only for exit 0 and the line shape, because the real exe imports `ntdll.dll` twice and `KERNEL32.dll` beside `kernel32.dll`.
    - AC4: inner `#![cfg(unix)]` lines in `crates/cli/tests/cli.rs` and `settings.rs`; `#[cfg(unix)]` among the attributes directly above `ac7_dangling_symlink_counts_as_taken` (output.rs), `socket_path_lives_in_the_runtime_dir` (contracts.rs), `office_find_program_respects_name_order_and_exec_bit` (office.rs unit tests), the four `office_engine.rs` tests that run `/bin/sh` as the program (`office_timeout_kills_the_process_group`, `office_cancel_kills_the_process_group`, `office_already_cancelled_starts_nothing` through `fake_engine`, and `office_missing_program_is_unavailable_error`), and `pdf_ghostscript_identity_and_capabilities` (`with_program("/bin/sh")`). `git diff -U0 5be5270` of the seven files has no `-` line, and every `+` line is a `#[cfg(...)]`, `#![cfg(...)]` or `#[cfg_attr(...)]` line, so helpers left unused on Windows may also get a cfg line.
    - AC5 (heavy): T exits 0, and the number of distinct `(... .exe)` paths it prints equals the lib, bin and test targets with `test = true` in the three packages from `cargo metadata --no-deps`: today 35 (core 17, engines 13, cli 5). `cli.rs` and `settings.rs` under `#![cfg(unix)]` still build an empty test exe, so 35 holds.
    - AC1 heavy: the first stdout line is read from a fifo and `memory.max` read at once, because the scope's cgroup is removed when the build ends; `file` must say `PE32+ executable ... x86-64` for the exe.
  - The cfg(unix) edits in the seven files are left to the implementer. The story names them in the S-044a file list without giving them to the tester, and AC4 is how they are checked. Adding them here would turn AC4, and maybe AC5, green before implementation. The manager must give the implementer explicit leave for these cfg lines only, since the chain forbids the implementer to edit tests.
  - Ambiguities and choices:
    - (1) "N imports" is read as the count of `DLL Name` lines; the ok fixture has no duplicates, so raw and distinct counts agree there.
    - (2) `<name>` with `--text` is open (see AC2 above).
    - (3) For office_engine.rs and pdf_ghostscript.rs, the AC's "fake programs with sh" is pinned to the five tests above, the ones that pass `/bin/sh` as the program. `office_run_child_*` in office.rs are not required to be guarded, because the AC names only the `PermissionsExt` test there.
    - (4) The AC1 "outside a scope" detection is the implementer's choice (env marker or memory.max); only the observable result is tested.
    - (5) AC6 already holds today (671 `[[package]]`, `Cargo.lock` clean), so a test for it would pass before implementation. AC6 is left to the reviewer: `grep -c '^\[\[package\]\]' Cargo.lock` and `git diff --quiet 5be5270 -- Cargo.lock`. The same goes for the CHANGELOG line, `git diff --stat main...HEAD -- crates/ui`, `C` and design-lint.
    - (6) AC7 is review in the table. The light test adds only a static floor: engines.md has a line with `mingw64-gcc`, `16.2.1` and `GPL`, and one with `genisoimage` and `GPL`. The README exists, names the three scripts, and has Arabic and English text.
  - AC4 baseline for the equal pass count in `L`: 697 passed, 0 failed, 1 ignored at `d6779fb` (S-043b reviewer). `d6779fb..5be5270` touches only the backlog, and this commit adds no Rust, so the count holds before the implementer's cfg lines.
  - Logic check: in a throwaway `git clone --shared` in the session scratch dir (deleted, not merged), I wrote stub scripts and added the cfg lines. With them the light suite passed (63 checks). Mutating the stub made it fail at the expected checks: dropping `-j 2`; skipping the scope in check.sh (memory.max `max`); dropping `<name>`; changing an assertion in contracts.rs, which gave a `-` and a non-cfg `+` line. The heavy suite was not run against the stub (see below).
  - Failing output (light suite, here, before implementation):
```
$ bash tests/windows/test_scripts.sh
== AC1: build.sh outside a scope
  FAIL: AC1: missing or not executable scripts/windows/build.sh
== AC2: check-dlls.sh
  FAIL: AC2: missing or not executable scripts/windows/check-dlls.sh
== AC3: check.sh
  FAIL: AC3: missing or not executable scripts/windows/check.sh
== AC4: cfg(unix) guards
  FAIL: AC4: crates/cli/tests/cli.rs starts with the inner attribute #![cfg(unix)]
  FAIL: AC4: crates/cli/tests/settings.rs starts with the inner attribute #![cfg(unix)]
  FAIL: AC4: output.rs ac7_dangling_symlink_counts_as_taken has #[cfg(unix)]
  FAIL: AC4: contracts.rs socket_path_lives_in_the_runtime_dir has #[cfg(unix)]
  FAIL: AC4: office.rs office_find_program_respects_name_order_and_exec_bit has #[cfg(unix)]
  FAIL: AC4: office_engine.rs office_timeout_kills_the_process_group (a /bin/sh fake program) has #[cfg(unix)]
  FAIL: AC4: office_engine.rs office_cancel_kills_the_process_group (a /bin/sh fake program) has #[cfg(unix)]
  FAIL: AC4: office_engine.rs office_already_cancelled_starts_nothing (a /bin/sh fake program) has #[cfg(unix)]
  FAIL: AC4: office_engine.rs office_missing_program_is_unavailable_error (a /bin/sh fake program) has #[cfg(unix)]
  FAIL: AC4: pdf_ghostscript.rs pdf_ghostscript_identity_and_capabilities (/bin/sh) has #[cfg(unix)]
== AC7: docs
  FAIL: AC7: engines.md names mingw64-gcc 16.2.1 with its licence
  FAIL: AC7: engines.md names genisoimage with its licence
  FAIL: AC7: missing scripts/windows/README.md
== AC1 AC2 AC3: bash -n, shellcheck, no long dash
  FAIL: lint: missing scripts/windows/build.sh
  FAIL: lint: missing scripts/windows/check-dlls.sh
  FAIL: lint: missing scripts/windows/check.sh
FAIL S-044a windows scripts (light) (19 of 21 checks failed)
```
  - Heavy suite, here, before implementation. Ran when no other cargo was running. T fails at the first crate, and cargo prints no `Executable` line on failure, hence 0 listed:
```
$ bash tests/windows/test_heavy.sh
== AC1: real build.sh (S)
  FAIL: AC1: missing or not executable scripts/windows/build.sh
== AC2: check-dlls.sh on the real kawaro.exe
  FAIL: AC2: missing or not executable scripts/windows/check-dlls.sh
== AC3: real check.sh
  FAIL: AC3: missing or not executable scripts/windows/check.sh
== AC5: cargo test --no-run for Windows (T)
  FAIL: AC5: exit code: got [101] want [0]
  AC5: test targets 35, listed .exe files 0
  FAIL: AC5: listed .exe files equal test targets: got [0] want [35]
      1 error: could not compile `kawaro-cli` (test "settings") due to 1 previous error
      1 error[E0433]: cannot find `unix` in `os`
FAIL S-044a windows build (heavy) (5 of 6 checks failed)
```
  - BLOCKER for the manager: the AC4 file list does not cover every Windows compile error under `--all-targets`. Full list from `cargo check --tests --keep-going -j 2 --target x86_64-pc-windows-gnu -p kawaro-core -p kawaro-engines -p kawaro-cli`, inside the scope with the Windows target dir:
    - Listed in AC4: `cli/tests/cli.rs` (3 errors), `cli/tests/settings.rs` (1), `core/tests/output.rs` (1), `engines/src/office.rs` unit tests (3).
    - NOT listed:
      - `crates/core/tests/transport.rs`, 4 errors: `Endpoint::from_path` at lines 35, 77 and 272, and `std::os::unix` at line 267.
      - `crates/core/src/output_impl.rs:401`, 1 error: `std::os::unix` inside its `#[cfg(test)] mod tests`.
    - AC5 and AC3 (clippy `--all-targets`) cannot pass without cfg lines in these two files too. The implementer may not edit tests, and these two are outside the story. So the story needs its AC4 list (and the S-044a file list) extended before implementation; I did not extend the AC4 test on my own.
- 2026-10-22, implementer (S-044a):
  - Manager decisions, recorded as given:
    1. "You are allowed to add cfg lines only (`#[cfg(...)]`, `#![cfg(...)]`, `#[cfg_attr(...)]`, no deleted lines, no changed assertion) to the seven test files AC4 lists. This is the configuration the story describes, not a test edit."
    2. "The S-044a file list and AC4's scope are extended with crates/core/tests/transport.rs (unix-only tests using Endpoint::from_path and std::os::unix) and the test module of crates/core/src/output_impl.rs (line ~401), under the same cfg-lines-only rule, so AC3 and AC5 can pass. The AC4 test itself covers the seven files; the reviewer checks the two extra files by diff."
  - Scripts: `scripts/windows/build.sh` and `check.sh` re-exec through `systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200% env KAWARO_WIN_SCOPE=1 "$0"` unless `CI=true` or that marker is set, print `cgroup: <path>` from `/proc/self/cgroup`, default `CARGO_TARGET_DIR` to `/home/sbarah/.cache/kawaro-win-target` and keep a caller's, then exec `cargo build -j 2 --release --target x86_64-pc-windows-gnu -p kawaro-cli` or the AC3 clippy line. `check-dlls.sh` counts `DLL Name` lines (raw, not distinct), names the file by its basename, and rejects `^lib.*\.dll$` case-insensitive with exit 1; usage or objdump failure is exit 2. `scripts/windows/README.md` in English and Arabic.
  - AC2 on the real exe: `dll check: ok kawaro.exe 11 imports`, exit 0. No mingw DLL, so no `.cargo/config.toml` and no `crt-static`.
  - AC4 cfg lines beyond the tester's named tests, all cfg lines only, needed by Windows clippy `-D warnings` once the guarded tests drop out:
    - `office_engine.rs`: `#[cfg(unix)]` on `fake_engine`, `read_pid`, `wait_gone` and `use std::ffi::OsString;`; `#[cfg_attr(not(unix), allow(unused_imports))]` on `use std::path::{Path, PathBuf};`.
    - `pdf_ghostscript.rs`: `#[cfg_attr(not(unix), allow(unused_imports))]` on the `kawaro_core` use line (`MediaKind`).
    - `transport.rs` (decision 2): `#[cfg(unix)]` on `ep`, `listen`, `read_line`, the `Path`, `thread` and `time` imports, and on every test that uses `Endpoint::from_path` or a socket (`ac1_endpoint_from_path_displays_the_path`, `ac1_ac3_server_pid_is_none_on_unix`, `ac3_*`, `ac4_*`, `ac6_*`), plus `ac1_endpoint_on_unix_is_the_socket_path`, which compiles on Windows but asserts the Unix socket path and would fail on real Windows in S-044b; `#[cfg_attr(not(unix), allow(unused_imports))]` on the three grouped use lines. Left on Windows: `ac1_stream_is_read_write_send_and_listener_is_send`, `ac1_socket_name_and_socket_path_keep_their_value` and the three `ac2_pipe_name*` tests.
    - `output_impl.rs` (decision 2): `#[cfg(unix)]` on `ac7_linux_place_refuses_a_dangling_link_name`.
    - `git diff -U0 5be5270` of all nine files: no `-` line, every `+` line is a cfg line.
  - Note for S-044b: `processes_mentioning` in `office_engine.rs` reads `/proc`; it compiles on Windows and is reached only by `office_cancel_stops_real_libreoffice_and_cleans_up`, which skips itself without LibreOffice.
  - AC5 count: 35 test executables for 35 test targets (core 17, engines 13, cli 5).
  - Results: light suite `PASS (63 checks)`; heavy suite `PASS (11 checks)` in 56 s, run from the app scope (memory.max `max`) with no other cargo; `cargo fmt --check` and Linux `cargo clippy -j 2 --workspace --all-targets -- -D warnings` clean; `cargo test -j 2 --workspace` 697 passed, 0 failed, 1 ignored (equal to the baseline); `shellcheck scripts/windows/*.sh` clean; `scripts/design-lint.sh` clean; AC6 671 `[[package]]`, `Cargo.lock` unchanged; `crates/ui` untouched.
  - One Windows clippy attempt overlapped with a `cargo build --release` another session started at the same moment; it failed on a real warning and was re-run alone. Every later run, the heavy suite and the Linux gates started with `pgrep -x cargo` empty.
- 2026-10-22, reviewer (S-044a): **PASS**.
  - Light suite `tests/windows/test_scripts.sh`: PASS (63 checks). Heavy suite `tests/windows/test_heavy.sh` from the app scope (memory.max `max`), `pgrep -x cargo` empty: PASS (11 checks), incremental (0.95 s, the implementer's artifacts were fingerprinted fresh). AC5: 35 test exe files for 35 test targets.
  - AC1 run by hand through a fifo: first line `cgroup: .../app.slice/run-p3239487-i20022953.scope`, its `memory.max` 6442450944 and `cpu.max` `200000 100000`; exit 0; `file` says `PE32+ executable ... x86-64`.
  - AC2 on the real exe: `dll check: ok kawaro.exe 11 imports`, exit 0 (no `lib*.dll`, so no `.cargo/config.toml` is right). `objdump-mingw.txt`: exit 1 with both DLLs. AC3 `check.sh`: exit 0 in its own 6G scope.
  - `cargo fmt --check` and `cargo clippy -j 2 --workspace --all-targets -- -D warnings`: clean. `cargo test -j 2 --all`: 697 passed, 0 failed, 1 ignored in 45 suites, equal to the baseline (AC4).
  - `git diff -U0 5be5270` of the nine files: 0 deleted lines, 35 added lines, all cfg lines (26 `#[cfg(unix)]`, 2 indented `#[cfg(unix)]`, 2 `#![cfg(unix)]`, 5 `#[cfg_attr(not(unix), allow(unused_imports))]`). `cargo mutants --list --in-diff` on the crates diff: 0 mutants (cfg lines only).
  - AC6: 671 `[[package]]`, `Cargo.lock` identical to `5be5270` (it differs from main only through S-040b, S-042 and S-043a). `crates/ui` diff against main empty. `tests/windows/*` unchanged since e6fd108. `shellcheck scripts/windows/*.sh` and `scripts/design-lint.sh` clean; no em or en dash in `main...HEAD`.
  - Scripts: re-exec through `systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200%` unless `CI=true` or the `KAWARO_WIN_SCOPE` marker; `"$0" "$@"` quoted; `-j 2`; caller `CARGO_TARGET_DIR` kept. CHANGELOG line present (added by the implementer, not duplicated).
  - Deviations accepted: extra cfg and `cfg_attr` lines on helpers and imports, the guarded `ac1_endpoint_on_unix_is_the_socket_path` (asserts the Unix socket path), and no `.cargo/config.toml`; all inside the manager's cfg-lines-only decisions.
  - Findings, none blocking:
    - Low, `docs/engines.md:285`: `x86_64-w64-mingw32-objdump` is called the tool of mingw64-gcc, but `rpm -qf` gives mingw64-binutils 2.46.0 (GPL-3.0-or-later and others), which is not recorded. Fix in S-044b, which must record Ubuntu's `mingw-w64` anyway; add `docs/engines.md` to its file list.
    - Low, `scripts/windows/check-dlls.sh:12`: `--text` on a missing file exits 1 (the rejection code), while the header at line 8 says input errors are exit 2.
    - Low, `scripts/windows/check-dlls.sh:20-28`: input with no `DLL Name` line prints `ok <name> 0 imports` and exits 0, so the gate fails open on unparseable output; worth a guard before CI (S-044b) uses another objdump build.
    - Info, `scripts/windows/build.sh:9`, `check.sh:8`: a caller with `KAWARO_WIN_SCOPE` already set skips the scope.
    - Info, `scripts/windows/build.sh:14-15`, `check.sh:13-14`: a relative caller `CARGO_TARGET_DIR` is resolved after the `cd` to the repo root.
    - Info for S-044b: with `CI=true` the default target dir is still `/home/sbarah/.cache/...`, which a GitHub runner cannot create; the CI job should set `CARGO_TARGET_DIR`.
- 2026-10-22, tester (S-044b):
  - Test file `tests/windows/test_ci_iso.sh` (light: no build, no VM, no network, a few seconds; needs `python3` with PyYAML, `genisoimage`, `isoinfo`, `file`; sources `tests/windows/lib.sh`). `tests/windows/test_scripts.sh` is untouched and stays the S-044a regression suite. Functions:
    - `ac8_ci_adds_windows_cross_and_windows_native_and_leaves_check_alone`
    - `ac9_make_iso_builds_a_joliet_rock_ridge_iso_with_exes_sums_and_runner`
    - `ac10_run_tests_cmd_is_crlf_runs_every_test_exe_and_writes_results_in_userprofile`
    - `ac11_readme_gives_the_virsh_attach_command_in_both_languages`
    - `ac12_check_dlls_exits_2_on_input_errors_and_docs_name_binutils_and_ubuntu_mingw`
    - `lint_no_long_dash_and_shellcheck_for_s026b_files` (`bash -n` and `shellcheck` on `make-iso.sh` and `check-dlls.sh`; no em or en dash in `make-iso.sh`, `run-tests.cmd`, the README, `ci.yml`, `engines.md`)
  - Contract pinned by the tests (the implementer reads this, not the tests):
    - AC8: the diff base is `git merge-base main HEAD` (equal to `main...HEAD`, stable while main moves, and it sees uncommitted work). `git diff -U0 <base> -- .github/workflows/ci.yml` has added lines and no `-` line; the parsed `jobs.check` equals the base's. `windows-cross`: `runs-on: ubuntu-24.04`; a `run` step with `mingw-w64` and `install`; the target from `rustup target add x86_64-pc-windows-gnu` or the toolchain action's `targets:`/`target:`; `run` steps containing `scripts/windows/check.sh`, `scripts/windows/build.sh` and `scripts/windows/check-dlls.sh`, none before the mingw and target steps, `check-dlls.sh` after `build.sh`, and its step text contains `x86_64-pc-windows-gnu/release/kawaro.exe`. `windows-native`: `runs-on: windows-latest`; a step containing `choco install ffmpeg -y` before one whose whitespace-normalised `run` contains `cargo test -p kawaro-core -p kawaro-engines -p kawaro-cli`; no `libreoffice`, `ghostscript` or `gsview` in its `run` steps. `run` text is matched by substring, so multi-line blocks are fine.
    - AC12 in ci.yml: `CARGO_TARGET_DIR` set in `env` at workflow or `windows-cross` job level, or on the `build.sh`/`check-dlls.sh` steps; each value contains `github.workspace` or is relative (not `/`, `~` or `$HOME`). The job text does not name `/home/sbarah`. A `$GITHUB_ENV` line is not read.
    - AC9: `make-iso.sh` takes no argument, keeps a caller's `CARGO_TARGET_DIR` (default `/home/sbarah/.cache/kawaro-win-target`, as build.sh) and writes `$CARGO_TARGET_DIR/kawaro-win-tests.iso`. Inputs: `$CARGO_TARGET_DIR/x86_64-pc-windows-gnu/release/kawaro.exe` to `/bin/kawaro.exe`, and the test programs that `cargo test --no-run --target x86_64-pc-windows-gnu ...` builds in `$CARGO_TARGET_DIR/x86_64-pc-windows-gnu/debug/deps/` to `/tests/` under their own names. `/tests/` holds the test programs only, one per test target (35 today), and not every `deps/*.exe`: the real cache has 36 there today, the extra one being the CLI binary that `cargo test` builds for the integration tests (a second `kawaro-<hash>.exe`, hardlinked to `debug/kawaro.exe`), and stale hashes can pile up. How to select them is the implementer's choice. The fake tree has five `name-<16hex>.exe` test programs with distinct bytes plus a `.d`, a `.rlib` and a host build script, none of which may land in `/tests/`. Checked: exit 0; no `cargo` call (a logging fake cargo is first on `PATH`); `file` says ISO 9660; `isoinfo -d` prints `Joliet with UCS level N found` and `Rock Ridge signatures version N found`; `isoinfo -J -f` lists `/bin/kawaro.exe`, every planted `/tests/*.exe`, `/run-tests.cmd`, `/SHA256SUMS`; the ISO has exactly 6 `.exe`; `SHA256SUMS` has one line per `.exe` and, for each, the line naming its base name carries the sha256 of the bytes extracted with `isoinfo -J -x` (path form inside SHA256SUMS is free, `/` or `\`); `/bin/kawaro.exe` hashes as the release file; extracted `/run-tests.cmd` hashes as `scripts/windows/run-tests.cmd`; the real cache ISO's mtime and size (or absence) are unchanged.
    - AC10 (static; executing it needs wine or the VM, both outside this story, so it is live in S-052): `file` says `with CRLF line terminators` and every line ends in CR LF; the text has `tests\*.exe`, `--test-threads=1`, `errorlevel`, `%USERPROFILE%\kawaro-tests`, `results.txt`, `log.txt`, and `total ... failed`; no `>`/`>>` onto `%~dp0`, `%cd%` or a drive letter, and no `mkdir`/`md` on `%~dp0`.
    - AC11: the regex `virsh -c qemu:///session attach-disk win11 <one word> sdb --type cdrom --mode readonly` (the word may be `<iso>` or the real path) appears under `## English` and again under `## العربية`. virsh is never run.
    - AC12: `check-dlls.sh` exits 2 and prints no `dll check: ok` for `--text` on a missing file, on an empty file, on `objdump-ok.txt` with its `DLL Name` lines removed, and when a fake `x86_64-w64-mingw32-objdump` prints that header-only text and exits 0 (the fail-open case). `objdump-mingw.txt` still exits 1 and `objdump-ok.txt` 0. engines.md: a table row starting `| mingw64-binutils |` with the version from `rpm -q mingw64-binutils` (2.46.0 here), `GPL` and `x86_64-w64-mingw32-objdump`; the `| mingw64-gcc |` row no longer mentions objdump; a line with `mingw-w64` and `Ubuntu` (no version asked).
  - Not testable here: the CI jobs only run on a push to main or a pull request, and pushing is outside this story, so their first green run (and whether `choco install ffmpeg -y` and the mingw steps really work on the runners) is the deferred S-052 check. AC8's YAML structure is all this suite can see.
  - Left to the reviewer: the CHANGELOG line (S-044b edits the S-044 line), `git diff --stat main...HEAD -- crates/ui`, `C`, design-lint, and reading the two jobs and `run-tests.cmd` as the table says.
  - Ambiguities and choices:
    - (1) How make-iso.sh finds its inputs is not in the AC; pinned as above, following build.sh's `CARGO_TARGET_DIR`. The light suite's fake tree has only test programs, so it cannot see a plain `deps/*.exe` glob take the CLI binary; the heavy test below does.
  - Heavy test `tests/windows/test_heavy_iso.sh`, `ac9_real_make_iso_carries_exactly_the_test_programs_t_lists`: refuses to start while `pgrep` sees cargo or cargo-mutants; runs T inside the 6G scope (incremental after `test_heavy.sh`) and takes the `.exe` base names it lists; runs `make-iso.sh` with the default target dir (the AC's own output path in the cache); asserts the `/tests/*.exe` names on the ISO are exactly T's list, `/bin/kawaro.exe` hashes as the release build, `/run-tests.cmd` is listed, and SHA256SUMS has one line per `.exe`. Run it after `tests/windows/test_heavy.sh`.
    - (2) "Number of executables" for SHA256SUMS is read as every `.exe` on the ISO: `kawaro.exe` plus the test programs.
    - (3) AC8's order "check.sh, build.sh, check-dlls.sh" is pinned only as: all after setup, check-dlls after build.
    - (4) AC12 "input error" pinned to the four cases above; an objdump that fails is not tested.
    - (5) Joliet keeps names up to 64 characters; real test names are short (`name-<16hex>.exe`, longest stem 15), so no long-name case.
  - Logic check: in a throwaway `git clone --shared` in the session scratch dir (deleted, not merged), stub versions of the two jobs, make-iso.sh, run-tests.cmd, the README command, a check-dlls.sh with exit 2 and the engines.md rows made the suite pass (78 checks). Mutations caught: dropping `-R` (Rock Ridge check; a first loose `grep -i "rock ridge"` matched `NO Rock Ridge present` and was tightened), dropping `-J`, a short SHA256SUMS, LF line ends, an edited `check` job (a `-` line and a dict diff), and `CARGO_TARGET_DIR` under `/home/sbarah`.
  - Failing output (here, before implementation):
```
$ bash tests/windows/test_ci_iso.sh
== AC8: ci.yml
  FAIL: AC8: ci.yml has added lines against the merge base (got 0)
  FAIL: AC8: windows-cross exists: no job windows-cross (jobs: ['check'])
  FAIL: AC8: windows-cross on ubuntu-24.04: windows-cross runs-on is not ubuntu-24.04
  FAIL: AC8: windows-cross installs mingw-w64: no step installs mingw-w64
  FAIL: AC8: windows-cross adds x86_64-pc-windows-gnu: no step adds the x86_64-pc-windows-gnu target
  FAIL: AC8: windows-cross runs check.sh, build.sh, check-dlls.sh: windows-cross does not run ['scripts/windows/check.sh', 'scripts/windows/build.sh', 'scripts/windows/check-dlls.sh']
  FAIL: AC8: check-dlls.sh reads the built kawaro.exe: check-dlls.sh is not given x86_64-pc-windows-gnu/release/kawaro.exe: []
  FAIL: AC8: windows-native exists: no job windows-native (jobs: ['check'])
  FAIL: AC8: windows-native on windows-latest: windows-native runs-on is not windows-latest
  FAIL: AC8: windows-native installs FFmpeg before cargo test: windows-native has no 'choco install ffmpeg -y' step
== AC9: make-iso.sh on a fake build output
  FAIL: AC9: missing or not executable scripts/windows/make-iso.sh
== AC10: run-tests.cmd
  FAIL: AC10: missing scripts/windows/run-tests.cmd
== AC11: README attach command
  FAIL: AC11: English section has the attach command
  FAIL: AC11: Arabic section has the attach command
== AC12: check-dlls.sh input errors
  FAIL: AC12: --text on a missing file: exit code: got [1] want [2]
  FAIL: AC12: --text on an empty file: exit code: got [0] want [2]
  FAIL: AC12: empty file prints no ok line ([dll check: ok empty.txt 0 imports])
  FAIL: AC12: --text on objdump output with no DLL Name line: exit code: got [0] want [2]
  FAIL: AC12: no DLL Name line prints no ok line ([dll check: ok no-dll-name.txt 0 imports])
  FAIL: AC12: objdump output with no DLL Name line: exit code: got [0] want [2]
  FAIL: AC12: objdump with no DLL Name prints no ok line ([dll check: ok kawaro.exe 0 imports])
== AC12: windows-cross CARGO_TARGET_DIR
  FAIL: AC12: windows-cross sets CARGO_TARGET_DIR inside the workspace: windows-cross sets no CARGO_TARGET_DIR (workflow, job or the build and check-dlls steps)
== AC12: engines.md
  FAIL: AC12: engines.md has a mingw64-binutils row
  FAIL: AC12: mingw64-binutils row has its version (2.46.0)
  FAIL: AC12: mingw64-binutils row has its licence
  FAIL: AC12: mingw64-binutils row names x86_64-w64-mingw32-objdump
  FAIL: AC12: the mingw64-gcc row no longer claims objdump
  FAIL: AC12: engines.md records Ubuntu's mingw-w64 package
== lint: bash -n, shellcheck, no long dash
  FAIL: lint: missing scripts/windows/make-iso.sh
FAIL S-044b windows CI, ISO and runner (light) (29 of 41 checks failed)
```
  - Heavy failing output (here, before implementation, `pgrep` empty; T was fresh and passed its 3 setup checks in about 1 s):
```
$ bash tests/windows/test_heavy_iso.sh
== AC9: make-iso.sh on the real build output
  FAIL: AC9: missing or not executable scripts/windows/make-iso.sh
FAIL S-044b windows ISO (heavy) (1 of 4 checks failed)
```
- 2026-10-22, implementer (S-044b):
  - Files: `.github/workflows/ci.yml` (two jobs appended, `check` untouched: `git diff -U0 main -- .github/workflows/ci.yml` has no `-` line), `scripts/windows/make-iso.sh` (new), `scripts/windows/run-tests.cmd` (new, CRLF), `scripts/windows/README.md` (make-iso and run-tests rows, the T, make-iso and `virsh ... attach-disk win11 <iso> sdb --type cdrom --mode readonly` commands in both languages; the Arabic "full check" line now names the three scripts, since the table has five rows), `scripts/windows/check-dlls.sh` (exit 2 on a missing or unreadable `--text` file and on input with no `DLL Name` line), `docs/engines.md` (`mingw64-gcc` row no longer claims objdump; new `mingw64-binutils` 2.46.0 and Ubuntu `mingw-w64` rows; heading `S-044`), `CHANGELOG.md` (the S-044 line extended).
  - `windows-cross`: job env `CI: true` and `CARGO_TARGET_DIR: ${{ github.workspace }}/target-windows`; apt `mingw-w64`; `dtolnay/rust-toolchain@stable` with `targets: x86_64-pc-windows-gnu` and clippy; rust-cache pointed at `target-windows`; then `check.sh`, `build.sh`, `check-dlls.sh "$CARGO_TARGET_DIR/x86_64-pc-windows-gnu/release/kawaro.exe"`. `windows-native`: toolchain, cache, `choco install ffmpeg -y`, `cargo test -p kawaro-core -p kawaro-engines -p kawaro-cli`. First green run stays the S-052 live check.
  - `make-iso.sh`: no cargo call; follows build.sh's `CARGO_TARGET_DIR`; exit 2 without the release exe or with no test program; grafts the inputs straight into `genisoimage -quiet -J -R -graft-points` (no staging copy of about 3 GB), writes `.iso.tmp` then renames; SHA256SUMS has one line per `.exe` (`bin/...`, `tests/...`).
  - Deviation, test selection: the manager's note suggested reading cargo's JSON messages, but AC9 (and the story's check commands) say `make-iso.sh` builds nothing, and the light suite puts a logging fake `cargo` first on `PATH` and fails on any call, so `make-iso.sh` cannot run cargo for the list. It selects from the build tree instead: every `debug/deps/*.exe` except one that is the same file (`-ef`, hardlink) as an uplifted `debug/*.exe`; that drops the CLI binary cargo builds for the integration tests (`kawaro-b229e0b4bca3b0e5.exe`, two links with `debug/kawaro.exe`) and keeps the 35 test programs T lists. A dedupe by name stem was not added: `kawaro` appears twice by design (the bin's unit test program and the bin), so it would drop a real test. Known limit: stale hashes from an older build in `deps/` are not told apart (none today); the script comment says to clean `deps/` then.
  - `run-tests.cmd`: `cd /d` into `%USERPROFILE%\kawaro-tests` first, so nothing a test writes to its working directory lands on the disc; prefix redirects (`>>"%RES%" echo ...`) so a trailing digit is never read as a stream number. Not executed here (needs wine or the VM): S-052.
  - Results: `bash tests/windows/test_ci_iso.sh` PASS 78 checks; `bash tests/windows/test_scripts.sh` PASS 63; `bash tests/windows/test_heavy.sh` PASS 11 (incremental, test targets 35, listed .exe 35); `bash tests/windows/test_heavy_iso.sh` PASS 9 (T 35, `/tests/*.exe` on the ISO 35; ISO 3257526272 bytes). `shellcheck scripts/windows/*.sh` clean; `scripts/design-lint.sh` clean; no em or en dash in the changed files. `C`: `cargo fmt --check` clean, `cargo clippy -j 2 --workspace --all-targets -- -D warnings` clean. `L`: 45 `test result: ok`, 697 passed, 0 failed, 1 ignored. No Rust changed, so no mutants; `git diff --stat main...HEAD -- crates/ui` empty.
  - Risk for S-052 (no change here): many test programs bake build host paths at compile time (`env!("CARGO_BIN_EXE_kawaro")` in the CLI tests, `CARGO_MANIFEST_DIR` fixture and actions paths in core, engines, daemon and CLI tests). On the VM those Linux paths do not exist and the CLI binary is not on the ISO, so such tests will fail there regardless of the ISO; S-052 should read their failures with this in mind or carry the fixtures and set the paths.
- 2026-10-22, reviewer (S-044b): **PASS**.
  - Light suites: `tests/windows/test_ci_iso.sh` PASS (78 checks), `tests/windows/test_scripts.sh` PASS (63). Heavy suites from a plain shell (memory.max `max`, `pgrep -x cargo` empty, 59 GB free): `test_heavy.sh` PASS (11, incremental, 35 test targets, 35 listed .exe), then `test_heavy_iso.sh` PASS (9; T 35, `/tests/*.exe` on the ISO 35; ISO 3257526272 bytes).
  - Real ISO by hand: `isoinfo -d` gives Joliet UCS level 3 and Rock Ridge version 1; `isoinfo -J -f` lists 36 `.exe` plus `/SHA256SUMS`, `/bin`, `/run-tests.cmd`, `/tests`; SHA256SUMS has 36 lines and every hash matches the bytes extracted with `isoinfo -J -x` (0 mismatches); extracted `/run-tests.cmd` hashes as the source. `debug/deps` has 36 `.exe`; the one left out is the CLI binary hardlinked to `debug/kawaro.exe`.
  - `cargo fmt --check` and `cargo clippy -j 2 --workspace --all-targets -- -D warnings`: clean. `cargo test -j 2 --all`: 45 suites, 697 passed, 0 failed, 1 ignored. `shellcheck scripts/windows/*.sh` and `scripts/design-lint.sh`: clean. No em or en dash in `main...HEAD`. `crates/ui` diff against main empty; no file under `crates/` changed in S-044b, so 0 mutants. `tests/windows/*` unchanged since a56e6b1. `git diff $(git merge-base main HEAD) -- .github/workflows/ci.yml`: only `+` lines (0 `-`).
  - AC12 by hand: `check-dlls.sh --text` on a missing file 2, on `/dev/null` 2, on `objdump-ok.txt` without `DLL Name` lines 2, on a missing exe 2, no argument or `--text` alone 2; `objdump-ok.txt` 0, `objdump-mingw.txt` 1. engines.md: `mingw64-binutils` 2.46.0 matches `rpm -q`, Ubuntu `mingw-w64` row present. AC10: `file` says `with CRLF line terminators`. AC11: the attach command is in both README sections; virsh not run. CHANGELOG: the S-044 line extended, not duplicated.
  - ci.yml read: action tags `actions/checkout@v4`, `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2`, the same as `check`; no secrets; no `permissions` key, the same as `check` (default token scope); `CARGO_TARGET_DIR` is `${{ github.workspace }}/target-windows` and the cache follows it; `check-dlls.sh` reads the built exe; `choco install ffmpeg -y` before `cargo test`. run-tests.cmd read: `%~dp0` captured before `cd /d` into `%USERPROFILE%\kawaro-tests`; every redirect goes to `%RES%` or `%LOG%` there; `!errorlevel!` read right after each program under delayed expansion.
  - Deviations accepted: selection from the build tree instead of cargo JSON (AC9 and the light suite forbid any cargo call in make-iso.sh); stale hashes in `deps/` not told apart (documented in the script header); host paths baked into test programs, a recorded S-052 risk that does not affect this story's ACs.
  - Findings, none blocking:
    - Low, `scripts/windows/make-iso.sh:25-31`: the CLI binary is excluded only through the hardlink; where cargo falls back to a copy (a filesystem without hard links) or stale hashes remain, extra `.exe` land in `/tests/` and run-tests.cmd counts them as failures. A count check against T's list would catch it; S-052 should compare the ISO count with T.
    - Low, `.github/workflows/ci.yml:1-8`: no workflow `permissions: contents: read`; consistent with the existing job, but a one-line hardening for a later CI story.
    - Info, `scripts/windows/check-dlls.sh:12`: a non-regular `--text` path (such as `/dev/null`) is reported as "no such file"; the exit code 2 is right.
    - Info, `scripts/windows/run-tests.cmd:7`: delayed expansion would mangle a path holding `!`; the disc and `%USERPROFILE%` paths here have none.
    - Info, `.github/workflows/ci.yml:28-61`: the first green run of both jobs stays the S-052 live check.
- 2026-10-23, tester (S-044b, AC8 base fix):
  - Cause: `tests/windows/test_ci_iso.sh` took `BASE="$(git merge-base main HEAD)"`. Since windows/0.6.0 was merged into main (c303826), the merge base is HEAD itself, so the AC8 check "ci.yml has added lines against the merge base" saw 0 added lines and failed on a clean main: `FAIL: AC8: ci.yml has added lines against the merge base (got 0)`, `FAIL S-044b windows CI, ISO and runner (light) (1 of 78 checks failed)`.
  - Change: `BASE` is now the fixed full hash `aa84616a634397031c97c058c2fceba16cf026f7`, the commit windows/0.6.0 was cut from, with a two-line comment saying why. `git log aa84616..main -- .github/workflows/ci.yml` lists only the Windows commits adc84ad and f2ff8a7. Every assertion is unchanged: only `+` lines, `check` byte-identical to the base, the two jobs and their steps. No other `merge-base` use in `tests/windows/*.sh`.
  - Results: `bash tests/windows/test_ci_iso.sh` PASS (78 checks); `bash tests/windows/test_scripts.sh` PASS (63). `shellcheck -x tests/windows/test_ci_iso.sh` clean. No em or en dash in the changed files.
- 2026-10-23, reviewer (S-044b, AC8 base fix, commits 2a930b3 6b12845 7854af6): PASS
  - Scope: `git diff a9575df..fix/s044b-ac8` touches only `tests/windows/test_ci_iso.sh` and this log. Every assertion is the same command, input and expected value as before; only `BASE` and three labels changed.
  - Base: aa84616 (S-021a review PASS) is an ancestor of main and predates the Windows work; `git log aa84616..main -- .github/workflows/ci.yml` lists only adc84ad and f2ff8a7 (Windows jobs and comment renumbering inside them), so "only + lines" and "check job unchanged" keep their meaning.
  - Mutation demo in a throwaway clone (removed afterwards): deleting `- run: scripts/design-lint.sh crates/ui` from job `check`, or editing `cargo test --workspace`, fails 2 of 78 (no deleted line; job check differs).
  - Results: `tests/windows/test_ci_iso.sh` PASS (78 checks); `tests/windows/test_scripts.sh` PASS (63); `shellcheck -x` clean; no em or en dash in the diff; `scripts/design-lint.sh` clean.
  - Info: `check_unchanged` compares the parsed YAML of job `check`, not bytes as the tester entry says; the textual "no deleted line" check covers byte edits. A shallow clone without aa84616 would break AC8, as the old merge base also needed history; the script is not run in CI.
