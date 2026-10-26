# S-073 Release asset names match what GitHub stores, and the update feed job runs
Status: approved (manager, delegated, 2026-10-26)

## Story
As a beta tester, I want the files on the release page, `SHA256SUMS` and the signed update feed to name the same files, so that `sha256sum -c SHA256SUMS` works on what I download and the app finds its update.

## Background (the first run of the S-055 pipeline, run 40261909745)
- GitHub replaces `~` with `.` in uploaded asset names: `kawaro-0.4.0~beta.1-1.x86_64.rpm` is stored as `kawaro-0.4.0.beta.1-1.x86_64.rpm`, and the DEB likewise. `SHA256SUMS` still lists the `~` names, so `sha256sum -c` on downloaded files fails for the RPM and the DEB.
- `update-manifest` failed at `apt-get install minisign`: Ubuntu 22.04 has no minisign package. Had it got further, `scripts/make-update-manifest.sh` would have stopped with exit 3, because it looks for `dist/<asset name>` and the local file still has the `~`.
- The RPM and DEB package versions inside the files keep `~` (that is what makes a beta sort before the final release). Only the file names change.

## Acceptance criteria
- AC1: The `release` job renames every downloaded file in `dist` whose name contains `~`, replacing each `~` with `.`, before it writes `SHA256SUMS`. `SHA256SUMS` and the `gh release upload` command then name `kawaro-0.4.0.beta.1-1.x86_64.rpm` and `kawaro_0.4.0.beta.1_amd64.deb` for the tag `v0.4.0-beta.1`, and `sha256sum -c SHA256SUMS` passes in `dist`. A final version (`v0.4.0`) has no `~`, so its names are unchanged.
- AC2: The `update-manifest` job runs on `ubuntu-24.04` (which has minisign) and does the same rename in `dist` before `scripts/make-update-manifest.sh`, so every asset name the release reports has a local file.
- AC3: The rename is one shared step text, not two hand written loops that can drift: a script `scripts/release/github-names.sh DIR` that renames in place, prints each new name, exits 0 when nothing needs renaming, and refuses (exit 2, nothing renamed) when a renamed name would overwrite an existing file. Both jobs call it.
- AC4: `docs/adr/0006-auto-update.md` says in one sentence that GitHub stores `~` as `.` in asset names and that the pipeline renames before checksums and the manifest.
- AC5: Gates: `tests/release/run_all.sh` with 0 failed (the existing S-055 expectations that pinned the `~` upload names are updated by the tester to the `.` names, authorized here), `shellcheck` clean on the new script, `scripts/design-lint.sh` clean, one line in `CHANGELOG.md` in English and Arabic.

## Test cases
| AC | Test | Type |
| --- | --- | --- |
| AC1 | Run the release job's step texts (the existing fake gh harness in `tests/release/beta_workflow.sh`) for `v0.4.0-beta.1` and `v0.4.0`: upload names and `SHA256SUMS` names are the `.` names for the beta and unchanged for the final; `sha256sum -c` passes | integration |
| AC2 | Parse `packages.yml`: `update-manifest` has `runs-on: ubuntu-24.04`, calls `scripts/release/github-names.sh dist` before `make-update-manifest.sh`; the `release` job calls it before the line that writes `SHA256SUMS` | unit |
| AC3 | Script on a temp dir: `a~b.rpm` becomes `a.b.rpm` and is printed; a dir with no `~` is untouched, exit 0; `a~b` next to an existing `a.b` exits 2 and renames nothing | unit |
| AC4 | `grep` the ADR for the sentence | unit |
| AC5 | the gates | gate |

## Out of scope
- Changing package versions inside the RPM and DEB.
- The site download page (it links the release page, not file names).

## Review log
-
