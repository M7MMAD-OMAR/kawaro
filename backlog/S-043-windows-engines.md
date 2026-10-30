# S-043 External engines on Windows: the process tree, discovery and the file URL
Status: S-043a: done (reviewer PASS, 2026-10-22); S-043b: done (reviewer PASS, 2026-10-22, windows/0.6.0 branch at the owner's request); ADR 0005 is still proposed; merged into main locally on 2026-10-23 (not pushed)
Renumbered on 2026-10-23: this story was S-025 on windows/0.6.0 before the merge into main (main had already used S-022 and S-027). Commit messages and review-log hashes keep the old number.

Builds on ADR 0005 sections 3 and 7. For release 0.6.0. Does not touch `crates/ui`. Independent of S-040 to S-042 in the code.

## Story
As a Windows user who installed LibreOffice, Ghostscript or FFmpeg, I want Kawaro to find them in their usual places, and I want cancelling to kill everything it started, so that conversion works with no setup and no `soffice.bin` stays in the background.

## Split (in order, each substory one work day or less)
| Story | Content | Criteria | Files |
| --- | --- | --- | --- |
| S-043a | The job object with suspend, then assign, then resume, and `is_executable` | AC1 to AC6 | `crates/engines/src/office.rs`, `crates/engines/src/office/{tree.rs,non_unix.rs}`, `crates/engines/Cargo.toml`, `Cargo.lock` (the engines array only), `.cargo/mutants.toml`, `CHANGELOG.md` |
| S-043b | Program names, their paths and the registry, `file_url`, and the missing engine message | AC7 to AC12 | `crates/engines/src/{locate.rs,locate/windows.rs,office.rs,pdf.rs,ffmpeg.rs}`, `crates/engines/Cargo.toml` (the registry feature), `.cargo/mutants.toml`, `docs/engines.md`, `README.md`, `CHANGELOG.md` |

## Acceptance criteria
S-043a:
- AC1: `tree::start_in_job(ops)` in a shared file that is mutated, and `ops` is an injected trait. The sequence recorded with successful fake operations: `create_job`, `kill_on_close`, `spawn_suspended`, `assign`, `threads_of(pid)`, `resume(tid)` for each thread. If `assign` fails: `terminate_process`, then the error is returned, with no `resume` (counter 0), and no job handle is left open (close counter 1).
- AC2: `tree::threads_of(pid, entries) -> Vec<u32>` is pure: `entries = [(10,1),(11,2),(10,3)]` gives `[1,3]` for 10 and `[]` for 12. An empty list for a suspended process gives an error and does not leave the process behind (`terminate_process` once).
- AC3: `spawn_suspended` adds `CREATE_SUSPENDED` (0x4) and `CREATE_NO_WINDOW` (0x08000000) to the command flags, so no console window appears for a console program that a daemon with no console starts. The two constants are in `tree.rs`, and `non_unix.rs` checks them at compile time against `windows_sys`.
- AC4: cancel, timeout and normal exit on Windows call `TerminateJobObject` instead of `kill_group`, and closing the handle uses `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. On Linux `process_group(0)` and `killpg` stay as today, and the existing `run_child` tests (cancel kills the group) pass with no change to their assertions.
- AC5: `tree::has_exe_extension(&str)` is pure and case-insensitive: `a.exe`, `A.EXE` and `soffice.Com` are true, and `a.bat`, `a.cmd`, `a`, `a.exe.txt` and `.exe` are false. `is_executable` on Windows equals `is_file() && has_exe_extension`.
- AC6: `cargo clippy -p kawaro-engines --lib` for the Windows target with no warning. The `[[package]]` count in `Cargo.lock` stays 671, and `X` lists `windows-sys v0.61.2` as the only new entry. `non_unix.rs` stays in `exclude_globs`.

S-043b:
- AC7: `locate::search(os, names, path_dirs, env, exe_dir, registry)` is pure, in the ADR's order: `PATH` first, then the known paths, then the registry. On Windows: LibreOffice by the two names `soffice.com` then `soffice.exe` in `%ProgramFiles%\LibreOffice\program`, then the `InstallPath` folder from `HKLM\SOFTWARE\LibreOffice\UNO\InstallPath`; Ghostscript by `gswin64c.exe` then `gswin32c.exe` in the newest `%ProgramFiles%\gs\gs*\bin`, then the registry; and FFmpeg by `ffmpeg.exe` and `ffprobe.exe` in `PATH`, then `<exe_dir>\ffmpeg\bin`. On Linux the result is as today (`soffice` then `libreoffice`, `gs`, and `ffmpeg`).
- AC8: `locate::newest_gs(names)` is pure, by version number, not by text: `["gs9.56.1","gs10.04.0","gs10.10.0","gsx"]` gives `gs10.10.0`, `["gs9.56.1","gs10.04.0"]` gives `gs10.04.0`, and `[]` gives `None`. The same rule picks the version key under `HKLM\SOFTWARE\GPL Ghostscript`.
- AC9: `locate/windows.rs` reads the registry with `RegGetValueW`, and the file is in `exclude_globs`. `cargo clippy -p kawaro-engines --lib` for the target with no warning, and the `[[package]]` count stays 671.
- AC10: `office::file_url_str(path: &str, windows: bool)` is pure, and `file_url` calls it with `cfg!(windows)`. Windows: `C:\a b\x` gives `file:///C:/a%20b/x`, `C:\ملف.docx` gives `file:///C:/%D9%85%D9%84%D9%81.docx`, and `\\?\C:\a` gives `file:///C:/a`. Linux: `/a b/c` gives `file:///a%20b/c` as today, and the existing tests pass.
- AC11: the `EngineError::Unavailable` message when the program is missing names it and gives the link to its site: `https://www.libreoffice.org/download/`, `https://ghostscript.com/releases/` and `https://ffmpeg.org/download.html`. The "no encoder" messages in ffmpeg (`missing`) stay as they are, and the tests that check `libx264` pass.
- AC12: `docs/engines.md` adds a Windows column (names, paths and registry), and `README.md` says in Arabic and English where Kawaro looks for the engines on Windows and that it does not download them.
- AC13 (added by the manager on 2026-10-22 from an S-043a reviewer note): on Windows, ffmpeg and ffprobe (`crates/engines/src/ffmpeg.rs`, every `Command` site) are started with the `CREATE_NO_WINDOW` flag (the value 0x08000000 from `office::tree`), so no console window flashes. No change on Linux. Proof: a text check that every `Command::new` in ffmpeg.rs goes through one function that adds the flag under `cfg(windows)`, and a clean W build for the Windows target.

For each substory:
- One line for the whole story in `CHANGELOG.md` under `[Unreleased]`, in English with Arabic text in «»: S-043a adds it, and S-043b edits the same line.
- `git diff --stat main...HEAD -- crates/ui` is empty.
- `C` with no warning on Linux: `start_in_job`, `threads_of`, `has_exe_extension` and the constants in `tree.rs`, and `newest_gs` in `locate.rs`, are called only by the Windows branch, so they carry `#[cfg_attr(not(windows), allow(dead_code))]`.
- `M` with no surviving mutant.

## Check commands
Read-only tools (`grep`) run outside the scope.
- `C`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% sh -c 'cargo fmt --check && cargo clippy -j 2 --workspace --all-targets -- -D warnings'`
- `L`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% cargo test -j 2 -p kawaro-engines`
- `W`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_TARGET_DIR=$HOME/.cache/kawaro-win-target cargo clippy -j 2 --target x86_64-pc-windows-gnu -p kawaro-engines --lib -- -D warnings`
- `M`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_BUILD_JOBS=2 cargo mutants --jobs 1 --in-diff <diff>` (the changed files only; `non_unix.rs` and `locate/windows.rs` are excluded)
- `X`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_TARGET_DIR=$HOME/.cache/kawaro-win-target cargo tree --target x86_64-pc-windows-gnu -p kawaro-engines -e normal --depth 1`

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1, AC2 | Unit test in `tree.rs` with fake operations that record the sequence and the close and terminate counters; under `M` | unit |
| AC3, AC6, AC9 | `W` with no warning, and the reviewer reads `non_unix.rs` and `locate/windows.rs` | build, review |
| AC4 | The existing `run_child` tests in `office.rs` and `crates/engines/tests/office_engine.rs` on Linux | integration |
| AC5 | Unit test with the eight names | unit |
| AC7, AC8 | Unit test in `locate.rs` with a fake environment and registry, and paths built with `Path::join`; under `M` | unit |
| AC10 | Unit test with the four strings | unit |
| AC11 | Unit test: an engine with no program returns a message with the name and the link, for each of the three engines | unit |
| AC6 | `grep -c '^\[\[package\]\]' Cargo.lock` gives 671, and `X` | review |
| AC12 | The reviewer reads engines.md, README and CHANGELOG.md | review |
| AC13 | Text check of ffmpeg.rs, and W | unit |

These stay green: `cargo test --workspace`, including `office_engine.rs`, `pdf_ghostscript.rs` and `ffmpeg_engine.rs`.

## Live check deferred to S-052
- Cancelling a document conversion kills `soffice.exe` and `soffice.bin` together, and the daemon dying kills them too.
- `soffice.com` waits for the conversion to finish (assumed in the ADR).
- The two registry keys for LibreOffice and Ghostscript exist under the assumed names.
- The `file:` prefix in ffmpeg with a drive letter (`file:C:\...`).
- No console window flashes when ffmpeg or gswin64c starts.

## Out of scope
- Downloading engines on demand: no download in 0.6.0.
- Translating engine messages: they stay in English, like the rest of the engine errors today.
- UNC paths in `file_url`: the engines work in a local temporary folder.

## Review log
- 2026-10-22, planner: `CREATE_NO_WINDOW` is an addition the ADR does not mention; without it, every console program opens a window when a daemon with the `windows` subsystem starts it. The missing engine message (ADR section 7) was not assigned to a story in the ADR table, so it was put in S-043b, because it lives in engines alone.
- 2026-10-22, tester (S-043a):
  - The contract pinned at the top of `crates/engines/src/office/tree.rs` (the file is new and holds only the test module; the implementation is written above it): the two constants `CREATE_SUSPENDED` and `CREATE_NO_WINDOW` of type `u32`; the trait `JobOps` with two associated types, `Job` (an owned handle closed on drop) and `Child`, and its methods `create_job`, `kill_on_close(&Job)`, `spawn_suspended() -> io::Result<(Child, u32)>` (the process and its id), `assign(&Job, &Child)`, `threads_of(pid) -> io::Result<Vec<u32>>`, `resume(tid)` and `terminate_process(&Child)` with no return value; `start_in_job<O: JobOps>(ops: &mut O) -> io::Result<(O::Job, O::Child)>`; `threads_of(pid: u32, entries: &[(u32, u32)]) -> Vec<u32>`, where the pairs are (owning process id, thread id); `has_exe_extension(name: &str) -> bool`. `mod tree;` was added to `office.rs` with no platform condition, and nothing else changed there.
  - The tests in `crates/engines/src/office/tree.rs` (the `kawaro-engines` library): `ac3_flag_constants_have_the_win32_values`; `ac1_success_runs_the_sequence_and_keeps_the_job_open` (the full sequence with `threads_of(10)`, then `resume(1)` and `resume(3)`, no terminate, and the close counter 0 while the returned job is held, then 1 after it is dropped); `ac1_assign_failure_terminates_returns_the_error_and_closes_the_job` (the sequence ends with `assign` then `terminate_process`, the returned error is the `assign` error with its code 5, `resume` zero, terminate 1 and close 1); `ac2_threads_of_keeps_the_threads_of_one_pid_in_order`; `ac2_no_threads_is_an_error_and_terminates_the_suspended_process`; `ac5_has_exe_extension_for_the_nine_names`.
  - Ambiguities and choices: (1) the criterion does not define the shape of the trait, so associated types were chosen so that both a Windows handle and `std::process::Child` fit, and closing is counted by dropping `Job`, not by an explicit `close` method. (2) AC2 does not mention the close counter for the empty list; the test asks for 1 as in AC1, because the error does not return the job, so it is either closed or leaked. (3) AC5 says "the nine names" and the criterion lists eight (`a.exe`, `A.EXE` and `soffice.Com`, and `a.bat`, `a.cmd`, `a`, `a.exe.txt` and `.exe`), so the eight were tested and no ninth was invented. (4) A failure of `create_job`, `kill_on_close`, `spawn_suspended`, `threads_of`, `resume` or `terminate_process` is not mentioned in the criterion, so it has no test. (5) `is_executable` on Windows, the two AC3 constants against `windows_sys` and the Windows side of AC4 (`TerminateJobObject` and `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`) do not build on Linux, so they are for `W` and for the reviewer to read in `non_unix.rs`, with no text check, because the table makes them review.
  - For the reviewer, not tests: `W`, `X` (today it lists no `windows-sys` for engines, so it is red before the implementation), the `[[package]]` count (already 671 today, so a test for it would pass before the implementation), `non_unix.rs` in `exclude_globs`, the CHANGELOG.md line, `git diff --stat main...HEAD -- crates/ui`, `C` and `M`.
  - AC4 on Linux: the existing `run_child` tests in the `office.rs` test module and in `crates/engines/tests/office_engine.rs` stay unchanged. The `crates/*/tests/*` check covers `office_engine.rs`; the two inline modules must not change after this commit (an empty diff is required; `c3619de` is the tester's commit):
    `diff <(git show c3619de:crates/engines/src/office/tree.rs | sed -n '/^#\[cfg(test)\]/,$p') <(sed -n '/^#\[cfg(test)\]/,$p' crates/engines/src/office/tree.rs)`
    `diff <(git show c3619de:crates/engines/src/office.rs | sed -n '/^#\[cfg(test)\]/,$p') <(sed -n '/^#\[cfg(test)\]/,$p' crates/engines/src/office.rs)`
  - Note: the engines library test binary does not build until the implementation lands, so the existing unit tests in it (including `run_child` in `office.rs`) are red until then. The integration tests build: `office_engine` is green (12).
  - Check of the test logic: a temporary copy in the session folder (deleted, not merged) with a short implementation of the contract passed the six tests, and `cargo clippy --tests -D warnings` is clean on it. Here `cargo fmt --check -p kawaro-engines` is clean.
  - Failing output (each command inside `systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200%`):
```
$ cargo test -j 2 -p kawaro-engines --lib
      1 error[E0405]: cannot find trait `JobOps` in this scope
      1 error[E0425]: cannot find function `has_exe_extension` in this scope
      3 error[E0425]: cannot find function `start_in_job` in this scope
      2 error[E0425]: cannot find function `threads_of` in this scope
      1 error[E0425]: cannot find value `CREATE_NO_WINDOW` in this scope
      1 error[E0425]: cannot find value `CREATE_SUSPENDED` in this scope
warning: unused import: `super::*`  --> crates/engines/src/office/tree.rs:31:9
error: could not compile `kawaro-engines` (lib test) due to 9 previous errors; 1 warning emitted

$ cargo test -j 2 -p kawaro-engines --test office_engine      # AC4, baseline
test result: ok. 12 passed; 0 failed

$ env CARGO_TARGET_DIR=$HOME/.cache/kawaro-win-target cargo clippy -j 2 --target x86_64-pc-windows-gnu -p kawaro-engines --lib -- -D warnings   # W, a gate, not a test
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.50s

$ env CARGO_TARGET_DIR=$HOME/.cache/kawaro-win-target cargo tree --target x86_64-pc-windows-gnu -p kawaro-engines -e normal --depth 1   # X
├── image v0.25.10  ├── lopdf v0.45.0  ├── miniz_oxide v0.8.9  ├── pdf-writer v0.15.0  ├── kawaro-core v0.3.0
├── tempfile v3.27.0  ├── thiserror v2.0.21  ├── toml v0.8.23  ├── webp v0.3.1  └── zip v9.0.5   (no windows-sys yet)
```
- 2026-10-22, implementer (S-043a):
  - Implementation: in `tree.rs` above the test module: the two constants, the `JobOps` trait, `start_in_job` with a single cleanup path (a helper `assign_and_resume` assigns, then reads the threads, returns an error for an empty list, then resumes; on any error after the start: `terminate_process`, then close the job, then the error), `threads_of`, and `has_exe_extension` (`.exe` or `.com` in any case and after a non-empty stem). All with `#[cfg_attr(not(windows), allow(dead_code))]`. In `non_unix.rs`: `WinOps` with `CreateJobObjectW`, `SetInformationJobObject` (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`), `creation_flags(CREATE_SUSPENDED | CREATE_NO_WINDOW)`, `AssignProcessToJobObject`, and a Toolhelp thread snapshot passed through `tree::threads_of`, then `OpenThread` and `ResumeThread`, and `TerminateProcess` on failure; `spawn_in_job` and `terminate_job` (`TerminateJobObject`), and an owned handle closed on drop; `is_executable` equals `is_file() && has_exe_extension`; a compile-time check of the two constants against `windows_sys`; and the non-Linux `kill_group` was removed. In `run_child`, a start line and a terminate line with `#[cfg(not(unix))]` only, and the Linux lines as they are. `windows-sys` 0.61 under `[target.'cfg(windows)'.dependencies]` for engines with the features Foundation, Security, Diagnostics_ToolHelp, JobObjects and Threading.
  - Correction to the story text: AC5 has eight names, not nine (the manager accepted the tester's choice), so the test case table row was corrected to "the eight"; the test name `ac5_has_exe_extension_for_the_nine_names` stays unchanged.
  - Note for S-043b: on Windows `find_program` does not find the two names `soffice` and `libreoffice` without an extension until S-043b adds the `.exe` and `.com` names, and this is expected.
  - Gates (each command inside `systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200%` and `-j 2`): `C` clean (fmt, and clippy for the whole workspace with all targets); `cargo test -j 2 --all` 670 passed and 0 failed (`kawaro-engines` lib 51, `office_engine` 12, `pdf_ghostscript` 6 and `ffmpeg_engine` 23); `W` clean, and `cargo check --target x86_64-pc-windows-gnu -p kawaro-engines --lib` clean; `X` lists `windows-sys v0.61.2` as the only new entry; `grep -c '^\[\[package\]\]' Cargo.lock` gives 671, and the `Cargo.lock` diff is one line in the `kawaro-engines` array; `scripts/design-lint.sh` clean; the two inline test module diffs are empty; `git diff --stat main...HEAD -- crates/ui` is empty.
  - `M` on a copy in the session folder, with the diff `git diff c3619de -- crates/engines/src` and `CARGO_BUILD_JOBS=2 cargo mutants --jobs 1 -p kawaro-engines`: 12 mutants, 10 caught, 2 unviable, and none surviving.
- 2026-10-22, reviewer (S-043a): **PASS**.
  - Gates (each command inside `systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200%` and `-j 2`): `cargo fmt --check` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo test --all` 670 passed and 0 failed (`kawaro-engines` lib 51, `office_engine` 12, `pdf_ghostscript` 6 and `ffmpeg_engine` 23); `W` clean and `cargo check --target x86_64-pc-windows-gnu -p kawaro-engines --lib` clean; `X` lists `windows-sys v0.61.2` as the only new entry; `[[package]]` in `Cargo.lock` 671; `scripts/design-lint.sh` clean; no long dash in the diff; `git diff --stat main...HEAD -- crates/ui` is empty; `git diff f800d92..HEAD -- 'crates/*/tests/*'` is empty, and the two inline module diffs are empty.
  - `M` on a copy in the session folder, `cargo mutants --in-place -p kawaro-engines --in-diff` with the diff `f800d92..HEAD` and `CARGO_BUILD_JOBS=2`: 12 mutants, 10 caught, 2 unviable (`Default` for a handle and an associated type), 0 surviving, 0 timeouts.
  - Criteria mapping: AC1, AC2, AC3 (the two constants) and AC5 by the six `tree.rs` tests; AC4 on Linux by the `run_child` and `office_engine` tests with no change, and the Linux `run_child` changed only by the added `#[cfg(unix)]`; AC3 and AC4 for Windows and AC6 by `W`, `X` and reading.
  - Check of `non_unix.rs`: the job is created and `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` is set before the start; `CREATE_SUSPENDED | CREATE_NO_WINDOW` on the command, and the two constants are checked at compile time; a single cleanup path after the start (`TerminateProcess`, then close the job) for every failure, and the job is closed by drop on a failure before the start; the job handle is not inheritable (default null security), so a child that inherited it cannot stop kill-on-close from working; the Toolhelp snapshot handle is wrapped after an `INVALID_HANDLE_VALUE` check and closed on drop; resume is only for the child id's threads, through `tree::threads_of`, and the id is not reused because the `Child` handle is open; `OpenThread` with `THREAD_SUSPEND_RESUME`, and a `ResumeThread` failure is checked with `u32::MAX`; cancel, timeout and exit call `TerminateJobObject`, and the early exit with `try_wait()?` skips it, but dropping the job kills the tree. The `unsafe` blocks are sound.
  - Not applicable: screenshots, performance and UI strings (no change in `crates/ui` and no strings). The CHANGELOG.md line was added by the implementer as the story asks, so no second line.
  - Non-blocking notes:
    - Medium, outside AC1 to AC6: `crates/engines/src/ffmpeg.rs:148` (and `:65`, `:77` and `:92`) starts ffmpeg and ffprobe with no `CREATE_NO_WINDOW` and no job, and the live list in S-052 expects "No console window flashes when ffmpeg starts". No story covers it today; the reviewer suggests assigning it to S-043b or a new story, at the planner's decision. The CHANGELOG.md line mentions only LibreOffice and Ghostscript, so it is accurate.
    - Low, accepted deviation: `crates/engines/src/office.rs:301` conditions `mod non_unix` on `not(unix)` while `windows-sys` is under `cfg(windows)`, so a target that is neither unix nor windows does not build. No supported target is affected; `cfg(windows)` is suggested later.
    - Accepted: `find_program` with the bare names does not find `soffice` on Windows until S-043b, and it did not find it before this change either, so there is no regression.
- 2026-10-22, tester (S-043b):
  - Tests commit `863709c`. Contract pinned at the top of `crates/engines/src/locate.rs` (new file, test module only; the implementation goes above it), and `mod locate;` added to `crates/engines/src/lib.rs` (not in the story's file list, needed to compile the module):
    - `enum Os { Unix, Windows }` and `enum Program { Office, Ghostscript, Ffmpeg, Ffprobe }` (`Clone, Copy, Debug, PartialEq`); `fn names(os, program) -> &'static [&'static str]`.
    - `trait Registry { fn value(&self, key: &str, name: &str) -> Option<String>; fn subkeys(&self, key: &str) -> Vec<String>; }`, keys under HKLM without the hive (`SOFTWARE\LibreOffice\UNO\InstallPath`), `""` is the default value. The fake compares keys and names without case and accepts an optional `HKLM\` prefix.
    - `fn search(os: Os, program: Program, path_dirs: &[PathBuf], env: &dyn Fn(&str) -> Option<OsString>, exe_dir: Option<&Path>, registry: &dyn Registry) -> Option<PathBuf>`.
    - `fn newest_gs<'a>(names: &[&'a str]) -> Option<&'a str>`.
    - `office::file_url_str(path: &str, windows: bool) -> String`.
  - Tests:
    - `crates/engines/src/locate.rs`: `ac7_windows_names_per_program`, `ac7_unix_names_as_today`, `ac7_windows_office_path_wins_over_program_files_and_registry`, `ac7_windows_office_com_before_exe_in_one_dir`, `ac7_windows_office_program_files_wins_over_registry`, `ac7_windows_office_registry_install_path_last`, `ac7_windows_office_none_when_missing_everywhere`, `ac7_windows_ghostscript_path_wins_and_64_before_32`, `ac7_ac8_windows_ghostscript_newest_program_files_dir_by_number`, `ac7_windows_ghostscript_32_bit_name_in_program_files`, `ac7_ac8_windows_ghostscript_registry_newest_version_key_by_number`, `ac7_windows_ghostscript_none_when_missing_everywhere`, `ac7_windows_ffmpeg_path_wins_over_exe_dir`, `ac7_windows_ffmpeg_and_ffprobe_in_exe_dir_ffmpeg_bin`, `ac7_unix_searches_path_only`, `ac7_unix_ignores_program_files_exe_dir_and_registry`, `ac8_newest_gs_compares_versions_by_number`.
    - `crates/engines/src/office.rs` test module: `ac10_file_url_str_windows_drive_letter_and_space`, `ac10_file_url_str_windows_arabic_name`, `ac10_file_url_str_windows_drops_verbatim_prefix`, `ac10_file_url_str_unix_unchanged` (also `file_url` equals `file_url_str(.., false)` off Windows), `ac11_office_without_program_names_libreoffice_and_link`.
    - `crates/engines/src/pdf.rs` test module: `ac11_ghostscript_without_program_names_it_and_link` (`compress` and `to-images`).
    - `crates/engines/tests/windows_engines.rs`: `ac11_office_missing_program_names_libreoffice_and_link`, `ac11_ghostscript_missing_program_names_it_and_link`, `ac11_ffmpeg_missing_programs_names_ffmpeg_and_link`, `ac13_every_ffmpeg_command_goes_through_one_no_window_function`.
  - Ambiguities and choices:
    1. AC7's `names` slot is a `Program` enum, with ffprobe as its own program, because the known dirs and the registry depend on the program, not only on its names. `names(os, program)` gives the lists.
    2. Order inside `PATH` stays as `find_program` today (names first, then dirs), which is what "Linux as today" requires. Across tiers PATH, then known dirs, then registry. Within one dir `.com` before `.exe`. Name against dir ties across different dirs on Windows are not pinned.
    3. The registry trait needs subkey enumeration for the Ghostscript version keys; AC9 names only `RegGetValueW`, so `locate/windows.rs` also needs `RegEnumKeyExW` (or similar). The Ghostscript value read is `GS_DLL` (full path of the DLL in `bin`, its parent is the folder), and LibreOffice's is the default value of `...\UNO\InstallPath`. Both are my choice: ADR 0005 marks the keys as assumed, to be verified in the VM (S-052).
    4. The registry version rule is tested through `search` with bare version subkeys (`9.56.1` larger as text, `10.10.0` newest), not by calling `newest_gs` with names without `gs`.
    5. AC11 "when the program is missing" is tested both ways: no program at all (`program: None`, inline, built with struct update syntax, so only the field name `program` is pinned) and a configured path that does not exist (public `with_program` / `with_binaries`). Name checks: `LibreOffice`, `Ghostscript`, `ffmpeg` (any case).
    6. AC13: exactly one `Command::new(` in the non-test part of `ffmpeg.rs`, and the enclosing fn (with its attributes) holds `cfg(windows)`, `creation_flags` and `CREATE_NO_WINDOW`; `ffmpeg.rs` has no `const CREATE_NO_WINDOW` of its own. Call sites are not counted. The constant in `office/tree.rs` must become reachable from ffmpeg.rs (today `mod tree;` is private).
    7. The executable helper in `locate.rs` sets the exec bit under `#[cfg(unix)]` only, so the module also builds for the Windows target.
  - For the reviewer, not tests: `W` (and `locate/windows.rs` in `exclude_globs`), the `[[package]]` count, AC12 docs, CHANGELOG.md, `C`, `M`, and `git diff --stat main...HEAD -- crates/ui`.
  - Tests unchanged after `863709c` (all must print nothing). The S-043a command for `office.rs` (keyed on `c3619de`) is superseded by the one below, since this commit adds tests to that module:
    `git diff 863709c -- 'crates/*/tests/*'`
    `diff <(git show 863709c:crates/engines/src/locate.rs | sed -n '/^#\[cfg(test)\]/,$p') <(sed -n '/^#\[cfg(test)\]/,$p' crates/engines/src/locate.rs)`
    `diff <(git show 863709c:crates/engines/src/office.rs | sed -n '/^#\[cfg(test)\]/,$p') <(sed -n '/^#\[cfg(test)\]/,$p' crates/engines/src/office.rs)`
    `diff <(git show 863709c:crates/engines/src/pdf.rs | sed -n '/^#\[cfg(test)\]/,$p') <(sed -n '/^#\[cfg(test)\]/,$p' crates/engines/src/pdf.rs)`
    `diff <(git show 863709c:crates/engines/src/office/tree.rs | sed -n '/^#\[cfg(test)\]/,$p') <(sed -n '/^#\[cfg(test)\]/,$p' crates/engines/src/office/tree.rs)`
    `diff <(git show 863709c:crates/engines/src/ffmpeg.rs | sed -n '/^#\[cfg(test)\]/,$p') <(sed -n '/^#\[cfg(test)\]/,$p' crates/engines/src/ffmpeg.rs)`
  - Note: the engines lib test binary does not build until the implementation lands, so every existing unit test in it is red until then (as in S-043a). The integration tests build.
  - Logic check: a throwaway copy in the session scratch dir (deleted, not merged) with a short implementation of the contract passed all 74 lib tests and the 4 in `windows_engines.rs`, with clippy `--all-targets -D warnings` clean on Linux; on the Windows target the new test code compiled (the only errors were the pre-existing unix-only tests in `office.rs` and the stub's own private-constant use). It caught one contract wording error (PATH order), fixed in the header before the commit. Here `cargo fmt -p kawaro-engines --check` is clean.
  - Failing output (each command inside `systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200%`):
```
$ cargo test -j 2 -p kawaro-engines --lib
      2 error[E0405]: cannot find trait `Registry` in this scope
      7 error[E0425]: cannot find function `file_url_str` in this scope
      8 error[E0425]: cannot find function `names` in this scope
      3 error[E0425]: cannot find function `newest_gs` in this scope
      5 error[E0425]: cannot find function `search` in this scope
      2 error[E0425]: cannot find type `Program` in this scope
     13 error[E0433]: cannot find type `Os` in this scope
     33 error[E0433]: cannot find type `Program` in this scope
warning: unused import: `super::*`
error: could not compile `kawaro-engines` (lib test) due to 73 previous errors; 1 warning emitted

$ cargo test -j 2 -p kawaro-engines --test windows_engines
---- ac11_ffmpeg_missing_programs_names_ffmpeg_and_link stdout ----
panicked at crates/engines/tests/windows_engines.rs:71:5:
`/nonexistent/ffmpeg` or `/nonexistent/ffprobe` not found
---- ac11_ghostscript_missing_program_names_it_and_link stdout ----
panicked at crates/engines/tests/windows_engines.rs:55:5:
/nonexistent/gs not found
---- ac13_every_ffmpeg_command_goes_through_one_no_window_function stdout ----
panicked at crates/engines/tests/windows_engines.rs:109:5:
assertion `left == right` failed: every ffmpeg and ffprobe command must be built in one function; `Command::new(` at lines [65, 77, 92, 148]
  left: 4
 right: 1
---- ac11_office_missing_program_names_libreoffice_and_link stdout ----
panicked at crates/engines/tests/windows_engines.rs:40:5:
/nonexistent/soffice not found
test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test -j 2 -p kawaro-engines --test office_engine --test pdf_ghostscript --test ffmpeg_engine   # stay green
test result: ok. 23 passed; 0 failed   (ffmpeg_engine)
test result: ok. 12 passed; 0 failed   (office_engine)
test result: ok. 6 passed; 0 failed    (pdf_ghostscript)
```
- 2026-10-22, implementer (S-043b):
  - Manager decisions applied: the tester's choices are accepted (Program enum with Ffprobe; PATH, then known dirs, then registry; `.com` before `.exe`; `GS_DLL` and the LibreOffice UNO `InstallPath` default value as assumptions to check in S-052; `RegEnumKeyExW` with the `Win32_System_Registry` feature). The S-043b file list is extended by the manager with `crates/engines/src/lib.rs` and the non-test part of `crates/engines/src/office/tree.rs`: `CREATE_NO_WINDOW` is now `pub(crate)` and `mod tree` is `pub(crate) mod tree` in `office.rs`, so `ffmpeg.rs` reads `crate::office::tree::CREATE_NO_WINDOW`. `lib.rs` is unchanged by me (the tester's `mod locate;` is enough).
  - `crates/engines/src/locate.rs` (above the tester's header and test module): `Os`, `Program`, `names`, `Registry`, `search`, `newest_gs` (one `newest(names, prefix)` helper with a `Vec<u32>` version and `max_by_key`, shared with the registry version subkeys), and `find(program)`, the live glue (`PATH`, `std::env::var_os`, `current_exe` folder, registry). `const OS` comes from `cfg!(windows)`, so the Windows tiers build and run on Linux and `newest_gs` needs no `allow(dead_code)` (clippy `--all-targets` is clean without it; the story asked for the attribute only if it was dead). Off Windows the registry is an empty `SystemRegistry` that `search` never asks.
  - `crates/engines/src/locate/windows.rs`: `SystemRegistry` over `RegGetValueW` (`RRF_RT_REG_SZ`, size query then read), `RegOpenKeyExW`, `RegEnumKeyExW` and `RegCloseKey`; added to `exclude_globs`.
  - Wiring: `OfficeEngine::default` and `GhostscriptEngine::default` call `locate::find` (still once per process); `FfmpegEngine::default` calls it for both programs and keeps the bare name when one is not found, so on Linux the stored name and the message stay as before. `find_program` stays in `office.rs` (its tests use it) and is step 1 of `search`.
  - AC10: `file_url_str(path, windows)`; `file_url` calls it with `cfg!(windows)`. Windows mode drops `\\?\`, turns `\` into `/`, prefixes `/` and keeps `:`; Unix mode is byte for byte the old encoder.
  - AC11: `office::missing(name, url, tried)` gives `"<name> not found[ at <path>]; install it from <url>"` for LibreOffice and Ghostscript, both for no program and for a spawn `NotFound`. ffmpeg keeps its path message and adds `; install FFmpeg from https://ffmpeg.org/download.html`. The encoder messages are unchanged.
  - AC13: one `fn command(bin)` in `ffmpeg.rs` holds the only `Command::new(`, with `#[cfg(windows)] cmd.creation_flags(crate::office::tree::CREATE_NO_WINDOW)` and `#[cfg_attr(not(windows), allow(unused_mut))]` on the `let mut`. The four call sites use it.
  - `.cargo/mutants.toml`: `locate/windows.rs` in `exclude_globs`; `exclude_re` adds `replace find -> Option<PathBuf> with None` (live glue; the Office and Ghostscript tests skip when the program is absent and ffmpeg falls back to the bare name the OS resolves the same way) and the two bodies of the empty off-Windows `SystemRegistry` (never asked on Unix). `find`'s other mutant (`Some(Default::default())`) stays and is caught.
  - Docs: `docs/engines.md` new section "Finding the programs (S-043b)" with Linux and Windows columns (names, install folder, registry, download page), the office "Program" bullet and the windows-sys row; `README.md` one paragraph in Arabic and one in English; the S-043 line in `CHANGELOG.md` edited (now "Windows engines (S-043)").
  - Gates (each cargo command inside `systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200%`, `-j 2`):
    - `cargo fmt --check`: clean. `cargo clippy -j 2 --workspace --all-targets -- -D warnings`: clean.
    - Windows target (`CARGO_TARGET_DIR=$HOME/.cache/kawaro-win-target`): `cargo clippy --target x86_64-pc-windows-gnu -p kawaro-engines --lib -- -D warnings` clean; same for `-p kawaro-cli` clean; `cargo check --target x86_64-pc-windows-gnu -p kawaro-engines --lib -p kawaro-cli` clean. `X` lists `windows-sys v0.61.2` as before; `[[package]]` count 671; `Cargo.lock` unchanged.
    - `cargo test -j 2 --all`: 697 passed, 0 failed, 1 ignored (engines lib 74, `windows_engines` 4).
    - `cargo mutants --jobs 1 --in-diff <git diff 8160592 -- crates/engines/src> -p kawaro-engines` in a copied tree, `CARGO_BUILD_JOBS=2`: 43 mutants, 41 caught, 2 unviable (`command -> Command with Default::default()`, `missing -> EngineError with Default::default()`), 0 missed, 0 timeouts.
    - `scripts/design-lint.sh`: clean. No em or en dash in the diff.
    - The six test-unchanged commands above print nothing. `crates/ui` untouched.
  - Follow-up: `docs/engines.md` "Finding the programs" now says the lookup runs once per process for LibreOffice and Ghostscript and once per engine for ffmpeg (no `OnceLock` added). A grep found no old message text (`LibreOffice (soffice)`, `Ghostscript (gs)`) left in code or docs, and `fn find(` returning `Option<PathBuf>` exists only in `locate.rs`, so the new `exclude_re` entry hides no other mutant.
- 2026-10-22, reviewer (S-043b): **PASS**.
  - Gates (each cargo command inside `systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200%`, `-j 2`): `cargo fmt --check` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo test --all` 697 passed, 0 failed, 1 ignored (the ignored one is the fixture `regenerate` test, not S-043b; engines lib 74, `windows_engines` 4); Windows target (`CARGO_TARGET_DIR=$HOME/.cache/kawaro-win-target`) clippy `-D warnings` clean for `-p kawaro-engines --lib` and `-p kawaro-cli`, `cargo check` clean for both; `X` lists `windows-sys v0.61.2` as before; `[[package]]` in `Cargo.lock` 671, `Cargo.lock` unchanged since `8160592`; `scripts/design-lint.sh` clean; no em or en dash in `git diff 4be2ce1..HEAD`; `git diff --stat main...HEAD -- crates/ui` empty; the six tests-unchanged commands print nothing.
  - `M` in a copied tree (`git archive HEAD`), `git diff 8160592..HEAD -- crates/engines/src`, `CARGO_BUILD_JOBS=2 cargo mutants --in-place -p kawaro-engines --in-diff`: 43 mutants, 41 caught, 2 unviable (`command` and `missing` with `Default::default()`), 0 missed, 0 timeouts.
  - `exclude_re` checked by listing, not grep: `cargo mutants --list --no-config --workspace` shows the three new patterns match only `locate.rs:179` (`find -> None`), the off-Windows `SystemRegistry` stub bodies in `locate.rs:200,203`, and `locate/windows.rs` (already in `exclude_globs`). The `find -> None` exclusion is justified: `find` only passes the live `PATH`, environment, `current_exe` and registry to `search`, which carries the order and is fully mutated; the engine tests skip when the program is absent, so the mutant is not observable without environment-dependent tests.
  - AC mapping: AC7 and AC8 by the 17 `locate::tests`; AC10 by the four `office::tests::ac10_*` (plus the old `file_url` tests unchanged); AC11 by `office::tests::ac11_*`, `pdf::tests::ac11_*` and three tests in `windows_engines.rs`, and the `libx264` encoder tests stay green; AC13 by `ac13_every_ffmpeg_command_goes_through_one_no_window_function` and `W`; AC9 and AC12 by reading and `W`.
  - Code checks: Linux lookup is `find_program(names, path_dirs)` with the same names as before and stops after `PATH` on Unix; `path_dirs` uses `std::env::split_paths`, so the Windows `PATH` tier splits on `;` with drive letters intact. Registry reader: `value` opens no key, sizes the buffer with `div_ceil(2)` from the queried byte count and passes `buf.len() * 2`, uses `RRF_RT_REG_SZ` (which also guarantees the nul, and `until_nul` cuts at it), and a value that grows between the two calls gives `None`; `subkeys` returns early only when `RegOpenKeyExW` failed (no handle), otherwise the single `RegCloseKey` after the loop runs on every path, and 256 characters covers the 255 character key name limit. `file_url_str` matches the four AC10 strings; it is only applied to the private temporary profile folder (`office.rs:115`), so UNC is not reachable. One `fn command` in `ffmpeg.rs` holds the only `Command::new` with `creation_flags(CREATE_NO_WINDOW)` under `cfg(windows)`. `kawaro_engines::all()` runs once at daemon or CLI start, so the per engine ffmpeg lookup is off the hive path; no performance budget touched.
  - Not applicable: screenshots and UI strings (no `crates/ui` change). The CHANGELOG.md line is the single S-043 line edited as the story asks, so no second line.
  - Non-blocking notes:
    - Low, follow-up for S-052: a 64-bit build reads the 64-bit registry view and `%ProgramFiles%`, so a 32-bit LibreOffice or Ghostscript (under `WOW6432Node` and `Program Files (x86)`) is found only through `PATH`; in practice `gswin32c.exe` comes only from `PATH`. AC7 as written is met. Suggest checking `KEY_WOW64_32KEY` and `ProgramFiles(x86)` in the VM.
    - Low, for S-052 or the CLI story: `crates/cli/src/main.rs:348` (`on_path`) and the `doctor` tool list at `:362` check bare `ffmpeg`, `soffice` and `gs` in `PATH`, so on Windows `kawaro doctor` reports them missing even when `locate::find` finds them. The engines section of `doctor` uses `available()` and is correct.
    - Low: the drive colon kept unencoded is the RFC 8089 form (`file:///C:/...`), not a limit; the S-052 live LibreOffice launch confirms it. UNC needs no follow-up while `file_url` is used only for the temporary profile.
    - Cosmetic: on Linux a found ffmpeg is now stored as an absolute path, so the "not found" message (only reached when `-version` fails) shows the full path instead of the bare name.
