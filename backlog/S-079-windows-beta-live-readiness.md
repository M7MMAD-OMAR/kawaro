# S-079 Windows beta: live readiness checklist for the shipped feature set
Status: approved under manager delegation (2026-10-26)

Rebuilds the owner-run S-052 checklist for the beta that actually shipped. Covers the live items deferred by S-040 to S-051, the shipped drop, tray, shortcut and notification stories, the Windows updater and the S-067 Windows fixes. Documentation and static tests only; the owner runs the live steps.

## Story
As the owner, I want every shipped Windows feature and deferred live check to have an actionable PASS or FAIL procedure, so that the beta report shows what is ready and identifies failures without guessing or skipping shipped behavior.

## Files
`docs/windows-beta-checklist.md`, `tests/windows/test_checklist.sh`.

## Acceptance criteria
- AC1: Replace every stale claim that drop, tray, shortcut, icon or notifications are absent. There is no SKIPPED step for shipped behavior. Steps are consecutive `W01`, `W02`, and so on; every step has one `Do:`, one observable `Expect:`, and `Result: PASS | FAIL`. The Report section names the full resulting ID range.
- AC2: `tests/windows/test_checklist.sh` contains a coverage table mapping every item in AC3 to AC9 to a step title and required action and expectation text. Each assertion searches its matched step, not merely the whole document, so a keyword in another step cannot satisfy coverage. The test also rejects vague expectations and any skipped shipped feature.
- AC3: Installation and hive steps cover zip and MSI checksums, unsigned SmartScreen, per-user install without elevation, Start menu and icon, portable integration install and uninstall, missing-file refusal and stopping at the first registry failure, one and ten selections, `Show more options`, pointer and screen-edge placement, transparent corners, no shadow, frame or taskbar button, 150% scaling, focus, Esc, Narrator, Arabic RTL and Arabic filenames. Timing remains three 60 fps trials, all within 150 ms and 9 frames or fewer. Conversion and collision steps prove a new result next to an unchanged original.
- AC4: Named-pipe steps give concrete second-account and remote-client procedures for `ERROR_ACCESS_DENIED`, remote rejection, and rejection of a foreign owner before request bytes are sent. They also cover a second daemon and ten concurrent starts leaving one server, the 10 s client timeout, the 500 ms daemon check, and `CancelIoEx` returning without a hung client or daemon. Daemon-start steps cover `kawaro open` surviving terminal close and the retry inside a job that forbids breakaway.
- AC5: Path steps cover atomic settings save to `%APPDATA%\Kawaro`, logs under `%LOCALAPPDATA%\Kawaro\logs`, user action discovery, empty or missing `APPDATA` and `LOCALAPPDATA` falling back to the actual known folders, and an exFAT conversion with occupied output names proving no replacement and the next free name. `kawaro doctor` has a live step for its endpoint and reported paths.
- AC6: Engine steps use real LibreOffice, Ghostscript and FFmpeg installs or bundled FFmpeg discovery, including default registry locations, missing-engine messages, a drive-letter path with spaces and Arabic, and `soffice.com` waiting for completion. Normal exit, cancel, timeout and daemon termination leave no engine child or grandchild, including `soffice.exe`, `soffice.bin`, FFmpeg, ffprobe and Ghostscript. No daemon, FFmpeg, ffprobe or Ghostscript console window appears or flashes.
- AC7: Shell steps cover Explorer drag highlight, drop with an unchanged original, cancel and a new drag after close with no stale files; shortcut timing from the `hold_ms` threshold, release, OLE drag and swapped primary buttons; tray progress, download progress, English and Arabic RTL menus, outside click, Cancel all, Settings, Quit without a ghost icon, Explorer restart and flat GDI handles over repeated progress updates. A taken shortcut logs `shortcut <string> unavailable` while the daemon stays alive. Each finished job gives exactly one Kawaro toast without taking focus, and Open folder selects a result whose path contains spaces, logs a spawn failure and reaps its child.
- AC8: Updater steps use the real signed beta feed and a maintainer-supplied newer MSI. Cover `update --check`, download and verify, `--status` without network, manual check with `update_check = false`, the enabled tray Update now action, busy-job or open-hive deferral, and both ask-mode and idle auto-mode apply. The live apply proves the detached helper survives daemon quit, finds `msiexec.exe` through `SystemRoot`, installs the per-user MSI, restarts `kawarod.exe`, writes the log and one-shot result, cleans temporary state, and shows one update notification. Include the observed mappings for MSI codes 0, 3010, 1602, 1618 and 1603, and any antivirus or SmartScreen block of the copied helper. An offline or hung check leaves the daemon responsive.
- AC9: The native runner step mounts the S-044 ISO read only, runs `run-tests.cmd` from the disc, and requires one result per test exe, `total N failed 0`, and full logs outside the disc. A release-readiness step requires `windows-cross` and `windows-native` success for the exact tested commit. MSI upgrade, refused downgrade, next-logon start, and uninstall remove only Kawaro files, keys and its Run value while preserving converted files and other Run values.
- AC10: Preserve the useful S-052 static checks for smoke and log scripts, report contents and privacy, CRLF, no uploads, and forbidden personal paths or long dashes. `bash tests/windows/test_checklist.sh`, `shellcheck -x tests/windows/test_checklist.sh`, and `scripts/design-lint.sh` pass. No live Windows result is claimed by these static tests.

