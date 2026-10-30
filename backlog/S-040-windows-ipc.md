# S-040 IPC transport behind one interface, then a named pipe on Windows
Status: approved for implementation on the windows/0.6.0 branch at the owner's request (2026-10-22); ADR 0005 is still proposed; merged into main locally on 2026-10-23 (not pushed); S-040a: done (PASS from the reviewer 2026-10-22); S-040b: done (PASS from the reviewer 2026-10-22)
Renumbered on 2026-10-23: this story was S-022 on windows/0.6.0 before the merge into main (main had already used S-022 and S-027). Commit messages and review-log hashes keep the old number.

Builds on ADR 0005 section 2. For release 0.6.0. Does not touch `crates/ui`.

## Story
As a user on Windows, I want `kawaro.exe` and `kawarod.exe` to talk over a pipe that no one else can open, so that the hive works as on Linux without another user or another machine sending commands to my daemon.

## Split (in order, each substory a working day or less)
| Story | Content | Criteria | Files |
| --- | --- | --- | --- |
| S-040a | The transport module with the current Unix implementation, the pure pipe name function, and moving the client and the daemon onto it | AC1 to AC7 | `crates/core/src/{lib.rs,transport.rs,transport/unix.rs,transport/windows.rs}` (seed), `crates/cli/src/{client.rs,main.rs}`, `crates/daemon/src/{main.rs,server.rs,state.rs}`, `.cargo/mutants.toml`, `CHANGELOG.md` |
| S-040b | The Windows pipe: DACL, rejecting remote clients, the single instance, the timeout with `CancelIoEx`, and the server's owner | AC8 to AC14 | `crates/core/Cargo.toml`, `Cargo.lock` (the core dependency array only), `crates/core/src/transport.rs` and `transport/{windows.rs,deadline.rs}`, `.cargo/mutants.toml`, `SECURITY.md`, `docs/architecture.md`, `docs/engines.md`, `CHANGELOG.md` |

## Acceptance criteria
S-040a:
- AC1: `kawaro_core::transport` provides the full interface (the names are proposals that the tester fixes): `endpoint() -> Endpoint`, `Listener::bind(&Endpoint) -> io::Result<Bound>` where `Bound` is either `Listening(Listener)` or `AlreadyRunning`, `Listener::accept()`, `Stream::connect(&Endpoint)`, `Stream::set_read_timeout(Option<Duration>)`, `Stream::try_clone()`, `Stream::server_pid() -> Option<u32>` (returns `None` on Unix), and `Stream` implements `Read` and `Write`. `ipc::socket_path()` and `SOCKET_NAME` keep their signature and value.
- AC2: `transport::pipe_name(sid: &str, session: u32) -> String` is a pure function: `pipe_name("S-1-5-21-1-2-3-1001", 1)` equals exactly `\\.\pipe\io.github.m7mmad_omar.Kawaro.S-1-5-21-1-2-3-1001.1`. A different SID or a different session gives a different name, and the longest possible SID (`S-1-5-21-4294967295-4294967295-4294967295-4294967295`) with session `4294967295` gives 256 characters or fewer.
- AC3: on Linux, `Listener` and `Stream` in a temporary directory: the line `{"cmd":"ping"}` from the client reaches the server as is, and its reply `{"status":"ok"}` reaches the client. Two lines written from a `Stream` and from its `try_clone` copy arrive in the same order.
- AC4: read timeout on Linux: a server that accepts and does not write, and the client with `set_read_timeout(Some(200 ms))`: `read_line` returns an error of kind `ErrorKind::TimedOut` (not `WouldBlock`) after 180 ms to 1000 ms. With `None`, a reply written after 300 ms is read.
- AC5: no `std::os::unix::net` in `crates/cli/src` or `crates/daemon/src`: `grep -rn 'std::os::unix::net' crates/cli/src crates/daemon/src` returns 0 lines. `State::subscribers` has type `HashMap<JobId, Vec<transport::Stream>>`, and `kawaro doctor` (`main.rs:384`) prints `endpoint()`.
- AC6: the daemon: `bind` gives `AlreadyRunning` when a daemon answers `ping` within 500 ms, so it prints `kawarod: already running` and exits with 0, as today. An old socket file that does not answer is removed and then bound.
- AC7: the Windows seed `transport/windows.rs`: every function in it returns `ErrorKind::Unsupported`, and the file is in `exclude_globs`. `cargo clippy -p kawaro-core --lib` for the Windows target has no warning.

