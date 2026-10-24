# S-052 Windows beta: live checklist the owner runs, and the release notes
Status: review

Builds on every Windows story S-040 to S-051 and collects their "live check deferred" items. For the Linux and Windows beta. This story writes files only (a checklist, small scripts, release notes); the owner runs the checklist on a real Windows machine. Half a work day.

## Story
As the owner, I want a short checklist with one script per step group, so that I can test the Windows beta on a real machine tonight and report each result as PASS or FAIL.

## Files
`docs/windows-beta-checklist.md` (English), `scripts/windows/{beta-smoke.cmd,collect-logs.cmd}`, `tests/windows/test_checklist.sh`, `README.md` (Windows beta section, English and Arabic), `CHANGELOG.md`.

## Acceptance criteria
- AC1: `docs/windows-beta-checklist.md` has at most 60 lines of steps, numbered `W01`, `W02`, and so on without gaps. Each step has three parts on its own lines: `Do:`, `Expect:` (a number or a visible state, no "fast" or "nice" without a number) and `Result: PASS | FAIL`. The test counts that every step has all three and that ids are consecutive.
- AC2: the checklist covers each deferred item as a step. A test greps for these keywords, each in at least one step: `transparent corners`, `console window`, `150%`, `Esc`, `ten files`, `drag`, `original`, `shortcut`, `tray`, `Arabic`, `toast`, `Open folder`, `Show more options`, `uninstall`, `SmartScreen`, `Narrator`, `soffice.bin` (cancel kills it), `ffmpeg`, `Ghostscript`, `LibreOffice`.
- AC3: timing steps state the budget as a number: the hive appears within 150 ms of the click, measured on a 60 fps screen recording by counting frames (9 frames or fewer), three trials, all three must pass; the shortcut hive is measured the same way from the `hold_ms` threshold.
- AC4: the first section is "Before you start": the Windows version (10 22H2 or 11), a free folder `%USERPROFILE%\sbarhive-beta-test`, and the rule that test windows have a bounded size and only windows the test opened are closed. It names the files to copy: the zip or the MSI from S-051, and `scripts/windows/run-tests.cmd` with the ISO from S-044.
- AC5: `scripts/windows/beta-smoke.cmd` (CRLF line endings, `file` says so) runs, from the unzipped folder: `sbarhive.exe --version`, `sbarhive.exe integration install --dry-run`, starts `sbarhived.exe`, runs `sbarhive.exe ping`, and writes one line per command `<command> <errorlevel>` to `%USERPROFILE%\sbarhive-beta-test\smoke.txt`, ending with `total N failed F`. It writes nothing outside that folder. A static test checks the file for these commands and the target path.
- AC6: `scripts/windows/collect-logs.cmd` (CRLF) copies `%LOCALAPPDATA%\SbarHive\logs\sbarhived.log` and `%APPDATA%\SbarHive\config.toml` into `%USERPROFILE%\sbarhive-beta-test\report\` and prints the folder. No upload of any file.
- AC7: the checklist ends with a "Report" section listing exactly what to send back: the PASS or FAIL list, `smoke.txt`, the log, and screenshots only of the hive and the toast. The files contain no personal names, machine names or paths: a test greps the new files for `/home/`, `C:\Users\<name>` patterns with a real name, and long dashes; there must be none.
- AC8: `README.md` gets a "Windows beta" section in English and Arabic: what works, what is unsigned (SmartScreen), Windows 11 menu under "Show more options", engines (LibreOffice, Ghostscript, FFmpeg) not bundled. `tests/windows/test_checklist.sh` passes (`bash`, `shellcheck` clean), `scripts/design-lint.sh` clean, one CHANGELOG line.

## Check commands
`bash tests/windows/test_checklist.sh` (light, a few seconds, outside the cargo workspace), `shellcheck`, `file` on the two `.cmd` files, `scripts/design-lint.sh`.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | Script counts steps and the three parts per step, and id continuity | unit |
| AC2 | Script greps the keyword list in the checklist | unit |
| AC3, AC4, AC7, AC8 | Script greps the numbers, headings and the forbidden patterns; reviewer reads the README | unit, review |
| AC5, AC6 | `file` says CRLF; script greps the required commands and paths | unit |

## Out of scope
- Running the checklist (the owner does); the 0.6.0 tag and CI first green run (decided after the beta report); signing; the Windows 11 first-level menu.

## Review log
- 2026-10-24, planner: replaces the ADR 0005 S-052 "live checklist and release" with a short owner-run checklist, as the owner wants the beta tomorrow.
- 2026-10-24, tester: `tests/windows/test_checklist.sh` written, 57 checks, 55 fail for the right reason (docs/windows-beta-checklist.md, both .cmd files and the README "Windows beta" sections do not exist; no CHANGELOG line for S-052). The 2 passing checks are shellcheck on the test itself and design-lint (gates, not features). Reduced for the beta because S-047, S-048, S-049 are not built: AC2 keywords drag, shortcut, tray, toast, Open folder are not required; AC3 shortcut timing from hold_ms is not tested; AC7 toast screenshot is not required.
- 2026-10-24, implementer: checklist (14 steps, 56 step lines), both .cmd files (CRLF), README sections in English and Arabic and the CHANGELOG line written; test_checklist.sh 84 checks, 0 failed; unbuilt features (S-047, S-048, S-049) marked as not in this build.
