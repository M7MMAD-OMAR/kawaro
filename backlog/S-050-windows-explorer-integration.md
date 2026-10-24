# S-050 Explorer right click menu: registry verb and `sbarhive integration install` and `uninstall`
Status: done

Builds on ADR 0005 section 6 and on S-048 (the icon) and S-049 (`APP_ID`). For the Windows beta. The command lines are built by a pure module in core and run through `reg.exe`, so the logic is tested on Linux and the Windows code is a thin runner. Half a work day.

## Story
As a Windows user who unpacked the zip file, I want one command to add SbarHive to the Explorer right click menu and one to remove it, with no administrator rights, so that I can try the beta and undo it cleanly.

## Files
`crates/core/src/registry.rs`, `crates/cli/src/{integration.rs,main.rs}`, `integrations/windows/registry/README.md` (English), `.cargo/mutants.toml`, `CHANGELOG.md`.

## Acceptance criteria
- AC1: `registry::install_commands(dir: &str, lang: Lang) -> Vec<Vec<String>>` is pure. For `dir = C:\Users\u\AppData\Local\Programs\SbarHive` it returns `reg.exe add` argument lists (`["add", KEY, "/v", NAME, "/t", "REG_SZ", "/d", DATA, "/f"]`, and `["add", KEY, "/ve", "/t", "REG_SZ", "/d", DATA, "/f"]` for the default value) for exactly these entries:
  - `HKCU\Software\Classes\*\shell\SbarHive`: `MUIVerb` = `SbarHive` (English) or `صبار هايف` (Arabic), `Icon` = `<dir>\sbarhive.ico`, `MultiSelectModel` = `Player`.
  - `HKCU\Software\Classes\*\shell\SbarHive\command`, default value = `"<dir>\sbarhived.exe" open "%1"` (the quotes are part of the value).
  - `HKCU\Software\Classes\AppUserModelId\io.github.m7mmad_omar.SbarHive`: `DisplayName` = `SbarHive`, `IconUri` = `<dir>\sbarhive.ico`.
  The list has exactly 6 commands, in that order.
- AC2: `registry::uninstall_commands() -> Vec<Vec<String>>` returns exactly `["delete", "HKCU\\Software\\Classes\\*\\shell\\SbarHive", "/f"]` and `["delete", "HKCU\\Software\\Classes\\AppUserModelId\\io.github.m7mmad_omar.SbarHive", "/f"]`, and nothing else (no key outside these two trees).
- AC3: a `dir` containing a double quote makes `install_commands` return `Err("path contains a quote")`; a `dir` with a trailing backslash is normalised (the command value has no `\\` before `sbarhived.exe`).
- AC4: `sbarhive integration install --dry-run` prints each command as one line `reg.exe <args>` to stdout, exit 0, on any OS, with the directory taken from the executable's folder. Run from a test with `--dir /x` (a test-only override) the output has 6 lines, the 4th contains `open "%1"`.
- AC5: `sbarhive integration install` without `--dry-run` on a non-Windows OS exits 2 and prints `integration install: available on Windows only`. `uninstall --dry-run` prints 2 lines.
- AC6: on Windows, `install` first checks that `sbarhived.exe` and `sbarhive.ico` exist next to the executable; if one is missing it exits 1 naming the missing file and runs no `reg.exe` command (pure `missing_files(dir, exists) -> Vec<String>` with an injected `exists`). Then it runs the commands in order and stops at the first failure with that command's exit code in the message.
- AC7: `integrations/windows/registry/README.md` lists the keys and the rule that Windows 11 shows the item under "Show more options" (Shift+F10). `W` (`scripts/windows/check.sh`) exits 0; `C`, `L` clean with baseline plus new tests; `M` has no surviving mutant in `registry.rs` and `integration.rs` (the process runner file is excluded); one CHANGELOG line.

## Check commands
`W`, `L`, `C`, `M` as in S-045 (6G scope, `-j 2`).

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1, AC2, AC3 | Unit tests comparing the full vectors for both languages | unit |
| AC4, AC5 | Run the cli binary and compare stdout and exit codes | integration |
| AC6 | Unit test with `exists` closures returning false for each file in turn | unit |
| AC7 | Reviewer reads the README and CHANGELOG; `W` | review, build |

