# S-041 Settings, actions and log paths on Windows, and naming the output without replacing
Status: done (PASS from the reviewer 2026-10-22), on the windows/0.6.0 branch at the owner's request; ADR 0005 is still proposed; merged into main locally on 2026-10-23 (not pushed)
Renumbered on 2026-10-23: this story was S-023 on windows/0.6.0 before the merge into main (main had already used S-022 and S-027). Commit messages and review-log hashes keep the old number.

Builds on ADR 0005 section 4, and on S-040b (the windows-sys dependency in core). For release 0.6.0. Does not touch `crates/ui`. One working day or less, so no split.

## Story
As a user on Windows, I want my settings and actions saved in `%APPDATA%\SbarHive` and the daemon log in `%LOCALAPPDATA%\SbarHive\logs`, and I want the output never to replace an existing file, even on a USB drive, so that I find my files where Windows expects them and never lose a file.

## Files
`crates/core/src/{lib.rs,paths.rs,paths/windows.rs,config.rs,output_impl.rs,output_impl/windows.rs}`, `crates/core/Cargo.toml` (windows-sys features only), `crates/daemon/src/state.rs`, `crates/daemon/src/watch.rs`, `crates/cli/src/main.rs`, `.cargo/mutants.toml`, `docs/architecture.md`, `README.md`, `CHANGELOG.md`.

## Acceptance criteria
- AC1: a pure function `paths::config_dir_with(os, env, known)` (the name is a proposal): `os` from `Os::{Linux, Windows}`, `env` a function that reads a variable, and `known` a function that gives the known folder. On Windows with `APPDATA=A` it returns `Path::new(A).join("SbarHive")`, and does not call `known` (counter 0). Without `APPDATA`, or with an empty value, it returns `known()` with `SbarHive` appended, and `None` if `known` returned `None`. On Linux it returns what it returns today: `XDG_CONFIG_HOME/sbarhive`, then `HOME/.config/sbarhive`, then `None`.
- AC2: `paths::log_dir_with(os, env, known)`: on Windows `Path::new(LOCALAPPDATA).join("SbarHive").join("logs")`, with the `known` fallback as in AC1. On Linux `None` (the log stays on stderr).
- AC3: `Config::path()` equals `config_dir().join("config.toml")`, and the new `Config::actions_dir()` equals `config_dir().join("actions")`. The signatures of `Config::path`, `Config::load` and `Config::load_from` stay as they are, because `crates/ui` calls them. The existing test `path_and_load_follow_xdg_then_home` passes without modification.
- AC4: `user_actions_dir` is removed from `crates/daemon/src/state.rs`, and `load_registry` calls `Config::actions_dir()`. `grep -rn 'fn user_actions_dir' crates/` returns 0 lines, and a TOML action in `XDG_CONFIG_HOME/sbarhive/actions/` appears in `sbarhive actions` as today.
- AC5: `paths/windows.rs` reads the two folders with `SHGetKnownFolderPath` (`FOLDERID_RoamingAppData` and `FOLDERID_LocalAppData`) and frees the memory with `CoTaskMemFree`. The file is in `exclude_globs`. `cargo clippy -p sbarhive-core --lib` for the Windows target has no warning.
- AC6: `finalize_with` takes an injected place-without-replacing function instead of the check then `rename` written inside it. With a fake function that returns `AlreadyExists` for `a.webp` and `a (1).webp`: the output is `a (2).webp`, the content of the two existing files is unchanged, and the temporary file is removed. With a function that returns another error (`PermissionDenied`): the same error is returned, and the temporary file remains.
- AC7: on Linux the place function is today's behaviour (the check then `rename`), so the four `ac9_*` tests in `output_impl.rs` and `crates/core/tests/output.rs` pass without modifying their assertions. On Windows it is `MoveFileExW` without `MOVEFILE_REPLACE_EXISTING` in `output_impl/windows.rs`, and codes 80 and 183 become `AlreadyExists`. The code mapping function is pure, in `output_impl.rs`: 80 and 183 give `AlreadyExists`, and 5 gives `PermissionDenied`. `output_impl/windows.rs` is in `exclude_globs`.
- AC8: no new crate: the count of `[[package]]` in `Cargo.lock` stays 671, and `X` lists nothing beyond what it listed after S-040b.
- AC9: `README.md` gives the location of `config.toml`, the actions and the log on Windows in Arabic and English, and `docs/architecture.md` describes the place-without-replacing function. One line in `CHANGELOG.md` under `[Unreleased]` in English with Arabic text in «». `git diff --stat main...HEAD -- crates/ui` is empty. `C` with no warning on Linux: the code function and the path functions called only by a Windows file are `pub` or carry `#[cfg_attr(not(windows), allow(dead_code))]`. `M` with no surviving mutant.

