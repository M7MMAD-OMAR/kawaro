# S-067 Fix what the cleanup review found in S-047, S-049 and S-054c
Status: approved (manager, by delegation, 2026-10-26)

A four way cleanup review (reuse, quality, efficiency, altitude) of the diff of S-054c, S-047 and S-049 on top of S-066 found one real bug in `kawaro update --apply`, two in the Windows drop glue, one in the update status, and code copied between the CLI and the daemon. Findings and evidence: the manager's checkpoint run, files `cleanup-reuse.md`, `cleanup-quality.md`, `cleanup-efficiency.md`, `cleanup-altitude.md`. One work day.

## Story
As a user of the command line updater and of the Windows hive, I want the review's bugs fixed and the copied update code shared, so that `kawaro update --apply` works with a repository, a drop never runs on files from an earlier drag, and one fix to the update probe reaches both the tray and the terminal.

## Acceptance criteria
- AC1 (bug: `--apply` with a repository): in `kawaro_core::update::apply`, a pure `pkexec_target(argv: &[String]) -> Option<&Path>` returns the program `pkexec` would run as root only for the privileged step shape (`argv[0] == PKEXEC`, `argv[2] == "update"`, `argv[3] == "--install-privileged"`), giving `Some(argv[1])`; for `repo_command(PKEXEC, PkgKind::Rpm)` and `repo_command(PKEXEC, PkgKind::Deb)` it gives `None`, and for any argv not starting with `PKEXEC` it gives `None`. One `SystemRunner` in core implements `CommandRunner`, checks `exe_is_trusted` only when `pkexec_target` is `Some`, and is used by both `crates/cli/src/update_sys.rs` and `crates/daemon/src/update_sys.rs`; neither of those files defines its own `impl CommandRunner`.
- AC2 (one probe): `kawaro_core::update::apply` has `is_repo_file(name: &str) -> bool` (true for a name containing `kawaro` and ending in `.repo`, case insensitive, and for exactly `kawaro.sources`; false for `kawaro.repo.bak`, `other.repo`, `kawaro.list`), `clean_exe_path` (moved from the daemon, same behaviour and tests) and one live `probe_facts()` that the CLI and the daemon both call. A text test over `crates/cli/src` and `crates/daemon/src` finds no `fn repo_configured`, no `fn clean_exe_path`, no `"/etc/yum.repos.d"` literal and no `" (deleted)"` literal.
- AC3 (bug: ready version): `kawaro_core::update::flow` has `part_path(final_path: &Path) -> PathBuf` used by `download_verified`, and `ready_on_disk(cache_dir: &Path, current: &Version) -> Option<Version>` that returns the HIGHEST version newer than `current` whose folder under `updates/` holds a finished package (a file without the `.part` suffix). With folders `0.5.0` and `0.6.0` both finished it gives `0.6.0`, whatever the directory order; with only `0.6.0/x.rpm.part` it gives `None`; with an older version it gives `None`. `kawaro update --status` uses it, and no `".part"` literal remains in `crates/cli/src`.
- AC4 (bug: stale drag files, Windows): `DropTracker` owns the files of a drag: `on_hover_file(path)` appends one path, `is_active()` is true from the first hover until a drop, a cancel or `on_close()`, and `on_close()` clears everything. Hover `a`, `on_close()`, hover `b`, then a drop over cell 3 gives `Run { cell: 3, files: [b] }`. `crates/ui/src/dnd/windows.rs` has no `static` file list of its own (no `HOVERED`), calls `on_hover_file` per winit `HoveredFile`, and stops the poll timer when the hive closes (a text test finds `.stop()` reachable from the close path, and `is_active` decides whether a drag is in progress rather than the timer state). The existing S-047 tests still pass unchanged.
- AC5 (bug: swapped mouse buttons, and the line anchored exclusion): a pure `primary_button_vk(swapped: bool) -> i32` returns `0x01` (VK_LBUTTON) for `false` and `0x02` (VK_RBUTTON) for `true`; the Windows probe reads `GetSystemMetrics(SM_SWAPBUTTON)` and lives in `crates/daemon/src/shell.rs`. `crates/daemon/src/server.rs` keeps only a pure `drop_refused(is_drop: bool, primary_down: bool) -> bool` (true only for `true, false`), mutated and unit tested with the four combinations, and has no `GetAsyncKeyState`. `.cargo/mutants.toml` has no `server.rs:106` entry and no `shortcut_drop_allowed` entry.
- AC6 (one open folder path): one daemon function opens the folder (from `open_folder_command`), logs a spawn failure as one stderr line, and does not leave a zombie (the child is waited on a thread). A text test finds `.spawn()` for the open folder command exactly once in `crates/daemon/src`. `notify_update_with` writes its line through `notify_logic::log_line`; the S-054b test that looked for the literal prefix in `notify.rs` is updated by the tester to look for `log_line(` instead, and the literal `kawarod: notify:` appears only in `notify_logic.rs`.
- AC7 (no silent CLI failures): with a `config.toml` that does not parse, `kawaro update --status` prints the config warning as one stderr line and still answers `status none`, exit 0. When the cache folder cannot be created, a check prints one stderr line naming the folder and exits 5.
- AC8 (small cleanups, each checked by text): `notify_logic::LOG_PREFIX` is private; the `TICK_MS` comment names `slint::Timer`, not `SetTimer`; the CLI's stable and beta name table is replaced by a core `Channel::parse` that `Config` also uses; `"--install-privileged"` is one exported core constant used by `privileged_argv` and the CLI dispatch, and the dispatch also accepts the `--install-privileged=FILE` form.
- AC9 (gates): fmt, clippy `-D warnings`, `cargo test --workspace`, `scripts/windows/check.sh` exit 0, `cargo mutants --in-place` on the diff with no surviving mutant in changed code that is not excluded with a written reason, design lint clean, one CHANGELOG line in English and Arabic.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | `pkexec_target` for the privileged argv, both repo commands, and a non pkexec argv; a text test that `impl CommandRunner` appears in core only (outside tests) | unit |
| AC1 | The CLI `--apply` path with a fake facts set where `repo_configured` and `has_pkexec` are true and a fake runner records the argv: the runner receives `pkexec dnf ...` and the exit is 0 | integration |
| AC2 | `is_repo_file` with the six names above; `clean_exe_path` cases moved with it; the text test over both crates | unit |
| AC3 | Temp cache dirs: 0.5.0 and 0.6.0 finished, created in both orders, give 0.6.0; only `.part` gives None; older gives None; the CLI `--status` prints `status ready 0.6.0` | unit, integration |
| AC4 | Tracker: hover a, close, hover b, drop on cell 3; `is_active` across hover, drop, cancel, close; the text test over `dnd/windows.rs` | unit |
| AC5 | `primary_button_vk` both values; `drop_refused` four combinations; text tests over `server.rs` and `mutants.toml` | unit |
| AC6 | The text test for one `.spawn()` and the prefix; the updated S-054b test | unit |
| AC7 | The real binary in a temp HOME with a broken `config.toml`; a fake facts set with an unwritable cache dir | integration |
| AC8 | Text tests for each item; `--install-privileged=FILE` is routed to the privileged step (exit 77 as a normal user) | unit, integration |

## Out of scope
- One shared status enum for the CLI and the daemon over IPC, the daemon restoring a ready package from disk at start, and a lock between a CLI download and a daemon download (S-068).
- The Explorer `/select,` quoting: checked live in S-052 before any change, since the S-049 test pins today's argument.
- `Plan::pick` allocating on every 16 ms tick (measured first, then a separate story if it matters).

## Review log
- 2026-10-26, planner (manager): written from the four cleanup reports; bug B1 (`--apply` with a repository) was confirmed by reading `linux_plan`, which builds `pkexec dnf ...`, and the CLI runner, which stats `argv[1]` (`dnf`) and refuses.