S-040b:
- AC8: `transport::dacl_sddl(sid) -> String` is pure: `dacl_sddl("S-1-5-21-1-2-3-1001")` equals `D:P(A;;GA;;;S-1-5-21-1-2-3-1001)`, and contains neither `WD` nor `AN` nor `BU`. `windows.rs` builds the descriptor from it (`ConvertStringSecurityDescriptorToSecurityDescriptorW`) and passes it to every `CreateNamedPipeW`.
- AC9: `transport::bind_result(first_instance_error: Option<u32>) -> BindOutcome` is pure: `None` gives listening, `Some(5)` (`ERROR_ACCESS_DENIED`) gives `AlreadyRunning`, and any other code gives an error carrying the same code in `raw_os_error()`.
- AC10: `transport::deadline::read_with_deadline(timeout, read, cancel)` runs a watchdog thread: a fake read that blocks until `cancel` is called returns `ErrorKind::TimedOut` after 180 ms to 1000 ms for a 200 ms timeout, and `cancel` is called exactly once. A read that finishes within 50 ms returns its data, `cancel` is called 0 times, and no watchdog thread is left alive after returning (a counter in the test). On Windows, `cancel` is `CancelIoEx` on the pipe handle.
- AC11: `transport::connect_plan(results)` is pure, for connection attempts: `ERROR_FILE_NOT_FOUND` (2) gives `NotFound` at once without waiting, and `ERROR_PIPE_BUSY` (231) gives "wait with `WaitNamedPipeW`, then retry" up to 2000 ms, then `TimedOut`. And the result of the owner check: a server SID different from the client SID gives `PermissionDenied` and no byte is sent (a write counter in the test is 0).
- AC12: the creation flags are two pure functions in `transport.rs`: `open_mode(first: bool)` gives `0x00080003` for the first instance (`PIPE_ACCESS_DUPLEX` with `FILE_FLAG_FIRST_PIPE_INSTANCE`) and `0x3` for later ones, so later instances do not fail with `ERROR_ACCESS_DENIED`. And `pipe_mode()` gives `0x8` (`PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT` has value 0, with `PIPE_REJECT_REMOTE_CLIENTS`). `windows.rs` passes them to `CreateNamedPipeW`, and checks every constant at compile time against `windows_sys` (`const _: () = assert!(...)`), so `W` fails if a value differs. The owner check uses `GetNamedPipeServerProcessId`, then the process token, and `endpoint()` comes from `ProcessIdToSessionId` and the SID from `GetTokenInformation(TokenUser)`. The import check on the client executable comes in S-042 AC6.
- AC13: no new crate: the count of `[[package]]` in `Cargo.lock` stays 671, `cargo tree --target x86_64-pc-windows-gnu -p kawaro-core -e normal --depth 1` lists `windows-sys v0.61.2` as the only new one, and `cargo tree -p kawaro-core -e normal --depth 1` on Linux does not list it.
- AC14: `cargo check` and `cargo clippy -p kawaro-core -p kawaro-engines --lib` for the Windows target have no warning. `SECURITY.md` describes the pipe (the user alone, no remote clients) in English and Arabic, `docs/architecture.md` describes the single instance on both platforms, and `docs/engines.md` records windows-sys 0.61.2 (MIT OR Apache-2.0).

For each substory:
- One line for the whole story in `CHANGELOG.md` under `[Unreleased]`, in English with Arabic text in «»: S-040a adds it, and S-040b edits the same line and adds no other.
- `git diff --stat main...HEAD -- crates/ui` is empty.
- `C` on Linux with no warning. Pure functions called only by a Windows file are `pub` in core, or carry `#[cfg_attr(not(windows), allow(dead_code))]`.
- `M` with no surviving mutant.