## Check commands
Read-only tools alone (`grep`) run outside the scope.
- `C`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% sh -c 'cargo fmt --check && cargo clippy -j 2 --workspace --all-targets -- -D warnings'`
- `L`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% cargo test -j 2 -p sbarhive-core -p sbarhive-daemon -p sbarhive-cli`
- `W`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_TARGET_DIR=/home/sbarah/.cache/sbarhive-win-target cargo clippy -j 2 --target x86_64-pc-windows-gnu -p sbarhive-core -p sbarhive-engines --lib -- -D warnings`
- `M`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_BUILD_JOBS=2 cargo mutants --jobs 1 --in-diff <diff>` (the changed files only, including `state.rs`; Windows files are excluded)
- `X`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_TARGET_DIR=/home/sbarah/.cache/sbarhive-win-target cargo tree --target x86_64-pc-windows-gnu -p sbarhive-core -e normal --depth 1`

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1, AC2 | Unit tests in `paths.rs` with a fake environment for each branch: `APPDATA` present, empty, and missing with `known` returning a folder, then `None`, and the three Linux branches. Expected values are built with `Path::join` itself, not with a string containing `\`, because Linux joins with `/`; under `M` | unit |
| AC3 | The existing test in `config.rs`, and a new test for `actions_dir` | unit |
| AC4 | The `grep` command; and `crates/cli/tests/cli.rs` with an action planted in the temporary directory | review, integration |
| AC5 | `W` with no warning, and the reviewer reads `paths/windows.rs` | build, review |
| AC6, AC7 | Unit tests in `output_impl.rs` with two fake place functions, and the code function; and the existing `ac9_*` tests; under `M` | unit |
| AC8 | `grep -c '^\[\[package\]\]' Cargo.lock` gives 671, and `X` | review |
| AC9 | The reviewer reads README, architecture.md and CHANGELOG.md | review |

Stay green: `cargo test --workspace`, and `tests/e2e/run.sh`.

## Live verification deferred to S-052
- `config.toml` is written to `%APPDATA%\SbarHive` from the settings window, and the atomic save with `std::fs::rename` replaces the old file.
- An account without `APPDATA` in the environment finds the folder with `SHGetKnownFolderPath`.
- A conversion on an exFAT drive does not replace an existing file with the same name.

## Out of scope
- Writing the log to the file, and `windows_subsystem`: S-045. Here only the function that gives the folder.
- Temporary files: `std::env::temp_dir()` gives `%TEMP%`, and the temporary file stays next to the output; no change.
- UNC paths (`\\server\share`) for settings.

## Review log
- 2026-10-22, planner: the choice takes `os` as a parameter so that the Windows branch is tested on Linux, and the real call passes `Os` from `cfg!(windows)`.
- 2026-10-22, tester: the contract the tester fixed at the top of `crates/core/tests/paths.rs` and in the comment of the test module in `output_impl.rs` (not to be modified):
  - `sbarhive_core::paths` is a `pub` module on all platforms, with `enum Os { Linux, Windows }`, `config_dir_with(os, env, known)`, `log_dir_with(os, env, known)` and `config_dir()`, all `pub`. `env` has the shape `Fn(&str) -> Option<OsString>` (the shape of `var_os`), and `known` has the shape `Fn() -> Option<PathBuf>`; the tests pass them by reference, so `&dyn Fn` and generic parameters both work. `Config::actions_dir() -> Option<PathBuf>`.
  - `finalize_with(temp, final_path, link: &LinkFn, place: &PlaceFn)` where `PlaceFn` is `dyn Fn(&Path, &Path) -> std::io::Result<()>`: it moves, and fails with `AlreadyExists` if the name is taken. `finalize_with` does not check existence and does not call `rename` itself when linking fails, and it still returns the link error if the temporary file is missing (`ac9_fallback_with_missing_temp_reports_the_link_error`). The fake functions capture nothing, so a `'static` alias like `LinkFn` works.
  - `place_no_replace(from, to)` visible in `output_impl.rs`: on Linux the check then `rename`, and on Windows from `output_impl/windows.rs`. And `move_error_kind(code: u32) -> ErrorKind` is pure: 80 and 183 are `AlreadyExists`, 5 is `PermissionDenied`, and others are neither.
  - Files and tests:
    - `crates/core/tests/paths.rs` (new): `ac1_windows_appdata_wins_and_known_is_not_called`, `ac1_windows_empty_appdata_falls_back_to_the_known_folder`, `ac1_windows_missing_appdata_falls_back_to_the_known_folder`, `ac1_windows_ignores_xdg_and_home`, `ac1_windows_without_appdata_and_without_known_folder_is_none`, `ac1_windows_appdata_is_not_read_as_localappdata`, `ac1_linux_xdg_config_home_first`, `ac1_linux_home_dot_config_second`, `ac1_linux_without_xdg_and_home_is_none_even_with_appdata` (the three Linux branches check that `known` is not called), `ac2_windows_log_dir_is_localappdata_sbarhive_logs_without_known`, `ac2_windows_empty_or_missing_localappdata_falls_back_to_the_known_folder` (and `APPDATA` alone does not stand in), `ac2_windows_without_localappdata_and_without_known_folder_is_none`, `ac2_linux_has_no_log_dir`, `ac3_config_dir_follows_the_process_environment_on_linux`, `ac3_config_path_is_config_dir_joined_with_config_toml`, `ac3_actions_dir_is_config_dir_joined_with_actions`.
    - `crates/core/src/output_impl.rs` (the test module): `ac6_place_already_exists_moves_on_to_the_next_number` (the two files really exist, and the order of names asked for is `a.webp`, then `a (1).webp`, then `a (2).webp`), `ac6_finalize_trusts_the_place_function_not_its_own_check` (no file on disk, so a check left inside `finalize_with` would give `a.webp`), `ac6_place_permission_denied_is_returned_and_keeps_the_temp` (one call, with no retry on another name), `ac6_a_successful_link_does_not_call_place`, `ac7_linux_place_refuses_a_taken_name_and_keeps_both_files`, `ac7_linux_place_refuses_a_dangling_link_name`, `ac7_linux_place_moves_to_a_free_name`, `ac7_move_error_codes_80_and_183_are_already_exists`, `ac7_move_error_code_5_is_permission_denied`, `ac7_move_error_other_codes_are_neither`. The `finalize_with` calls in the three `ac9_*` tests that call it were edited to pass `&place_no_replace` as a fourth argument, and their assertions are literally unchanged (AC7).
    - `crates/core/tests/paths_windows.rs` (new, a text check on the Windows files in the manner of `transport_seed.rs`): `ac5_paths_windows_reads_both_known_folders_and_frees_them`, `ac5_ac7_windows_files_are_excluded_from_mutants`, `ac5_ac7_paths_rs_and_output_impl_rs_stay_mutated`, `ac7_windows_places_with_move_file_ex_without_replacing` (bans the name `MOVEFILE_REPLACE_EXISTING` in the whole file, comments included, and requires `MoveFileExW` and `move_error_kind(`).
    - `crates/daemon/tests/actions_dir.rs` (new): `ac4_no_crate_defines_user_actions_dir` (walks all of `crates/`, and the searched text is built from two pieces so the reviewer's `grep` does not match it), `ac4_load_registry_uses_config_actions_dir`.
  - Passes before implementation on purpose: `actions_include_user_actions_from_the_config_dir` in `crates/cli/tests/cli.rs` (a regression guard for the AC4 "as today" behaviour), `ac13_cargo_lock_keeps_671_packages` from S-040b (a guard for AC8), and `path_and_load_follow_xdg_then_home` and `ac9_*` in `crates/core/tests/output.rs`.
  - Ambiguities and what the tester chose: (1) the table asks for unit tests inside `paths.rs`, and that file belongs to the implementer, so the AC1, AC2 and AC3 tests are in `crates/core/tests/paths.rs` through the public interface; and the `actions_dir` test is not in `config.rs` because the existing test there changes the environment and so races with it, while a separate binary that does not change the environment reads it as it is. (2) **AC4 cannot pass without touching two files outside the file list:** `crates/cli/src/main.rs:235` defines its own `user_actions_dir`, and `crates/daemon/src/watch.rs` imports it from `state.rs`; the criterion's `grep` over all of `crates/` counts the first, and the second does not build after the removal. The tester did not narrow the test, so the implementer must change both to `Config::actions_dir()`. (3) An empty `XDG_CONFIG_HOME` on Linux is not pinned: "as today" means it is used as it is, and no test was written for it. (4) Windows codes other than 80, 183 and 5 were not specified by AC7; the tester pinned that they are neither `AlreadyExists` nor `PermissionDenied` (0, 1, 2, 3, 79, 81, 87, 182 and 184) so that an "every code is taken" mutant does not survive, and left their kind to the implementer. (5) The names `place_no_replace`, `move_error_kind` and `config_dir` were chosen by the tester because the criterion did not name them.
  - For the reviewer, not for tests: `W` and `X`, reading `paths/windows.rs` (freeing memory on every path), the `windows-sys` features in `crates/core/Cargo.toml`, README, architecture.md and CHANGELOG.md, `git diff --stat main...HEAD -- crates/ui`, and `M`.
  - For the reviewer, the inline tests are not seen by the `crates/*/tests/*` check: the test module in `output_impl.rs` and the existing test in `config.rs` must not change after this commit. The two commands (an empty diff is required):
    `diff <(git show f1e430e:crates/core/src/output_impl.rs | sed -n '/^#\[cfg(test)\]/,$p') <(sed -n '/^#\[cfg(test)\]/,$p' crates/core/src/output_impl.rs)`
    `diff <(git show f1e430e:crates/core/src/config.rs | sed -n '/fn path_and_load_follow_xdg_then_home/,/^    }$/p') <(sed -n '/fn path_and_load_follow_xdg_then_home/,/^    }$/p' crates/core/src/config.rs)`
  - Warning: the `--lib` test binary in core does not build until the implementation lands, so all existing unit tests in core (including `transport.rs` from S-040a and `config.rs`) show red until then; this is unavoidable because the tested functions are private.
  - Check of the tests' logic: a temporary copy of the repository in the session directory (deleted and not merged) with a short implementation of the contract passed 28 of the core `--lib` tests, the sixteen `paths` tests, the four `paths_windows` tests, the fourteen `output` tests and the two `actions_dir` tests; and `cargo clippy -p sbarhive-core --tests -D warnings` is clean on it. `cargo fmt --check` is clean here.
  - Failure output (each command inside `systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200%`):
```
$ cargo test -j 2 -p sbarhive-core --lib
error[E0061]: this function takes 3 arguments but 4 arguments were supplied
error[E0425]: cannot find function `move_error_kind` in this scope
error[E0425]: cannot find function `place_no_replace` in this scope
error[E0425]: cannot find value `place_no_replace` in this scope
error: could not compile `sbarhive-core` (lib test) due to 17 previous errors

$ cargo test -j 2 -p sbarhive-core --test paths
error[E0432]: unresolved import `sbarhive_core::paths`
error[E0599]: no associated function or constant named `actions_dir` found for struct `Config` in the current scope
error: could not compile `sbarhive-core` (test "paths") due to 2 previous errors

$ cargo test -j 2 -p sbarhive-core --test paths_windows
test ac5_ac7_paths_rs_and_output_impl_rs_stay_mutated ... FAILED
test ac5_ac7_windows_files_are_excluded_from_mutants ... FAILED
test ac5_paths_windows_reads_both_known_folders_and_frees_them ... FAILED
test ac7_windows_places_with_move_file_ex_without_replacing ... FAILED
crates/core/src/paths.rs must exist
"crates/core/src/paths/windows.rs" must be in exclude_globs (AC5, AC7)
.../crates/core/src/output_impl/windows.rs must exist: No such file or directory (os error 2)
.../crates/core/src/paths/windows.rs must exist: No such file or directory (os error 2)
test result: FAILED. 0 passed; 4 failed

$ cargo test -j 2 -p sbarhive-daemon --test actions_dir
test ac4_load_registry_uses_config_actions_dir ... FAILED
test ac4_no_crate_defines_user_actions_dir ... FAILED
load_registry must call Config::actions_dir() (AC4):
these files still define the old helper: [".../crates/cli/src/main.rs", ".../crates/daemon/src/state.rs"] (AC4)
test result: FAILED. 0 passed; 2 failed

$ cargo test -j 2 -p sbarhive-cli --test cli actions_include_user_actions
test actions_include_user_actions_from_the_config_dir ... ok
test result: ok. 1 passed; 0 failed
```
- 2026-10-22, manager: decision on the tester's blocker (2): `crates/cli/src/main.rs` (its own folder function is removed and `Config::actions_dir()` is used) and `crates/daemon/src/watch.rs` (imports `Config::actions_dir()` instead of the removed function) are added to the file list. Both are outside `crates/ui`.
- 2026-10-22, implementer:
  - `crates/core/src/paths.rs` (new): `Os`, `CURRENT`, `config_dir_with`, `log_dir_with` and `config_dir`, with the two parameters as `&dyn Fn`. An empty variable on Windows falls back to the known folder, and an empty `XDG_CONFIG_HOME` on Linux is used as it is (as today). No `log_dir()` function on Linux, because it always returns `None` and nothing tests it; it is in `paths/windows.rs` alone and is re-exported on Windows for S-045.
  - `crates/core/src/paths/windows.rs` (new): `SHGetKnownFolderPath` with `FOLDERID_RoamingAppData` and `FOLDERID_LocalAppData`, and `CoTaskMemFree` on every path, success and failure.
  - `crates/core/src/output_impl.rs`: `PlaceFn` and `finalize_with` with four parameters and no check or `rename` inside it, `place_no_replace` (Linux: the check then `rename`), and `move_error_kind` (80 and 183 `AlreadyExists`, 5 `PermissionDenied`, others `Other`; no `from_raw_os_error` because 1 on Linux is EPERM). The test module did not change (the two diffs from the tester's log are empty).
  - `crates/core/src/output_impl/windows.rs` (new): `MoveFileExW` with flags 0, and it keeps the system code inside the error.
  - `config.rs`: `path()` and `actions_dir()` from `paths::config_dir()`, with the signatures as they are. `Cargo.toml`: the two features `Win32_UI_Shell` and `Win32_System_Com`. `.cargo/mutants.toml`: the two files in `exclude_globs`.
  - Deviation: the `replace run with ()` mutant in `watch.rs` survives because `run` is an endless polling loop that no unit test runs, and S-041 changed only the import line and the actions folder call inside `run`. I added the pattern `replace run with \\(\\)$` to `exclude_re` with a comment, on the `serve` precedent; it does not match `run` in `cli/src/main.rs` because that one returns `Res<()>` (I checked with `--list`). `tests/e2e/run.sh` covers reloading.
  - Gates (all inside `systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200%`):
    - `C`: `cargo fmt --check` and `cargo clippy -j 2 --workspace --all-targets -- -D warnings` clean.
    - `L` and `cargo test -j 2 --all`: 642 passed, 0 failed, 1 ignored, in 43 binaries.
    - `W`: clippy and `cargo check` for core and engines `--lib` for the `x86_64-pc-windows-gnu` target with no warning. daemon and cli have no `lib` target.
    - `M` on the diff of the seven changed files from 8a27c8e: 25 mutants, 23 caught, 1 unviable (`move_error_kind` with `Default::default()`, because `ErrorKind` has no `Default`), 1 survivor (`watch.rs` above, now excluded, so the list is 24).
    - `X`: serde, serde_json, thiserror, toml and windows-sys as after S-040b. `Cargo.lock` unchanged, and 671 `[[package]]`.
    - `scripts/design-lint.sh` clean. `tests/e2e/run.sh`: all passed. `git diff --stat main...HEAD -- crates/ui` is empty.
    - Note: `--jobs 1` is rejected together with `--in-place`, so I ran `--in-place`, which is sequential by nature. And two runs (the e2e build and the last round) overlapped with a build from another session despite the `pgrep` check, and nothing failed.
- 2026-10-22, reviewer: **PASS**. All gates were rerun inside the scope (`MemoryMax=6G`, `-j 2`) from the `windows/0.6.0` tree at `6e010dd`:
  - `cargo fmt --check` clean. `cargo clippy -j 2 --workspace --all-targets -- -D warnings` with no warning. `cargo test -j 2 --all`: 642 passed, 0 failed, 1 ignored, in 43 binaries; the story's thirty-four tests pass by name, along with the four `ac9_*` tests in `output_impl.rs`, the five in `tests/output.rs` and `ac13_cargo_lock_keeps_671_packages`.
  - `W` (after `touch` to force a recheck, with the separate `CARGO_TARGET_DIR`): `cargo clippy` and `cargo check` for the `x86_64-pc-windows-gnu` target for core and engines `--lib` with no warning.
  - `X`: serde, serde_json, thiserror, toml and `windows-sys v0.61.2` as after S-040b. `Cargo.lock` unchanged, and 671 `[[package]]`.
  - `M`: `cargo mutants --jobs 1 --in-diff` on `git diff 8a27c8e..HEAD -- crates` with `CARGO_BUILD_JOBS=2`, in a copy, not `--in-place`: 24 mutants, 23 caught, 1 unviable (`move_error_kind` with `Default::default()`), 0 timeouts, **0 survivors**. `load_registry` at `state.rs:42` is caught.
  - `scripts/design-lint.sh` clean. No long dash in `git diff 15d7523..HEAD` (the positive control gives 1). `git diff --stat main...HEAD -- crates/ui` is empty. `git diff 8a27c8e..HEAD -- 'crates/*/tests/*'` is empty (the control on `15d7523..8a27c8e` lists the three files), and the two inline test diffs in `output_impl.rs` and `config.rs` from the tester's log are empty (the second really matches 43 lines). `grep -rn 'fn user_actions_dir' crates/` gives 0. `tests/e2e/run.sh`: 37 PASS, 0 FAIL, including "edited settings apply without reload".
  - The criteria against the code: AC1 and AC2 in `paths.rs` are pure, with `&dyn Fn` parameters, and an empty variable on Windows falls back to the known folder as AC1 says literally (not a deviation). AC3: the signatures are as they were. AC4: `state.rs`, `cli/src/main.rs` and `watch.rs` use `Config::actions_dir()` by the manager's decision. AC5: `paths/windows.rs` frees with `CoTaskMemFree` after every call, on success, failure and a null pointer (allowed), reads the string before freeing, and checks `hr >= 0` and `!raw.is_null()`. AC6: `finalize_with` neither checks nor calls `rename`; `AlreadyExists` moves to the next number, anything else is returned with the temporary file kept, and success moves the temporary file so it does not remain; no path touches anything but the temporary file and the candidate names, so the original is never touched. AC7: `place_no_replace` on Linux is literally the previous check then `rename`, so Linux behaviour did not change, and the existing race is documented; and on Windows `MoveFileExW` with flags 0, so it never replaces. AC9: README in Arabic and English, architecture.md, and one CHANGELOG line from the implementer, so no new line from the reviewer (one line per story).
  - Accepted deviations: the new `exclude_re` `replace run with \(\)$` matches across the whole tree (`--list --no-config`) one line, `watch.rs:41`, and `run` is an endless loop covered by e2e through reloading `config.toml` without `reload` (which would fail if `run` became empty), on the `serve` precedent. `log_dir()` on Windows alone: AC2 asks only for `log_dir_with`, which is `pub`, so there is no warning on Linux.
  - Non-blocking notes, for S-052:
    - Low, `crates/core/src/output_impl/windows.rs:19-21`: `wide()` passes the path raw to `MoveFileExW`, while `std::fs` on Windows adds the `\\?\` prefix for long paths (from knowledge of the std implementation; rust-src is not installed locally, so not verified). Question: a path over 260 characters on FAT may succeed at linking and fail at placing with `Other`, with the temporary file remaining. No risk to the original.
    - Low, `output_impl/windows.rs:30-35` with `output_impl.rs:136-140`: if the candidate name is a directory, `MoveFileExW` most likely returns 5, so `finalize_with` stops with `PermissionDenied` instead of moving to the next number as on Linux. A rare case, on FAT only.
    - Info, `output_impl/windows.rs:34`: the error wrapped with `io::Error::new(kind, os)` returns `raw_os_error() == None`; the code remains in the text only.
    - Info, `.cargo/mutants.toml`: the new `exclude_re` comment line is longer than its neighbours.
