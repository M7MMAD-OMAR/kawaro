# S-030 Run every engine child in one killable process tree
Status: review

Follow-up named in S-028 "Out of scope". On Windows `ffmpeg.rs` spawns ffmpeg through its own `command()` (CREATE_NO_WINDOW only), outside the kill-on-close job that `run_child` uses, so ffmpeg.exe keeps writing the hidden temp file after the service dies. On every platform `supervise` cancel calls `child.kill()`, which spares the helpers ffmpeg started. One type, `ProcessTree`, owns the child and its tree for both engines. About one work day, so no split. Windows only files are not built by the Linux gates; `scripts/windows/check.sh` builds and lints them.

## Story
As a user, I want a cancelled or orphaned conversion to stop every process it started, so that no engine keeps running or writing after the task or the service is gone.

## Acceptance criteria
- AC1 (`crates/engines/src/process.rs`, declared in `lib.rs` as `pub(crate) mod process;`): `pub(crate) struct ProcessTree` with `pub(crate) fn spawn(cmd: &mut Command) -> io::Result<ProcessTree>`, `pub(crate) fn child_mut(&mut self) -> &mut Child` and `pub(crate) fn kill_all(&mut self)`, which kills every process in the tree and does not panic when called twice. `impl Drop for ProcessTree` calls `kill_all` unconditionally, then `wait`s on the child. `grep -c killed process.rs` gives 0; that it has no other branch a Linux test cannot observe is a reviewer read. Every grep in this story counts only code above the file's `#[cfg(test)]` line, and `<tester commit>` is the tester's commit recorded in the review log.
- AC2 (Unix, inline in `process.rs`): `spawn` sets `process_group(0)` before spawning; `kill_all` is `libc::killpg(<child pid>, SIGKILL)`.
- AC3 (Windows, `crates/engines/src/process/windows.rs`, `#[cfg(windows)] mod windows;`): it holds `WinOps` and its `JobOps` impl, spawns through `office::tree::start_in_job`, and `kill_all` calls `TerminateJobObject`. The job `OwnedHandle` is a field of `ProcessTree`, so the tree dies when the service process dies.
- AC4 (`office/non_unix.rs`): only `is_executable` is left; `grep -cE 'WinOps|spawn_in_job|terminate_job|TerminateJobObject'` gives 0, and `grep -ci job` on the file gives 0 (module doc updated). `office/tree.rs` stays in place; the items `windows.rs` needs may widen from `pub(super)` to `pub(crate)`. `git diff <tester commit> -- crates/engines/src/office/tree.rs` shows no change on or after the `#[cfg(test)]` line.
- AC5 (`office.rs`): `run_child` spawns with `ProcessTree::spawn` and calls `kill_all` after the loop, on success too, as today. `grep -cE 'process_group|killpg|spawn_in_job|terminate_job|kill_group'` in `office.rs` gives 0.
- AC6 (`ffmpeg.rs`): `run` spawns with `ProcessTree::spawn(command(&self.ffmpeg).args(..)...)`. The signature is `fn supervise(tree: ProcessTree, duration: Option<f64>, progress: Progress, cancel: &CancelToken) -> Result<(), EngineError>`; it keeps its own loop (not merged into `run_child`), and on cancel calls `kill_all`, then waits, then returns `Err(EngineError::Cancelled)`. In `ffmpeg.rs`, `grep -c '\.spawn()'` and `grep -c '\.kill()'` each give 0 (`.kill_all()` does not match); the ffprobe `.output()` and the `runs_ok` `.status()` stay on bare `command()`.
- AC7 (`.cargo/mutants.toml`): `crates/engines/src/process/windows.rs` is in `exclude_globs`, with a comment in the style of the S-043a one; `office/non_unix.rs` stays excluded.
- AC8 (unchanged tests): `git diff <tester commit> --` of the test modules in `locate.rs`, `office/tree.rs` and `crates/cli/src/spawn.rs`, and of `crates/*/tests/`, is empty. `cargo test --all` passes, with the count rising by the new tests only.
- AC9 (gates): `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all`, `scripts/design-lint.sh` clean; `cargo mutants --in-place --in-diff` on the changed non Windows files gives 0 missed; `scripts/windows/check.sh` exits 0; one CHANGELOG line, English and Arabic, under `[Unreleased]`, `### Fixed`.

## Test cases
All new tests are `#[cfg(unix)]`: `check.sh` builds `--all-targets` for `x86_64-pc-windows-gnu`, which has no `sh` or `libc::kill`. The tree is `sh -c 'sleep 30 & echo $!; wait'`. Reading the pid: take `child_mut().stdout`, read one line with `BufReader::read_line` on a helper thread that sends it over a channel, and `recv_timeout(3 s)` on the test side, panicking with a message on timeout. "Gone" means `/proc/<pid>` is absent or its `stat` state field is `Z`, polled every 20 ms for up to 2 s (`kill(pid, 0)` succeeds on a zombie, and an orphan may be reaped late).

