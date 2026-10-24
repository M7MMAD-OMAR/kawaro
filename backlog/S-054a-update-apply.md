# S-054a Apply a verified update on Linux and Windows
Status: review

First surface story of ADR 0006. Depends on S-053b. The Windows part needs P3 (S-045 to S-051, a per-user MSI with a stable `UpgradeCode` that relaunches the daemon); until then only its pure parts are built and tested on Linux, and its live run is an open item. Windows only files are in `exclude_globs` and are built by `scripts/windows/check.sh`. About one work day.

## Story
As a user, I want one action to install the verified update and bring the app back, so that I never download or install anything by hand.

## Acceptance criteria
- AC1 (`crates/core/src/update/apply.rs`): `msiexec_args(msi, log) -> Vec<String>` is exactly `/i <msi> /qn /norestart /l*v <log>`. `classify_msi(code: i32) -> MsiResult`: 0 `Installed`, 3010 `InstalledRebootPending`, 1602 `Cancelled`, 1618 `Busy`, 1603 and any other code `Failed`.
- AC2 (`apply.rs`): `linux_plan(&Facts) -> Plan` where `Facts` holds `has_dnf`, `has_apt`, `has_pkexec`, `repo_configured` (a file matching `*sbarhive*.repo` or `sbarhive.sources`) and `installed_kind`. Table: repo and dnf and pkexec gives `RepoUpgrade("pkexec dnf upgrade -y sbarhive")`; repo and apt gives `RepoUpgrade("pkexec apt-get install -y --only-upgrade sbarhive")`; dnf only gives `Local(Rpm)`; apt only gives `Local(Deb)`; no pkexec gives `ShowCommand` with the exact command for the manager found; neither manager gives `ShowRelease` with the release page URL.
- AC3 (`apply.rs`): `install_privileged(file, sha256, kind, root_dir, programs) -> Result<(), UpdateError>` copies `file` to a new directory under `root_dir` with mode 0700, hashes the copy, and on a match runs `<dnf> install -y <copy>` or `<apt-get> install -y <copy>` with the copy's path, never the original. On a mismatch it runs nothing and deletes the copy. If `file` is replaced between the caller's check and the call, the result is `Err(HashMismatch)` and the fake manager has 0 invocations.
- AC4 (`apply.rs`): `auto_action(mode, os) -> AutoAction`: `(Auto, Windows)` is `ApplyWhenIdle`; `(Auto, Linux)` and `(Ask, Linux)` are `NotifyReady`; `(Ask, Windows)` is `NotifyReady`. No path in `apply.rs` runs `pkexec` except through an explicit `apply_now` call.
- AC5 (`apply.rs`): `can_apply(running_jobs: usize, hive_open: bool) -> bool` is true only for `(0, false)`; the retry interval constant is 30 s.
- AC6 (`update/result.rs`): `UpdateResult { version, outcome: Installed | Failed(MsiResult or text), at }` writes `update-result.json` atomically (temp file, then rename) and reads back equal; an unreadable file reads as `None`; `take_result()` deletes the file after reading so a result is reported once.
- AC7 (`apply/windows.rs`, excluded from mutants): `apply_msi` copies `sbarhive.exe` to a temp folder, starts the copy detached with `update --apply-msi <msi>`, and the copy waits for the daemon to exit, runs `msiexec` by absolute path, writes the result, and starts the new daemon on success or the old one on any other result. Verified by `scripts/windows/check.sh` building and linting it, and by a manual run in the Windows machine when P3 is done.
- AC8 (`sbarhive update --install-privileged <file> --sha256 <hex> --kind <rpm|deb>`): refuses to run unless the effective user is root (exit 77 and nothing copied), and otherwise calls `install_privileged` with `/var/tmp` as `root_dir`.
- AC9 (gates): fmt, clippy, tests, `design-lint.sh`, mutants clean on changed non Windows files, `check.sh` exits 0, one CHANGELOG line, English and Arabic.

## Test cases
Fake `dnf`, `apt-get` and `pkexec` are shell scripts in a temp dir that append their argv to a file, so invocation counts and arguments are asserted.
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | The vector, and the six codes | unit |
| AC2 | One row per case, comparing the `Plan` value and the command text | unit |
| AC3 | Good file: argv recorded is `install -y <copy>` with the copy under `root_dir`, bytes equal, mode 0700. Tampered or swapped file: 0 invocations, copy absent | integration |
| AC4, AC5 | The tables | unit |
| AC6 | Write, read, take twice (second gives `None`), truncated file gives `None` | unit |
| AC7 | `check.sh` output | build |
| AC8 | Run as a non root user: exit code 77 and an empty `root_dir` | integration |

## Out of scope
- A polkit policy file (generic `pkexec` text for now).
- Any other package manager (zypper, pacman) and Flatpak.
- Rolling back after a successful install.

## CHANGELOG plan
- Install updates (S-054a): a verified update installs with one action: a quiet per-user installer on Windows, `pkexec` with `dnf` or `apt` on Linux. «تثبيت التحديثات: بإجراء واحد، تثبيت صامت لكل مستخدم في ويندوز، وpkexec مع dnf أو apt في لينكس».

## Review log
- 2026-10-24, planner: written from ADR 0006; approved by the manager under the maintainer's delegation. The privileged copy and re-hash closes the gap between checking a user-writable file and root reading it.
- 2026-10-24, tester: tests in `crates/core/tests/s054a_apply.rs` (pure: AC1, AC2, AC4, AC5, AC6, AC7 helper and source checks, AC8 root gate), `crates/core/tests/s054a_apply_privileged.rs` (AC3, fake dnf and apt-get scripts, unix only) and `crates/cli/tests/s054a_install_privileged.rs` (AC8, unix only). Failure output before implementation:
  - core: `error[E0432]: unresolved import sbarhive_core::update::apply` and `sbarhive_core::update::result` (both core test binaries do not compile; the modules do not exist).
  - cli: `ac8_a_non_root_user_gets_exit_77_and_nothing_is_copied` and `ac8_the_refusal_comes_before_the_file_is_looked_at` FAILED: `left: Some(2) right: Some(77)`, stderr `error: unrecognized subcommand 'update'`.
  - With throwaway `todo!()` stubs (removed) all 38 + 10 core tests panic or fail on the assertion, clippy `-D warnings` is clean on the three files, and `scripts/windows/check.sh` compiles the portable file.
  - Decisions are listed at the top of `s054a_apply.rs` and `s054a_apply_privileged.rs` (installed_kind picks the manager when both exist, ShowCommand text, copy folder `sbarhive-update-*` removed on every path, JSON shape of `update-result.json`, `apply_now` and `HelperOps` seams).
- 2026-10-24, implementer: `update/apply.rs`, `update/result.rs`, `update/apply/windows.rs` (excluded from mutants), the CLI `update --install-privileged` stub, `.cargo/mutants.toml`, CHANGELOG. No new dependency: the effective user id is read from `/proc/self/status` (pure parser `effective_uid`, an unreadable file counts as not root). `Programs::system()` uses `/usr/bin/dnf` and `/usr/bin/apt-get`. Extra small tests in `crates/core/tests/s054a_extra.rs`. Deviations: `DAEMON_EXIT_TIMEOUT` is 60 s; the Windows `apply_msi` does not ask the daemon to quit (S-054b does, then the helper waits on the pipe); `helper_main` takes the version from the package folder name and the program folder from the `SBARHIVE_PROGRAM_DIR` variable set by `apply_msi`; the `update --apply-msi` flag is not wired in the CLI (S-054c).
