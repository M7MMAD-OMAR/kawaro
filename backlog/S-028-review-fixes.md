# S-028 Fix the findings of the review of v0.3.3..HEAD
Status: approved (manager, under the maintainer's delegation, 2026-10-24)

From a code review of `git diff v0.3.3..HEAD`. Behaviour fixes only. Windows only files (`*/windows.rs`, `office/non_unix.rs`) are not built by the Linux gates, so every decision lives in a pure function tested on Linux, as S-040b did, and those files are compiled and linted by `scripts/windows/check.sh`.

## Story
As a user and maintainer, I want the service to stay correct and quiet under errors on every platform, and the public repo to carry no personal path, so that a network share converts, a failing accept costs no CPU, and the scripts work for anyone.

## Split (in order, each one work day or less)
| Story | Items | Criteria | Files |
| --- | --- | --- | --- |
| S-028a | UNC URL, FFmpeg lookup cache, deadline keeps data | AC1 to AC3 | `crates/engines/src/{office.rs,ffmpeg.rs}`, `crates/core/src/transport/deadline.rs`, `crates/core/tests/deadline.rs` |
| S-028b | Accept backoff and hand over, request read without clone, socket cleanup in the transport | AC4 to AC7 | `crates/core/src/transport.rs`, `transport/{unix.rs,windows.rs}`, `crates/core/tests/{transport_plan.rs,transport.rs}`, `crates/daemon/src/{server.rs,main.rs}` |
| S-028c | No personal path, README split | AC8, AC9 | `scripts/windows/{build.sh,check.sh,make-iso.sh,README.md,README.ar.md}`, `tests/windows/{lib.sh,test_scripts.sh,test_ci_iso.sh,fixtures/objdump-*.txt}` |

## Acceptance criteria
- AC1 (`office.rs`): `file_url_str(r"\\?\UNC\srv\share\a b.docx", true)` is `file://srv/share/a%20b.docx` and `file_url_str(r"\\srv\share\x", true)` is `file://srv/share/x` (host form, two slashes). The four existing `ac10_file_url_str_*` tests pass unchanged.
- AC2 (`ffmpeg.rs`): `pub(crate) fn located() -> &'static (PathBuf, PathBuf)` runs `locate::find` for ffmpeg and ffprobe once per process (`OnceLock`); two calls return the same address (`std::ptr::eq`), and `FfmpegEngine::default()` has exactly those two paths.
- AC3 (`deadline.rs`), contract change: a read that returns `Ok(v)` is returned as is, even when the watcher fired at the same moment. A read that returns `Err` after the cancel gives `ErrorKind::TimedOut`; an `Err` before the deadline is returned as is. On Windows `T` is `usize` and a cancelled `ReadFile` is `ERROR_OPERATION_ABORTED`, an `Err`, so `Ok(0)` stays a real end of file.
- AC4 (`transport.rs`, every platform): `pub fn accept_backoff(consecutive_errors: u32) -> Duration` is 0 for 0 and 50 ms for every value from 1 to `u32::MAX`. `serve` resets its counter to 0 on each accepted stream and sleeps `accept_backoff(n)` after the n-th error in a row.
- AC5 (`transport.rs`): `pub enum NextInstance { Fresh, Keep, Empty }`, `pub struct AcceptPlan { pub hand_over: bool, pub next: NextInstance }`, `pub fn accept_plan(connected: bool, connect_error: Option<u32>, fresh_ok: bool) -> AcceptPlan`. `hand_over` is `connected || connect_error == Some(535)` (`ERROR_PIPE_CONNECTED`). `next` is `Fresh` when `fresh_ok`, `Empty` when the client is handed over and the fresh instance failed, and `Keep` when there is no client and it failed. `windows.rs` holds `next: Mutex<Option<OwnedHandle>>` and creates an instance first when it is empty, returning that error to the serve loop. `scripts/windows/check.sh` exits 0.
- AC6 (`server.rs`): `fn read_request(stream: &mut impl Read) -> io::Result<String>` reads one line through `BufReader::new(stream)`. `server.rs` has one `try_clone` (the `wait` subscriber).
- AC7: `Listener::cleanup(&self) -> transport::Cleanup`; dropping `Cleanup` removes the socket file on Unix and does nothing on Windows. `main.rs` holds it until it returns, and `grep -c socket_path crates/daemon/src/main.rs` gives 0. A `Drop for Listener` alone is not enough, because `serve` owns the listener on a thread that never returns.
- AC8: build.sh, check.sh and make-iso.sh default `CARGO_TARGET_DIR` to `${XDG_CACHE_HOME:-$HOME/.cache}/sbarhive-win-target`, and keep a value the caller sets. `git grep -n '/home/sbarah' -- ':!backlog'` prints nothing.
- AC9: `scripts/windows/README.md` (English) and `README.ar.md` (Arabic) each start with `[English](README.md) | [العربية](README.ar.md)`. README.md has no Arabic letter after line 1, and README.ar.md has the same commands, including the `virsh ... attach-disk` line.
- For each substory: the gates in CLAUDE.md, with no surviving mutant in changed non Windows code; `scripts/design-lint.sh` clean; the test count in `cargo test --all` rises by the new tests only; `crates/ui` untouched.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | The two exact strings above; `\\?\UNC\srv\share\ملف.docx` gives `file://srv/share/%D9%85%D9%84%D9%81.docx` | unit |
| AC2 | `ptr::eq(located(), located())`; `default().ffmpeg == located().0` and the same for ffprobe | unit |
| AC3 | A fake read that blocks until the cancel probe signals it, then returns `Ok(b"late\n")`: the result is `Ok(b"late\n")` with one cancel (fails today with `TimedOut`). The three existing tests stay as they are | unit |
| AC4 | `accept_backoff` at 0, 1, 2, 1000 and `u32::MAX`; the reviewer reads `serve` | unit, review |
| AC5 | Every row: `(true, None, true)` gives hand over and Fresh; `(false, Some(535), false)` gives hand over and Empty; `(true, None, false)` gives Empty; `(false, Some(232), true)` gives no hand over and Fresh; `(false, Some(232), false)` gives Keep; then `check.sh` | unit, build |
| AC6 | `read_request` on a `Cursor` over `{"cmd":"ping"}\n{"x":1}\n` gives the first line only. On `UnixStream::pair`, after `read_request` the same stream writes a reply that the peer reads. `grep -c try_clone` gives 1 | unit |
| AC7 | `cfg(unix)` in `transport.rs`: bind in a temp dir, the file exists, drop `Cleanup`, the file is gone; a second drop after removal does not panic; the grep above | integration |
| AC8 | For each script, with a fake cargo: `XDG_CACHE_HOME=$T/x` gives `$T/x/sbarhive-win-target`; with it unset and `HOME=$T/h`, `$T/h/.cache/sbarhive-win-target`; a caller value is kept. `lib.sh` computes `WIN_TARGET_DEFAULT` the same way, the fixtures' line 2 drops the path prefix, and the `test_ci_iso.sh` check rejects any `/home/` path | integration |
| AC9 | `head -1` of both files; `\p{Arabic}` in README.ar.md and not after line 1 in README.md; the AC11 check in `test_ci_iso.sh` reads the two files instead of two sections; both lint loops include README.ar.md | integration |

## Contract changes for the tester
- `crates/core/tests/deadline.rs` header: "whatever the cancelled `read` then returns, the result is `TimedOut`" becomes the AC3 rule. The three existing assertions do not change.
- `tests/windows/*` as in AC8 and AC9. S-044 AC1 and AC9 named the old default path; this story replaces it.

## Known trade-off
While the slot is `Empty` (AC5), a new Windows client gets `ERROR_FILE_NOT_FOUND`, and `connect_plan` reports `NotFound` at once. The in-flight client keeps the pipe name, and the next accept retries within 50 ms. The comment at `windows.rs:159` ("always one instance") is updated to say so.

## Out of scope (follow-ups)
- A `ProcessTree` type for `run_child`, and FFmpeg in the Windows kill-on-close job: FFmpeg outlives a crashed service on Windows today.
- The hold gate owning the activation token; one shared `Os` enum; `renameat2(RENAME_NOREPLACE)` on Linux.
- Overlapped I/O instead of a thread per timed pipe read; the CLI sending the real request before `ping`.
- WOW6432Node registry views, to be verified live in S-052.
- The personal path in older backlog entries (S-040 to S-044).

## CHANGELOG plan
One line under `[Unreleased]`, `### Fixed`:
- Review fixes (S-028): a document on a Windows network share opens in LibreOffice through a `file://server/share/...` URL, a failing accept pauses 50 ms instead of spinning, a Windows client is served even when the next pipe instance cannot be made, a pipe read that brings data at its deadline keeps it, FFmpeg is looked up once per process, the Unix transport removes its own socket, and the Windows build scripts default to `${XDG_CACHE_HOME:-$HOME/.cache}/sbarhive-win-target` with their README in separate English and Arabic files. «إصلاحات المراجعة: تفتح ملفات مجلدات الشبكة في ويندوز، ولا تستهلك الخدمة المعالج عند خطأ الاتصال، ولا تضيع بيانات وصلت عند انتهاء المهلة، ولا مسار شخصياً في السكربتات».

## Review log
- 2026-10-24, planner: written as a draft from the review brief; about two and a half days, split into S-028a to S-028c. AC3 changes the pinned deadline contract on purpose (see above). Approved the same day by the manager under the maintainer's delegation, as the brief states.
- 2026-10-24, tester: tests written from the criteria only, run on main at 789afb8 in a 6 GB scope with one job. Every new test fails for the missing behaviour:
  - AC1, `crates/engines/src/office.rs` (`s028_ac1_file_url_str_verbatim_unc_is_host_form`, `_plain_unc_is_host_form`, `_verbatim_unc_arabic_name`), run before AC2 was added: `left: "file:///UNC/srv/share/a%20b.docx" right: "file://srv/share/a%20b.docx"`, `left: "file://///srv/share/x" right: "file://srv/share/x"`, `left: "file:///UNC/srv/share/%D9%85%D9%84%D9%81.docx"`; 3 failed. The four `ac10_file_url_str_*` tests are unchanged.
  - AC2, `crates/engines/src/ffmpeg.rs` (`s028_ac2_located_is_one_value_per_process`, `s028_ac2_default_engine_has_the_located_paths`): `error[E0425]: cannot find function located in this scope`, so the engines lib tests do not compile until it exists.
  - AC3, `crates/core/tests/deadline.rs` (`s028_ac3_data_that_arrives_with_the_cancel_is_kept`): `data read at the deadline must be kept, got Custom { kind: TimedOut, error: "read deadline passed" }`; the three `ac10_*` tests pass unchanged. Only the header bullet changed, to the AC3 rule.
  - AC4 and AC5, `crates/core/tests/transport_plan.rs` (`s028_ac4_accept_backoff_is_zero_without_errors`, `s028_ac4_accept_backoff_is_50_ms_after_any_error`, `s028_ac5_*`, one test per row): `error[E0432]: unresolved imports AcceptPlan, NextInstance, accept_backoff, accept_plan`. The `serve` counter (AC4) and `scripts/windows/check.sh` (AC5) are review and build rows, left to the implementer and the reviewer.
  - AC6, `crates/daemon/src/server.rs` test module (`s028_ac6_read_request_reads_the_first_line_only`, `s028_ac6_the_same_stream_writes_the_reply_after_read_request`): `error[E0432]: unresolved import super::read_request`. `crates/daemon/tests/review_sources.rs` `s028_ac6_server_rs_has_one_try_clone`: `left: 2 right: 1`. The test module names no `try_clone`, so it does not count itself.
  - AC7, `crates/core/tests/transport.rs` (`s028_ac7_dropping_cleanup_removes_the_socket_file`, `s028_ac7_a_second_cleanup_dropped_after_removal_does_not_panic`): `cannot find type Cleanup in module transport`, `no method named cleanup found for struct Listener`. `review_sources.rs` `s028_ac7_main_rs_names_no_socket_path`: `left: 2 right: 0`.
  - AC8, `tests/windows/test_scripts.sh` `s028_ac8_scripts_default_the_target_dir_to_the_xdg_cache`, with a fake cargo and a fake genisoimage that writes nothing: for build.sh, check.sh and make-iso.sh, `XDG_CACHE_HOME` set and unset both give the old fixed path instead of `$T/x/sbarhive-win-target` and `$T/h/.cache/sbarhive-win-target` (6 FAIL); each header names a `/home/` path (3 FAIL); the `git grep` for the maintainer's home prints the README and the three scripts. A caller's `CARGO_TARGET_DIR` is kept already; that case is a guard. The real ISO in the cache keeps its mtime and size. `lib.sh` computes `WIN_TARGET_DEFAULT` from `${XDG_CACHE_HOME:-$HOME/.cache}`, the fixtures' line 2 drops the path prefix (S-044a AC2 still passes), and the `test_ci_iso.sh` check rejects any `/home/` path.
  - AC9, `tests/windows/test_ci_iso.sh` `s028_ac9_readme_is_split_into_english_and_arabic_files`: README.md line 1 is `# Windows build scripts / ...`, not the switch line; README.md has Arabic after line 1; README.ar.md is missing. AC11 now reads the two files (`README.ar.md has the attach command` FAIL), and both lint loops fail on a missing README.ar.md instead of skipping it; S-044a AC7 reads Arabic from README.ar.md.
  - Test side fix: S-044a AC4 in `test_scripts.sh` diffs the seven files from 5be5270 to 789afb8, the last commit before S-028, because this story adds tests and code to `office.rs` on purpose.
- 2026-10-24, implementer: AC1 to AC9 implemented. Every S-028 test passes, and so do `tests/windows/test_scripts.sh` (78 checks), `test_ci_iso.sh` (86), `test_heavy.sh` (11, which runs the real `scripts/windows/check.sh`; a direct run of it exits 0 too), `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `scripts/design-lint.sh` and `cargo mutants --in-place --in-diff` on the changed non Windows lines (17 mutants: 12 caught, 5 unviable, none missed). `cargo test --all --no-fail-fast` gives 746 passed, 1 failed. Status stays `approved`: the one failure is a test against test conflict that no implementation can fix, so it goes back to the tester.
  - Failing test: `crates/cli/tests/no_unix_net.rs` `ac5_no_std_os_unix_net_in_cli_or_daemon_sources` (S-040a AC5) reads every `.rs` file under `crates/daemon/src` and fails on any line that holds the text `std::os::unix::net`. The S-028 test `s028_ac6_the_same_stream_writes_the_reply_after_read_request` in the `server.rs` test module (added in f31de7e, line 159 there) builds its pair with `std::os::unix::net::UnixStream::pair()`, so the old test is red from f31de7e on, before any implementation. An alias or a `use` still holds the text, and moving or rewriting the test is editing a test.
  - Suggested test side fix: build the pair through `sbarhive_core::transport` (bind a `Listener` on an `Endpoint::from_path` in a `tempfile` dir, `Stream::connect`, then `accept`), which keeps S-040a AC5 whole; `tempfile` is already a daemon dev-dependency. `read_request` takes `&mut impl Read`, so a `transport::Stream` works as is.
  - Seen, not from S-028: `tests/windows/test_heavy_iso.sh` AC9 fails because `debug/deps` in the real cache holds 36 stale test programs from older builds next to the 36 current ones (`make-iso.sh` says stale hashes are not told apart); cleaning that folder is the documented fix.
  - Windows accept (AC5): on `Keep` the old instance gets `DisconnectNamedPipe` before it waits again, so a client that connected and left (`ERROR_NO_DATA`) does not fail the same instance forever.
- 2026-10-24, tester: `s028_ac6_the_same_stream_writes_the_reply_after_read_request` used `UnixStream::pair` from the standard library's Unix socket module, which broke the pinned S-040a AC5 test `crates/cli/tests/no_unix_net.rs`. It now gets its pair from `sbarhive_core::transport` (a `Listener` bound on `Endpoint::from_path` in a temp dir, `Stream::connect`, then `accept`) and asserts the same. On f5806dc: `-p sbarhive-daemon --bin sbarhived s028_ac6` 2 passed, `-p sbarhive-cli --test no_unix_net` 1 passed.