| Criterion | Test | Type |
| --- | --- | --- |
| AC1, AC2 | Spawn the tree with stdout piped, read the grandchild pid, `kill_all`: grandchild and `sh` gone; a second `kill_all` does not panic. Fails to compile today (no `process` module) | unit |
| AC1 | Same tree, drop the `ProcessTree`: grandchild gone, and `/proc/<sh pid>` absent right after the drop (reaped, not a zombie of the test). Fails to compile today | unit |
| AC6 | `sh -c 'sleep 30 & echo $! > "$1"; wait' sh <pidfile>` with stdout and stderr piped, since `supervise` takes both pipes; poll the pidfile up to 3 s, set the `CancelToken`, call `supervise`: `Err(EngineError::Cancelled)` and grandchild gone. Fails today (`child.kill()` spares the grandchild); failing to compile before the new signature is acceptable | unit |
| AC5 | Existing `office_run_child_*` tests and `office_cancel_kills_the_process_group` pass; new guard (passes today through `killpg`): `run_child` on the pidfile command with a 10 s timeout; a helper thread polls the pidfile up to 3 s for non-empty trimmed text, then cancels; the result is `Err(EngineError::Cancelled)`, not `Failed`, and the grandchild is gone | unit |
| AC3 to AC7 | The greps above with their counts; `check.sh` exits 0 | review, build |
| AC8, AC9 | The `git diff`, the pass count and the gates | review |

## Out of scope
- The ffprobe and `runs_ok` probes stay outside a tree.
- Unix has no parent death kill (no `PR_SET_PDEATHSIG`), so a group outlives a dead service on Unix.
- `killpg` after the leader is reaped can hit a reused pgid (existing behaviour).
- A live Windows test of the job (needs a virtual machine).

## CHANGELOG plan
- Engine process trees (S-030): FFmpeg runs in the same kill-on-close job as LibreOffice and Ghostscript on Windows, so it stops when the service dies, and cancelling a video conversion kills every helper process FFmpeg started. «يعمل FFmpeg داخل شجرة عمليات واحدة: يتوقف مع توقف الخدمة في ويندوز، ويوقف الإلغاء كل عملية بدأها».

## Review log
- 2026-10-24, planner: written from the approved design in the manager's brief; status approved by the manager under the maintainer's delegation. The `supervise` test reads its pid from a file, not stdout, because `supervise` takes both pipes. `office_cancel_kills_the_process_group` in `crates/engines/tests/office_engine.rs` already covers the `run_child` group kill end to end.
- 2026-10-24, tester: tests written from the acceptance criteria, all `#[cfg(unix)]`. `crates/engines/src/process.rs` holds only the `//!` doc and the test module; `lib.rs` declares `pub(crate) mod process;`.
  - `process::tests::s030_ac1_ac2_kill_all_kills_the_whole_tree_and_is_idempotent` (AC1, AC2) and `process::tests::s030_ac1_drop_kills_the_tree_and_reaps_the_child` (AC1): fail to compile, `error[E0425]: cannot find type `ProcessTree` in this scope --> crates/engines/src/process.rs:20:25` and `error[E0433]: cannot find type `ProcessTree` in this scope --> crates/engines/src/process.rs:26:24`.
  - `ffmpeg::tests::s030_ac6_supervise_cancel_kills_the_grandchild` (AC6): fails to compile, `error[E0433]: cannot find `ProcessTree` in `process` --> crates/engines/src/ffmpeg.rs:1281:36` (the new `supervise(tree: ProcessTree, ..)` signature). With the module and this test's tree swapped for a bare `cmd.spawn()` on today's `supervise`, it fails at run time: `panicked at crates/engines/src/ffmpeg.rs:1287:9: grandchild 2864292 survived the cancel`, so `child.kill()` spares the grandchild as the story says.
  - `office::tests::s030_ac5_run_child_cancel_kills_the_grandchild` (AC5 guard): passes today through `killpg` (`test office::tests::s030_ac5_run_child_cancel_kills_the_grandchild ... ok`, run with the `process` module switched off), as the story expects; `office_run_child_*` pass too.
  - Result of `cargo test -p sbarhive-engines --lib`: `error: could not compile `sbarhive-engines` (lib test) due to 3 previous errors`.
- 2026-10-24, implementer: `ProcessTree` in `process.rs` (Unix `process_group(0)` and `killpg`) and `process/windows.rs` (job, `WinOps`); `run_child` and ffmpeg `supervise` use it; `non_unix.rs` keeps only `is_executable`; mutants exclude and CHANGELOG added. All six S-030 tests pass; `scripts/windows/check.sh` exits 0; design-lint clean; fmt clean. Two gate failures need a test edit, which the implementer may not make:
  - `crates/core/tests/s029_sources.rs::s029_ac2_non_unix_uses_std_owned_handle` still requires `OwnedHandle` in `office/non_unix.rs`, which S-030 AC4 empties (the handle code moved to `process/windows.rs`). The S-029 test should read `process/windows.rs` now.
  - `cargo clippy --all-targets -D warnings` fails on `collapsible_if` in the tester's `read_pid` helpers: `ffmpeg.rs:1241` and `office.rs:752`. Fix: `if let Ok(text) = .. && let Ok(pid) = ..`.
  - `cargo mutants` not run.
