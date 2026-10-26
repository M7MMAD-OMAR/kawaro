# S-080 Windows `kawaro doctor` engine discovery matches engine lookup
Status: done (2026-10-26, implemented + merged to main 1bd98f6; 14 S-080 tests plus all gates green). Owns the first confirmed follow-up candidate recorded in S-079.

## Story
As a Windows user running `kawaro doctor`, I want its engine and tool lines to resolve programs exactly as the conversion engines do, so that a green line means the engine will actually run and a red line means it will not.

## Background (confirmed 2026-10-26, from the S-079 review)
- `doctor()` in `crates/cli/src/main.rs` probes bare `ffmpeg`, `ffprobe`, `soffice` and `gs` names on `PATH` through `on_path`, while the engines resolve programs through `crates/engines/src/locate.rs` (`locate::find`: `PATH`, then the Windows known dirs, then the registry; Ghostscript as `gs`, `gswin64c` or `gswin64`; LibreOffice from its default registry locations).
- The two disagree in both directions: doctor can print `missing` for an engine the engines will run (found in a known dir or the registry), and `ok` for a bare name the engines will not pick up. The S-079 checklist live step for `kawaro doctor` cannot be trusted until both use one lookup.

## Decisions
1. Engine and engine-tool discovery has one source of truth: `crates/engines/src/locate.rs` (or a shared helper that calls it). `doctor` calls it; `on_path` is removed from the four engine tools and stays only for the plain PATH tools.
2. Each engine tool line prints the resolved absolute path when found, so the operator can compare it with the engine logs in the same run.
3. The fix is cross-platform, since the engines share `locate::find`, but the acceptance focuses on the Windows names, known dirs and registry paths where the defect was confirmed.

## Acceptance criteria
- AC1: `kawaro doctor` resolves `ffmpeg`, `ffprobe`, `soffice` and `gs` (including `gswin64c` and `gswin64`) through the same lookup the engines use; a test proves the CLI contains no bare PATH probe for these four tools and that the resolution is `locate::find` or a direct wrapper of it.
- AC2: with FFmpeg, LibreOffice or Ghostscript present only in a Windows known dir or a default registry location and absent from `PATH`, doctor prints `ok` for its tools and the line shows the resolved absolute path. Proven with fixtures for the known-dir and registry cases.
- AC3: with an engine absent, its engine line and its tool lines agree: both `missing` in the same run. A fixture run proves the two never disagree.
- AC4: the plain PATH tools (`wl-paste`, `xclip`, `xdg-open`, `kawarod`) keep their current behaviour unchanged, covered by tests.
- AC5: gates in CLAUDE.md pass and one `CHANGELOG.md` line in English and Arabic is added. The S-079 doctor live step then reports the same paths the engine logs record when the owner runs it.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | Source test: the CLI has no `on_path` use for engine tools; unit test that the doctor tool rows call the shared lookup | unit |
| AC2 | Fixture environments with a known dir hit and a registry hit; assert `ok` and the printed path | unit, integration |
| AC3 | Fixture with engines removed; assert engine and tool lines agree | unit |
| AC4 | Existing doctor output tests for the plain PATH tools stay green | unit |
| AC5 | Quality gates and changelog check | quality gates |

## Out of scope
- The live Windows run itself (owner work, S-079).
- The `MoveFileExW` output placement defect (S-081).
- Any change to engine behaviour or to `locate::find` ordering beyond what doctor needs to share it.

## Review log
-