## Live check deferred to S-052
- After `install`, right click on one file and on ten files shows the item under "Show more options" and opens the hive; after `uninstall` the item is gone; the toast is named SbarHive.

## Out of scope
- Directory and background menus; the Windows 11 first-level menu (needs signing); autostart at logon (S-051 installer).

## Review log
- 2026-10-24, planner: `reg.exe` is used instead of registry API calls so every argument is checkable on Linux.
- 2026-10-24, tester: tests written from the ACs only, in `crates/core/tests/explorer_registry.rs` (AC1, AC2, AC3, AC6 `missing_files`) and `crates/cli/tests/integration.rs` (AC4, AC5). Assumed API: `sbarhive_core::registry::{install_commands, uninstall_commands, missing_files}`, `missing_files(dir: &str, exists: impl Fn(&str) -> bool) -> Vec<String>` returning file names in the order `sbarhived.exe`, `sbarhive.ico`, calling `exists` with `<dir>\<name>`. Deferred or reduced: the AppUserModelId is the literal string of AC1 (S-049 `APP_ID` not used); the icon is only a file name (S-048 not needed); the AC6 runner (stop at first failure, exit 1) and AC7 (README, `W`, `M`, CHANGELOG) have no test here. Failing output (before implementation):
  - core: `error[E0432]: unresolved import sbarhive_core::registry` (module absent).
  - cli: 7 of 7 fail: `ac4_*` with `left: Some(2) right: Some(0)` and `ac5_*` with clap `error: unrecognized subcommand 'integration'` (exit 2 matches by accident, so the tests also check the exact stderr text).
- 2026-10-24, implementer: STOPPED, two tests contradict AC1 and the test file's own exact-vector test. AC1 gives the default value command as `["add", KEY, "/ve", "/t", "REG_SZ", "/d", DATA, "/f"]`, which has 8 elements with DATA at index 6, and `ac1_install_commands_*_exact` expects that. But `ac1_default_value_command_keeps_its_quotes` (crates/core/tests/explorer_registry.rs:69 and :71) reads `got[3][7]` and asserts `got[3].len() == 9`, and `ac3_trailing_backslash_is_normalised` (line 135) reads `with[3][7]`; index 7 is `/f` in the 8 element list. Both need index 6 (and length 8). Implementation of registry.rs, the CLI integration, README, CHANGELOG and mutants.toml is written and uncommitted; all other tests pass (12 of 14 core, CLI not yet reached).
- 2026-10-24, implementer (run 2): after the manager fixed the two test indexes, all tests pass. Gates: fmt, clippy --all-targets -D warnings, cargo test --all, scripts/windows/check.sh and scripts/design-lint.sh all exit 0. Mutants (`--in-place --in-diff` over the diff vs 894a1ce~1): 25 tested, 25 caught, 0 missed. Status set to review.
- 2026-10-24, reviewer: PASS (reviewer PASS 2026-10-24). Gates run: fmt, clippy --all-targets -D warnings, cargo test --all, design-lint, scripts/windows/check.sh all exit 0; `cargo mutants --in-place --in-diff` over `git diff 894a1ce~1 HEAD`: 25 tested, 25 caught, 0 missed. Checked: registry.rs commands match ADR 0005 section 6 and AC1 to AC3 (HKCU only, MultiSelectModel Player, command `"<dir>\sbarhived.exe" open "%1"`, AppUserModelId key, uninstall deletes only the two trees); `integration/windows.rs` is excluded in `.cargo/mutants.toml` with a comment and holds only the reg.exe loop, the existence probe and printing, with the decisions (`missing_files`, `install_commands`) in core; README pair has the switch line, English main plus Arabic copy, no personal info; no long dashes in the changed files; CHANGELOG line in English and Arabic. Notes: (1) the stderr line is `sbarhive: integration install: available on Windows only`; it contains the AC5 text and follows the other CLI messages' `sbarhive:` prefix, accepted. (2) Manager deviation accepted: S-048/S-049 are not done, so the icon is only the file name `sbarhive.ico` in the program folder and the AUMID is the literal string from AC1; when S-049 lands, wire `registry.rs` AUMID_KEY and the tester's expectations to `APP_ID` so the notification and the registry key cannot drift apart. (3) The runner's stop at first failure and exit 1 on a missing file are not covered by a test on Linux; deferred to the S-052 live check.