## Check commands
Read-only tools alone (`grep`, `file`, `objdump` and `isoinfo`) run outside the scope.
- `C`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% sh -c 'cargo fmt --check && cargo clippy -j 2 --workspace --all-targets -- -D warnings'`
- `L`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% cargo test -j 2 -p kawaro-core -p kawaro-cli -p kawaro-daemon`
- `W`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_TARGET_DIR=$HOME/.cache/kawaro-win-target cargo clippy -j 2 --target x86_64-pc-windows-gnu -p kawaro-core --lib -- -D warnings` (and `-p kawaro-engines` is added in S-040b)
- `M`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_BUILD_JOBS=2 cargo mutants --jobs 1 --in-diff <diff>` (the changed files only; Windows files are excluded by `exclude_globs`)
- `X`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_TARGET_DIR=$HOME/.cache/kawaro-win-target cargo tree --target x86_64-pc-windows-gnu -p kawaro-core -e normal --depth 1`

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1, AC3 | `crates/core/tests/transport.rs`: bind, connect, the `ping` round trip and the order of the two lines through `try_clone`, and `server_pid()` equals `None`; `L` | integration |
| AC2, AC8, AC9, AC11, AC12 | Unit tests with the literal values, the codes 2, 5 and 231, and the `open_mode` and `pipe_mode` values, under `M`; and `W` proves the constants match | unit, build |
| AC4, AC10 | Unit tests with the stated times, the `cancel` counter and the thread counter | unit |
| AC5 | The stated `grep` command returns 0 lines; `kawaro doctor` in `crates/cli/tests/cli.rs` prints the path in the temporary directory | review, integration |
| AC6 | Two daemons on one `XDG_RUNTIME_DIR`: the second exits with 0 and its text is `already running`; and an old socket is replaced | integration |
| AC7, AC14 | `W` with no warning; the reviewer reads the three documents | build, review |
| AC13 | `grep -c '^\[\[package\]\]' Cargo.lock` gives 671, and `X`, and `cargo tree -p kawaro-core -e normal --depth 1` inside the same scope on Linux | review |

Stay green: `cargo test --workspace`, `crates/cli/tests/cli.rs` and `settings.rs` with their fake servers on `UnixListener` without modification (evidence that the protocol did not change), and `tests/e2e/run.sh`.

## Live verification deferred to S-052
- A second Windows user cannot open the pipe (`ERROR_ACCESS_DENIED`).
- A connection from another machine through `\\<host>\pipe\...` is rejected.
- A second `kawarod.exe` exits with 0 at once, and ten instances together leave one server.
- The client rejects a process from another account that grabbed the name first.
- The 10 s timeout in the client and the 500 ms timeout in the daemon check work with `CancelIoEx`.

## Out of scope
- Starting the daemon, its flags and `AllowSetForegroundWindow`: S-042.
- Checking the whole daemon for the Windows target: S-045 (depends on ui).
- `crates/ui/examples/hive_bench.rs` stays on `UnixStream`: S-045.

## Review log
- 2026-10-22, planner: the whole interface is fixed in S-040a (including `server_pid` and `AlreadyRunning`) so that S-040b and S-042 do not reopen it. Windows decisions live in pure functions tested and mutated on Linux, and the thin calls in the excluded `windows.rs`.
- 2026-10-22, tester (S-040a): tests for AC1 to AC7 written. The contract the tester fixed at the top of `crates/core/tests/transport.rs` (not to be modified): `transport::endpoint() -> Endpoint`, `Endpoint: Clone + Debug + Display` displayed on Unix like `ipc::socket_path().display()`, `Endpoint::from_path(impl Into<PathBuf>)` on Unix so the tests do not change `XDG_RUNTIME_DIR` (in edition 2024 `set_var` is unsafe and races with parallel tests), `enum Bound { Listening(Listener), AlreadyRunning }`, `Listener::bind(&Endpoint) -> io::Result<Bound>`, `Listener::accept(&self) -> io::Result<Stream>`, `Listener: Send`, `Stream::connect(&Endpoint)`, `set_read_timeout(&self, Option<Duration>) -> io::Result<()>`, `try_clone(&self) -> io::Result<Stream>`, `server_pid(&self) -> Option<u32>`, `Stream: Read + Write + Send`, and `pipe_name(&str, u32) -> String` on all platforms.
  - Files and tests:
    - `crates/core/tests/transport.rs`: `ac1_stream_is_read_write_send_and_listener_is_send`, `ac1_socket_name_and_socket_path_keep_their_value`, `ac1_endpoint_on_unix_is_the_socket_path`, `ac1_endpoint_from_path_displays_the_path`, `ac1_ac3_server_pid_is_none_on_unix`, `ac2_pipe_name_is_the_exact_literal`, `ac2_pipe_name_differs_by_sid_and_by_session`, `ac2_longest_sid_and_session_fit_in_256_chars`, `ac3_ping_line_reaches_the_server_and_the_reply_reaches_the_client`, `ac3_lines_from_a_stream_and_its_try_clone_arrive_in_order`, `ac4_read_timeout_is_timed_out_between_180_and_1000_ms`, `ac4_no_timeout_reads_a_reply_written_after_300_ms` (sets a timeout, then clears it with `None`), `ac6_bind_is_already_running_when_a_server_answers_ping`, `ac6_a_stale_socket_file_is_removed_and_bound`.
    - `crates/core/tests/transport_seed.rs`: `ac7_windows_seed_is_unsupported_and_excluded_from_mutants` (the file exists, contains `ErrorKind::Unsupported` and no `todo!`, and is listed in `exclude_globs`). The Windows build itself is proved by `W` at the reviewer; it passes today because the module does not exist.
    - `crates/cli/tests/no_unix_net.rs`: `ac5_no_std_os_unix_net_in_cli_or_daemon_sources` (the same `grep` command as a test).
    - `crates/cli/tests/cli.rs`: `ac5_doctor_prints_the_endpoint_in_the_runtime_dir`. No import of `transport` was added to this file, and its fake servers stay on `UnixListener` as the story requires.
    - `crates/daemon/tests/single_instance.rs`: `ac6_second_daemon_on_the_same_runtime_dir_says_already_running_and_exits_0`. The first daemon is a fake server on `transport::Listener`, and the second is the real `kawarod` with no display variables, with a 5 s timeout, then a kill.
  - Passes before implementation on purpose: `ac5_doctor_prints_the_endpoint_in_the_runtime_dir`, because `doctor` prints the socket path today, and Linux behaviour must not change. It is a regression guard, not evidence of implementation. Likewise, the AC6 behaviour exists today, and what makes its test fail is the fake server built on `transport`.
  - For the reviewer, not for tests: the type of `State::subscribers` (`HashMap<JobId, Vec<transport::Stream>>`), because the daemon is a binary crate and a text check is fragile, and `W` for AC7.
  - Ambiguity: a socket with a listener that accepts and does not answer `ping` within 500 ms: is it considered stale and removed? AC6 does not say, so it was not tested.
  - Check of the daemon test's logic: an uncommitted temporary copy with a fake server on `UnixListener` passed on the current `kawarod` (0.02 s), so no display or icon blocks it. `cargo clippy -D warnings` is clean on `kawaro-cli --tests` and `transport_seed`; `transport.rs` and `single_instance.rs` cannot be checked before the interface exists, so any warning in them comes from the test, not the implementation.
  - Failure output (`L` inside the scope, `-j 2`):
```
$ cargo test -j 2 -p kawaro-core --test transport
error[E0432]: unresolved import `kawaro_core::transport`
  --> crates/core/tests/transport.rs:28:20
   |
28 | use kawaro_core::transport::{self, Bound, Endpoint, Listener, Stream};
   |                    ^^^^^^^^^ could not find `transport` in `kawaro_core`
error: could not compile `kawaro-core` (test "transport") due to 1 previous error

$ cargo test -j 2 -p kawaro-core --test transport_seed
---- ac7_windows_seed_is_unsupported_and_excluded_from_mutants stdout ----
panicked at crates/core/tests/transport_seed.rs:12:29:
.../crates/core/src/transport/windows.rs must exist: No such file or directory (os error 2)
test result: FAILED. 0 passed; 1 failed

