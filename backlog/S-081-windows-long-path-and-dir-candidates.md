# S-081 Windows output placement: long paths and directory-name candidates
Status: draft (2026-10-26, planner). Owns the second confirmed follow-up candidate recorded in S-079.

## Story
As a Windows user converting a file in a deep or crowded folder, I want the result written correctly even when the output path is over 260 characters or a numbered output name is taken by a directory, so that a long path works and a taken name moves to the next free name instead of failing the job.

## Background (confirmed 2026-10-26, from the S-079 review)
- `place_no_replace` in `crates/core/src/output_impl/windows.rs` passes raw wide paths to `MoveFileExW` with flags 0. `wide()` in `crates/core/src/win.rs` adds no `\\?\` long-path prefix, so output paths over `MAX_PATH` fail at the move.
- `move_error_kind` in `crates/core/src/output_impl.rs` maps Win32 code 5 to `PermissionDenied`, and `finalize_with` (around lines 178 to 182) retries only `AlreadyExists`. When a numbered output candidate collides with an existing directory, `MoveFileExW` returns `ERROR_ACCESS_DENIED` (5), so the whole job fails with a terminal error instead of trying the next free name.

## Decisions
1. Long paths: the Windows placement path sends the `\\?\`-prefixed form (or another measured long-path mechanism) for both the temp file and the final name. The prefix logic is pure and unit tested; the live proof is an output path over 260 characters on NTFS.
2. A directory-name collision is "name taken": before mapping the error, the code checks whether the candidate exists as a directory and, if so, continues to the next numbered candidate. A blanket retry on `PermissionDenied` is not acceptable, since a real permission failure must still fail the job with its original error.
3. The `MAX_NUMBER` bound and the "never replace, never modify the original" rule are unchanged. The exFAT no-replace case from S-079 must keep passing.

## Acceptance criteria
- AC1: an output path longer than 260 characters writes the result on NTFS, with the temp file and the final name both placed successfully. A unit test proves the prefixed (or measured) form is used for both paths; a Windows run proves the file exists and its contents match the conversion.
- AC2: when a numbered output candidate collides with an existing directory, `finalize_with` moves to the next candidate and the result is the next free `name (N).ext` beside the unchanged original; nothing is deleted or replaced. Proven by an integration test with a fixture directory.
- AC3: a real file collision behaves exactly as today (`AlreadyExists` continues), and a genuine permission failure that is not a name collision still fails the job with the original error; the loop stays bounded by `MAX_NUMBER`.
- AC4: `move_error_kind` unit tests cover codes 5, 80, 183 and one unmapped code, and the directory-collision classification is separately tested, so code 5 keeps its plain mapping and the directory rule lives where the candidate is inspected.
- AC5: the S-079 exFAT no-replace case still passes, gates in CLAUDE.md pass, and one `CHANGELOG.md` line in English and Arabic is added.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | Unit test on the path form for temp and final names; Windows run with a 300 character output path | unit, integration |
| AC2 | Fixture with a directory at `name.ext` and `name (1).ext`; assert the result is `name (2).ext` and the directory is untouched | integration |
| AC3 | Fixture with a taken file (continues) and a fake place function returning a non-collision permission error (terminal) | unit |
| AC4 | Table test over the four error codes and the directory-collision branch | unit |
| AC5 | Existing exFAT and finalize tests unchanged; quality gates and changelog check | unit, quality gates |

## Out of scope
- The live Windows checklist run (owner work, S-079) beyond the cases above.
- The `kawaro doctor` discovery defect (S-080).
- Cross-volume moves and copy fallbacks, since the temp file always sits next to the output.

## Review log
-