## Check commands
`bash tests/windows/test_checklist.sh`; `shellcheck -x tests/windows/test_checklist.sh`; `bash scripts/design-lint.sh`. No cargo build or mutation run is needed because no Rust code changes.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1, AC2 | Parse step blocks, IDs, required fields, result values, stale and skipped wording; evaluate the per-step coverage table | unit |
| AC3 | Match install, hive, timing, language and output-preservation actions and observable expectations in their mapped steps | unit |
| AC4 | Match both account and remote attacks, foreign owner, instance storm, both deadlines and cancellation | unit |
| AC5 | Match environment fallback, known folders, settings, actions, logs, doctor and exFAT collision procedures | unit |
| AC6 | Match discovery, path, waiting, missing-engine, complete process-tree and console checks | unit |
| AC7 | Match drop freshness, shortcut input variants, tray lifecycle, resources and toast behavior | unit |
| AC8, AC9 | Match updater helper evidence, native ISO results, CI identity and MSI lifecycle | unit |
| AC10 | Run the static suite, shellcheck and design lint; reviewer reads every step as a Windows operator | unit, review |

## Manager decision (2026-10-26)
- D1: The S-052 limit of 60 nonblank step lines is superseded. Security setup, process-tree checks and the real updater cannot remain actionable within it. The new limit is 40 steps and 160 nonblank lines from `W01` to before Report, normally four lines per step. Related checks may share a step only when one procedure performs them and Expect states each result. Compression may not remove commands, prerequisites, counts, paths or observable outcomes.

## Out of scope
- Production code, installers, release workflows, new test helpers and all fixes.
- Public code signing and the modern first-level Windows 11 Explorer menu.
- Custom shortcut reset on Settings save, owned by S-076, not S-079.
- Confirmed follow-up candidate: Windows `kawaro doctor` engine discovery disagrees with engine lookup because its tool list probes bare `ffmpeg`, `soffice` and `gs` names.
- Confirmed follow-up candidate: `MoveFileExW` long-path handling and directory-name output candidates need separate coverage; this story requires the existing exFAT no-replace case only.

## Review log
- 2026-10-26, planner: approved under manager delegation. Inventory taken from S-052, the deferred live items in S-040 to S-051, the shipped updater and S-067. D1 explicitly replaces the stale line limit; the two confirmed follow-up candidates are recorded separately above.