$ cargo test -j 2 -p kawaro-cli --test no_unix_net --test cli -- ac5
test ac5_doctor_prints_the_endpoint_in_the_runtime_dir ... ok
---- ac5_no_std_os_unix_net_in_cli_or_daemon_sources stdout ----
4 lines:
crates/cli/src/client.rs:5: use std::os::unix::net::UnixStream;
crates/daemon/src/main.rs:19: use std::os::unix::net::{UnixListener, UnixStream};
crates/daemon/src/server.rs:8: use std::os::unix::net::{UnixListener, UnixStream};
crates/daemon/src/state.rs:10: use std::os::unix::net::UnixStream;
test result: FAILED. 0 passed; 1 failed

$ cargo test -j 2 -p kawaro-daemon --test single_instance
error[E0432]: unresolved import `kawaro_core::transport`
 --> crates/daemon/tests/single_instance.rs:8:20
  |
8 | use kawaro_core::transport::{Bound, Endpoint, Listener};
  |                    ^^^^^^^^^ could not find `transport` in `kawaro_core`
error: could not compile `kawaro-daemon` (test "single_instance") due to 1 previous error
```
- 2026-10-22, implementer (S-040a): the module `crates/core/src/transport.rs` (`Bound`, `pipe_name` and per-platform dispatch), `transport/unix.rs` (the socket with a wrapper that turns `WouldBlock` into `TimedOut` in `Read`, and `bind` sends `ping` with a 500 ms timeout as `already_running()` did in the daemon), and `transport/windows.rs`, a seed that returns `ErrorKind::Unsupported` and whose `endpoint()` has an empty name until S-040b. The client (`client.rs`), the daemon (`main.rs`, `server.rs` and `state.rs`) and `doctor` moved onto it. No test was modified.
  - Manager decision on the ambiguity: a listener that accepts and does not answer `ping` within 500 ms stays exactly as today: it is considered not running, so the file is removed and bound. Likewise a reply other than `ok`, or a failed write.
  - Removing the file when the daemon exits stays on `ipc::socket_path()` as before.
  - `C`: `cargo fmt --check` clean, and `cargo clippy -j 2 --workspace --all-targets -- -D warnings` with no warning (including `transport.rs` and `single_instance.rs` in the tests).
  - `W`: `cargo clippy --target x86_64-pc-windows-gnu -p kawaro-core --lib -- -D warnings` with no warning.
  - `L`: `cargo test -j 2 -p kawaro-core -p kawaro-cli -p kawaro-daemon`: 210 passed, 0 failed (of which 14 in `transport.rs`, 1 each in `transport_seed.rs`, `no_unix_net.rs` and `single_instance.rs`, and 32 in `cli.rs`).
  - AC5: `grep -rn 'std::os::unix::net' crates/cli/src crates/daemon/src` returns 0 lines. `git diff --stat main...HEAD -- crates/ui` is empty.
  - `M` on the diff from `20502bc`: 26 mutants, 14 caught, 8 unviable, 2 timeouts (`read` and `write` with `Ok(1)`, an endless loop caught by the timeout), and 2 survivors excluded in `exclude_re` with the reason: `replace serve with ()` (the accept loop in the live daemon, covered by `tests/e2e/run.sh` like the `handle` precedent), and `flush -> Ok(())` in `unix.rs` (an equivalent mutant: `flush` for the socket in std does nothing). After the exclusion `cargo mutants --list` lists 24 mutants, and no known survivor is among them.
  - `scripts/design-lint.sh`: clean. CHANGELOG line under `[Unreleased]`.
  - Follow-up: `tests/e2e/run.sh` on a release build from this branch (headless KWin, temporary HOME) passed fully: "e2e: all passed" and "e2e S-019 AC7: all passed", including `open` and `drop` starting the daemon, "single instance", "second instance leaves the socket alone", and "quit stops the daemon and removes the socket". And `cargo test -j 2 --workspace`: 581 passed, 0 failed.
  - About `M`: with the two timeouts `cargo mutants` exits with code 3, not 0. The two timeouts are `read` and `write` with `Ok(1)`; the tests catch them by spinning until the timeout, and catching them faster is not possible without modifying the tests.
- 2026-10-22, reviewer (S-040a): **PASS**. All gates were rerun inside the scope (`MemoryMax=6G`, `-j 2`) from the `windows/0.6.0` tree:
  - `C`: `cargo fmt --check` clean, and `cargo clippy -j 2 --workspace --all-targets -- -D warnings` ended with `Finished` with no warning.
  - `W`: `cargo clippy --target x86_64-pc-windows-gnu -p kawaro-core --lib -- -D warnings` after touching `lib.rs` to force the check: exit code 0, no warning.
  - `cargo test -j 2 --all`: 581 passed, 0 failed, 1 ignored. The seventeen AC1 to AC7 tests all pass, and each criterion has at least one test (AC5 through the type of `State::subscribers` and AC7 through `W`, both verified by the reviewer in the code).
  - `M`: `cargo mutants --jobs 1 --in-diff` on `git diff 20502bc..HEAD -- crates` with `CARGO_BUILD_JOBS=2`: 24 mutants, 14 caught, 8 unviable, 2 timeouts, **0 survivors** (`missed.txt` empty). The two timeouts `read` and `write` with `Ok(1)` at `unix.rs:103` and `unix.rs:109`: the tests spin until the timeout and fail, so they are not survivors, and exit code 3 is caused by the timeouts alone.
  - `scripts/design-lint.sh`: clean. No long dash in the `main...HEAD` diff. One CHANGELOG line for the story under `[Unreleased]` in English with Arabic text. `git diff --stat main...HEAD -- crates/ui` is empty. `Cargo.lock`: 671 `[[package]]` as in `main`, and no change in any `Cargo.toml`. `grep -rn 'std::os::unix::net' crates/cli/src crates/daemon/src`: 0 lines.
  - The tests were not modified after the tester: `git diff 20502bc..HEAD -- crates/*/tests` is empty.
  - Screenshots: no UI change, so no new screenshot. Performance budget: does not touch the hive show path; the only new cost is one `dup` per request in `handle`.
  - Accepted deviations: (1) `replace serve with ()` in `exclude_re` on the `handle` precedent; the reviewer did not rerun `tests/e2e/run.sh`, so the acceptance rests on the precedent and the implementer's log. (2) `flush -> Ok(())` is equivalent: `flush` for a Unix socket in std returns `Ok(())` (the std source is not installed locally to quote from). (3) `endpoint()`, `server_pid()` and `Display` in the Windows seed cannot return `Unsupported` with the AC1 signatures, so the empty name and `None` are forced; every function that returns `io::Result` returns `Unsupported`. (4) `handle()` reads through `try_clone` because `Read` is not implemented for `&Stream`; behaviour on Linux is as before, and a failed clone closes the connection with no reply.
  - Small notes for later: `crates/daemon/src/main.rs:84` still removes `socket_path()` directly, not through the transport (for S-040b or S-045). And `flush` in `exclude_re` matches any `<impl Write for Stream>` in the repository; today there is only `unix.rs` and the excluded `windows.rs`.
  - No new CHANGELOG line from the reviewer: the single line for the whole story was added by S-040a, and S-040b edits it.
- 2026-10-22, tester (S-040b): tests for AC8 to AC14 written in new files so that an unresolved import does not break the fourteen S-040a tests in `transport.rs`. The contract the tester fixed at the top of `crates/core/tests/transport_plan.rs` and `deadline.rs` (not to be modified), all `pub` in `kawaro_core::transport` on all platforms:
  - `dacl_sddl(&str) -> String`.
  - `enum BindOutcome { Listen, AlreadyRunning, Failed(io::Error) }` and `bind_result(Option<u32>) -> BindOutcome`: `None` is listening, `Some(5)` is `AlreadyRunning`, and anything else is `Failed` with its code in `raw_os_error()`.
  - `enum ConnectStep { Connected, Wait(Duration), Fail(io::Error) }` and `connect_plan(open_error: Option<u32>, elapsed: Duration) -> ConnectStep`, one step of the connection loop with time as an input and no real sleep: `None` is connected, `Some(2)` is `NotFound` at any time without waiting, `Some(231)` before 2000 ms is `Wait(2000 ms - elapsed)`, which is the `WaitNamedPipeW` timeout, and at 2000 ms or later is `TimedOut` (the boundary: 2000 itself is an expired timeout), and any other code is `Fail` with its code.
  - `verify_owner<S>(server_sid: &str, client_sid: &str, stream: S) -> io::Result<S>`: returns the stream if the SIDs match, otherwise `PermissionDenied` and the stream is dropped without writing. This is the shape of "the result of the owner check" in AC11, with a write counter of 0 and a drop counter of 1 in the test.
  - `open_mode(bool) -> u32` and `pipe_mode() -> u32`.
  - `pub mod deadline` and `read_with_deadline<T, R, C>(timeout: Duration, read: R, cancel: C) -> io::Result<T>` where `R: FnOnce() -> io::Result<T>` runs on the calling thread and `C: FnOnce() + Send + 'static` is owned by the watchdog. If the timeout expires, `cancel` is called once and the result is `TimedOut` whatever the cancelled read returned; if the read returns first, its result is returned as is and `cancel` is never called, even later; and on return no watchdog is alive (the thread counter: an `Arc` inside `cancel` whose `strong_count` returns to 1).
  - Files and tests:
    - `crates/core/tests/transport_plan.rs`: `ac8_dacl_sddl_is_the_exact_literal`, `ac8_dacl_sddl_grants_no_one_else`, `ac8_dacl_sddl_embeds_the_given_sid` (the longest SID, so that a constant value does not survive `M`), `ac9_bind_result_none_listens`, `ac9_bind_result_access_denied_is_already_running`, `ac9_bind_result_other_codes_fail_with_the_same_code` (2, 231, 1, 87 and `0xFFFFFFF0`), `ac11_connect_plan_no_error_is_connected`, `ac11_connect_plan_file_not_found_is_not_found_at_once`, `ac11_connect_plan_pipe_busy_waits_for_the_rest_of_2000_ms` (0, 1, 750 and 1999 ms), `ac11_connect_plan_pipe_busy_times_out_at_2000_ms` (2000, 2001 and 10000), `ac11_connect_plan_other_codes_fail_with_the_same_code`, `ac11_another_users_server_is_permission_denied_and_gets_no_byte`, `ac11_own_server_hands_the_stream_back_for_writing` (kills the "always reject" mutant), `ac12_open_mode_first_instance_is_duplex_and_first_pipe_instance`, `ac12_open_mode_later_instances_are_duplex_only`, `ac12_pipe_mode_is_byte_wait_and_rejects_remote_clients`.
    - `crates/core/tests/deadline.rs`: `ac10_blocked_read_times_out_between_180_and_1000_ms_with_one_cancel` (the fake read blocks with a 3 s `recv_timeout` so that a mutant that does not cancel does not hang `M`, and after cancellation returns an error that is not `TimedOut`, so the test proves the conversion), `ac10_read_done_within_50_ms_returns_its_data_and_never_cancels` (returns before 150 ms, and `cancel` is not called up to 400 ms after the return), `ac10_a_read_error_before_the_deadline_is_returned_as_is`.
    - `crates/core/tests/transport_seed.rs` (modified by the tester): the `ErrorKind::Unsupported` check was removed because S-040b replaces the seed with the real pipe and the implementer may not modify the test; the `exclude_globs` check and the `todo!` ban remain under the name `ac7_windows_transport_is_excluded_from_mutants_and_has_no_todo`. Text checks on `windows.rs` required by AC8, AC10 and AC12 were added: `ac8_windows_builds_the_descriptor_from_dacl_sddl`, `ac10_windows_cancels_the_read_with_cancel_io_ex`, `ac9_ac11_ac12_windows_uses_the_pure_decisions` (calls `open_mode(`, `pipe_mode(`, `bind_result(`, `connect_plan(`, `verify_owner(` and `WaitNamedPipeW`), `ac12_windows_asserts_every_flag_against_windows_sys_at_compile_time` (`const _: () = assert!` and the names of the nine constants), `ac12_windows_checks_the_owner_and_names_the_endpoint_from_the_token`, and `ac10_deadline_rs_is_mutated_on_linux` (the file exists and is not in `exclude_globs`).
    - `crates/core/tests/transport_deps.rs`: `ac13_cargo_lock_keeps_671_packages`, `ac13_core_depends_on_windows_sys_0_61_2_in_the_lock` (the `kawaro-core` entry in `Cargo.lock` lists `"windows-sys 0.61.2"`), `ac13_core_declares_windows_sys_for_windows_only` (in `[target.'cfg(windows)'.dependencies]` of `crates/core/Cargo.toml` with version 0.61, and nowhere else), `ac14_engines_md_records_windows_sys_and_its_license` (the name, the version and `MIT OR Apache-2.0` within eight lines).
  - Passes before implementation on purpose: `ac13_cargo_lock_keeps_671_packages` (a regression guard; `windows-sys 0.61.2` is in the lock already), and `ac7_windows_transport_is_excluded_from_mutants_and_has_no_todo` (a guard from S-040a).
  - For the reviewer, not for tests: `X` and `cargo tree` on Linux, `W` with `-p kawaro-engines`, the text of `SECURITY.md` in Arabic and English and `docs/architecture.md`, that every `CreateNamedPipeW` takes the descriptor (the text check proves presence, not every call), and the edit of the same CHANGELOG line.
  - Check of the tests' logic: a temporary copy outside the repository (a crate in the session's temporary directory, deleted and not merged) with a short implementation of the contract passed the sixteen `transport_plan.rs` tests and the three `deadline.rs` tests, and `cargo clippy --tests -D warnings` is clean on it and on `transport_seed` and `transport_deps` here. `cargo fmt --check` is clean, and the S-040a tests in `transport.rs` still pass, 14 of them.
  - Ambiguity: (1) AC11 names `connect_plan(results)` in the plural; it was fixed as a single step with time as an input so that the test does not sleep 2 s under `M`, and the loop itself is in `windows.rs`. (2) The "wait, then retry" case when `WaitNamedPipeW` returns success early and the pipe then stays busy: the elapsed time is measured by the clock in the loop, so it was not tested outside the function. (3) Codes other than 2 and 231 on connect were not specified by AC11; the tester fixed them as in AC9: the same code in `raw_os_error()`.
  - Failure output (`cargo test -j 2 -p kawaro-core` inside the scope):
```
$ cargo test -j 2 -p kawaro-core --test transport_plan
error[E0432]: unresolved imports `kawaro_core::transport::BindOutcome`, `kawaro_core::transport::ConnectStep`, `kawaro_core::transport::bind_result`, `kawaro_core::transport::connect_plan`, `kawaro_core::transport::dacl_sddl`, `kawaro_core::transport::open_mode`, `kawaro_core::transport::pipe_mode`, `kawaro_core::transport::verify_owner`
error: could not compile `kawaro-core` (test "transport_plan") due to 1 previous error

$ cargo test -j 2 -p kawaro-core --test deadline
error[E0432]: unresolved import `kawaro_core::transport::deadline`
  --> crates/core/tests/deadline.rs:18:31
   |
18 | use kawaro_core::transport::deadline::read_with_deadline;
   |                               ^^^^^^^^ could not find `deadline` in `transport`
error: could not compile `kawaro-core` (test "deadline") due to 1 previous error

$ cargo test -j 2 -p kawaro-core --test transport_seed
test ac7_windows_transport_is_excluded_from_mutants_and_has_no_todo ... ok
---- ac10_deadline_rs_is_mutated_on_linux ----
.../crates/core/src/transport/deadline.rs must exist
---- ac10_windows_cancels_the_read_with_cancel_io_ex ----
windows.rs lacks ["read_with_deadline(", "CancelIoEx"]: the read timeout must cancel the pipe read (AC10)
---- ac12_windows_asserts_every_flag_against_windows_sys_at_compile_time ----
compile time asserts are required (AC12)
---- ac12_windows_checks_the_owner_and_names_the_endpoint_from_the_token ----
windows.rs lacks ["GetNamedPipeServerProcessId", "ProcessIdToSessionId", "GetTokenInformation", "TokenUser", "pipe_name("]: owner check and endpoint come from the process token (AC12)
---- ac8_windows_builds_the_descriptor_from_dacl_sddl ----
windows.rs lacks ["dacl_sddl(", "ConvertStringSecurityDescriptorToSecurityDescriptorW", "CreateNamedPipeW"]: the pipe must be created with the DACL from dacl_sddl (AC8)
---- ac9_ac11_ac12_windows_uses_the_pure_decisions ----
windows.rs lacks ["open_mode(", "pipe_mode(", "bind_result(", "connect_plan(", "verify_owner(", "WaitNamedPipeW"]: windows.rs must call the tested pure functions (AC9, AC11, AC12)
test result: FAILED. 1 passed; 6 failed

$ cargo test -j 2 -p kawaro-core --test transport_deps
test ac13_cargo_lock_keeps_671_packages ... ok
---- ac13_core_declares_windows_sys_for_windows_only ----
crates/core/Cargo.toml needs [target.'cfg(windows)'.dependencies]
---- ac14_engines_md_records_windows_sys_and_its_license ----
docs/engines.md must record windows-sys 0.61.2 (MIT OR Apache-2.0)
---- ac13_core_depends_on_windows_sys_0_61_2_in_the_lock ----
kawaro-core must use windows-sys 0.61.2:
name = "kawaro-core" ... dependencies = [ "serde", "serde_json", "tempfile", "thiserror 2.0.21", "toml 0.8.23" ]
test result: FAILED. 1 passed; 3 failed
```
- 2026-10-22, implementer (S-040b): no test was modified (`git diff 80345be -- crates/*/tests` is empty). The manager accepted the tester's six choices as they are.
  - `transport.rs`: `dacl_sddl`, `BindOutcome`/`bind_result`, `ConnectStep`/`connect_plan`, `verify_owner` and `open_mode`/`pipe_mode`, with the values as private literal constants (`OPEN_MODE_FIRST`, `OPEN_MODE_LATER`, `PIPE_MODE` and the three error codes) so that `|` does not produce equivalent mutants; `windows.rs` checks them with `const _: () = assert!` against `windows_sys`. Code 2 has no arm in `connect_plan`: it passes through as a raw code, and std reads it as `NotFound` on Windows (and on Linux, because 2 is `ENOENT`), so there is no arm whose deletion would yield a surviving mutant; its constant `ERROR_FILE_NOT_FOUND` carries `cfg_attr(not(windows), allow(dead_code))` and is used in the compile-time check.
  - `transport/deadline.rs`: a watchdog thread with `recv_timeout` and one lock that decides who came first (the cancellation or the read), then `join` before returning, so `cancel` is never called after the read returns and no watchdog stays alive.
  - `transport/windows.rs`: a single place for `CreateNamedPipeW` (`create_instance`) that takes the descriptor built from `dacl_sddl` with `ConvertStringSecurityDescriptorToSecurityDescriptorW`, so every instance gets it; the descriptor lives with `Listener` and is freed with `LocalFree`. `accept` creates the next instance before handing over the connection, so the client finds the pipe busy rather than missing, and `ERROR_PIPE_CONNECTED` is success. The client opens with `OpenOptions` with `security_qos_flags(SECURITY_IDENTIFICATION)` and loops on `connect_plan` and `WaitNamedPipeW` (the timeout at least 1 ms, because 0 means the default timeout), then `GetNamedPipeServerProcessId`, `OpenProcess`, `GetTokenInformation(TokenUser)` and `verify_owner`; any failure to determine the owner gives an empty SID, so it is rejected (fails closed). The timeout is shared across `try_clone` as on Unix, and the read with a timeout is `read_with_deadline` with `CancelIoEx` on the same read handle. No `DisconnectNamedPipe` on close, so that an unread reply is not lost. `endpoint()` comes from `ProcessIdToSessionId` and the token SID, and `server_pid()` returns the server pid on the client stream.
  - `crates/core/Cargo.toml`: `windows-sys = { version = "0.61", features = [...] }` under `[target.'cfg(windows)'.dependencies]` only. `Cargo.lock`: one line in the `kawaro-core` entry (`"windows-sys 0.61.2"`), and 671 `[[package]]`.
  - `C`: `cargo fmt --check` clean, and `cargo clippy -j 2 --workspace --all-targets -- -D warnings` with no warning.
  - `W`: `cargo clippy -j 2 --target x86_64-pc-windows-gnu -p kawaro-core -p kawaro-engines --lib -- -D warnings` with no warning, and `cargo check` with the same flags with no warning.
  - `X`: lists serde, serde_json, thiserror, toml and `windows-sys v0.61.2`; and `cargo tree -p kawaro-core -e normal --depth 1` on Linux does not list it.
  - `cargo test -j 2 --all`: 610 passed, 0 failed, 1 ignored (581 before, and 29 new: 16 in `transport_plan.rs`, 3 in `deadline.rs`, 7 in `transport_seed.rs` where there was 1, and 4 in `transport_deps.rs`).
  - `M`: `cargo mutants --jobs 1 --in-diff` on `git diff 80345be -- crates` with `CARGO_BUILD_JOBS=2`: 21 mutants, 15 caught, 6 unviable, 0 timeouts, **0 survivors**, exit code 0. Nothing added to `exclude_re`; only the `exclude_globs` comment for `windows.rs` was edited.
  - `scripts/design-lint.sh`: clean. No long dash in the diff. `git diff --stat main...HEAD -- crates/ui` is empty.
  - Documents: `SECURITY.md` (a Windows paragraph in English and an Arabic paragraph), `docs/architecture.md` (the single instance on both platforms), and `docs/engines.md` (a windows-sys 0.61.2 line, MIT OR Apache-2.0 from the crate's own `Cargo.toml`). The same CHANGELOG line was edited and no other was added.
  - Not verified live, and not possible on Linux: that `CancelIoEx` from another thread cancels a synchronous `ReadFile` on the handle (relying on ADR 0005), and all S-052 items.
  - For S-045: `crates/daemon/src/main.rs` still removes `ipc::socket_path()` directly on exit; left because the S-040b criteria do not require it and the daemon is not built for Windows before S-045.
- 2026-10-22, reviewer (S-040b): **PASS**. All gates were rerun inside the scope (`MemoryMax=6G`, `-j 2`) from the `windows/0.6.0` tree at `78fe2c5`:
  - `cargo fmt --check` clean. `cargo clippy -j 2 --workspace --all-targets -- -D warnings` with no warning. `cargo test -j 2 --all`: 610 passed, 0 failed, 1 ignored.
  - `W` (after `touch` to force a recheck, with the separate `CARGO_TARGET_DIR`): `cargo check` and `cargo clippy --target x86_64-pc-windows-gnu -p kawaro-core -p kawaro-engines --lib -- -D warnings` with no warning.
  - `X`: Windows lists serde, serde_json, thiserror, toml and `windows-sys v0.61.2`; Linux does not list it. `Cargo.lock` has 671 `[[package]]`.
  - `M`: `cargo mutants --jobs 1 --in-diff` on `git diff 80345be -- crates` with `CARGO_BUILD_JOBS=2`: 21 mutants, 15 caught, 6 unviable, 0 timeouts, **0 survivors**; matches the implementer's log.
  - `scripts/design-lint.sh` clean. No long dash in `git diff da12857..HEAD` (with a positive control check). `git diff --stat main...HEAD -- crates/ui` is empty. The tests have not changed since `80345be`: note, the path `crates/*/tests` gives empty output even on the tester's own diff, so the correct check is `crates/*/tests/*`, which is empty on `80345be..HEAD` and lists the four files on `da12857..80345be`.
  - Every criterion is tied to a test: AC8, AC9, AC11 and AC12 in `transport_plan.rs` and the text checks in `transport_seed.rs`, AC10 in `deadline.rs`, and AC13 and AC14 in `transport_deps.rs`; the rest of AC13 and AC14 through the commands above and reading the documents.
  - Security review of `crates/core/src/transport/windows.rs`: the descriptor `D:P(A;;GA;;;<SID>)` is protected with a single entry for the user, so no one else can open the pipe or add instances to it. `CreateNamedPipeW` is called in one place (`create_instance`, line 137) with the descriptor, `open_mode(true)` in `bind` only (170) and `false` in `accept` (190). `PIPE_REJECT_REMOTE_CLIENTS` is in `pipe_mode`, and every constant is checked at compile time (55 to 63). The owner check fails closed: a failure of `GetNamedPipeServerProcessId`, `OpenProcess` or reading the token gives an empty SID, and a failure of `current_sid` returns an error, not an empty string. All handles are `OwnedHandle`, the descriptor and the SID string are freed with `LocalFree`, the `TOKEN_USER` buffer is aligned with u64 cells, and the watchdog is joined before the raw handle is closed. The client opens with `security_qos_flags(SECURITY_IDENTIFICATION)` (std adds `SECURITY_SQOS_PRESENT`; from knowledge of the std implementation, its source is not installed locally), and `bInheritHandle` is zero.
  - The deviation in `connect_plan` is accepted: an explicit arm for code 2 would give the same `io::Error`, so deleting it is an equivalent mutant. `ac11_connect_plan_file_not_found_is_not_found_at_once` proves `NotFound` and no `Wait` at 0, 1, 1999, 2000 and 5000 ms, and std reads 2 as `NotFound` on both platforms.
  - CHANGELOG: the S-040 line itself was edited and no other was added, so there is no new line from the reviewer (settled in the S-040a review: one line for the whole story). `SECURITY.md` in English and Arabic, `docs/architecture.md` describes the single instance on both platforms, and `docs/engines.md` records windows-sys 0.61.2 (MIT OR Apache-2.0).
  - Non-blocking notes for later:
    - Low, `windows.rs:190-191` with `crates/daemon/src/server.rs:14-16`: if `create_instance` fails in `accept`, the connected handle stays in `next`, so the next `ConnectNamedPipe` returns `ERROR_PIPE_CONNECTED` at once and creation fails again, and `serve` spins without stopping while the client hangs. For S-045 or S-052.
    - Low, `windows.rs:85`: the result of `ProcessIdToSessionId` is ignored, so a failure silently makes the session 0.
    - Info: another user can reserve the pipe name first (the SID is not secret), so the daemon reads `ERROR_ACCESS_DENIED` as `AlreadyRunning` and exits, and the client rightly rejects with `PermissionDenied`. This is what AC9 requires; for S-045: verify the owner before trusting `AlreadyRunning`.
    - Info: if the watchdog takes the lock at the moment the read returns with data, the data is discarded and `TimedOut` is returned; this is the tester's contract.
    - Carried risk: that `CancelIoEx` cancels a synchronous `ReadFile` on the pipe has not been verified live; deferred to S-052.
