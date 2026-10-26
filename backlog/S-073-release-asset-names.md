# S-073 Release asset names match what GitHub stores, and the update feed job runs
Status: done

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
- 2026-10-26, tester: new `tests/release/s073_asset_names.sh` (41 checks, AC number in each name), added to `tests/release/run_all.sh`; `tests/release/beta_workflow.sh` now expects the `.` upload names for the beta tag and links the repo's `scripts` into its throwaway workspace (the release job will call `scripts/release/github-names.sh`). Contracts: AC1 runs the release run texts with a fake gh for `v0.4.0-beta.1` and `v0.4.0`, contents survive the rename; AC2 the call is `scripts/release/github-names.sh dist`, before the first `make-update-manifest.sh` and before the first redirect into SHA256SUMS; AC3 prints one line per renamed file (base name or DIR/base name), prints nothing when nothing needs renaming, and a collision renames nothing at all, also not other files in DIR; no `mv` left in the release and update-manifest run texts; AC4 one ADR 0006 paragraph names GitHub, asset, `~`, a quoted dot, a rename, checksum or SHA256SUMS, and the manifest; AC5 shellcheck on the script and one CHANGELOG line naming S-073. The `v0.4.0` checks and the "untouched" checks pass now as regression guards. `shellcheck -x` clean. Run before implementation (`PATH=/usr/bin:$PATH bash tests/release/run_all.sh`, rc 1; every other file 0 failed):
  ```
  == beta_workflow
  FAIL AC3 v0.4.0-beta.1: upload names the four files and SHA256SUMS
  82 checks, 1 failed
  == s073_asset_names
  FAIL AC2 update-manifest runs-on ubuntu-24.04
  FAIL AC2 update-manifest calls scripts/release/github-names.sh dist
  FAIL AC2 update-manifest renames before make-update-manifest.sh
  FAIL AC2 release calls scripts/release/github-names.sh dist
  FAIL AC2 release renames before the line that writes SHA256SUMS
  FAIL AC4 ADR 0006 says GitHub stores ~ as . in asset names and the pipeline renames before checksums and the manifest
  FAIL AC1 v0.4.0-beta.1: no file name in dist contains ~
  FAIL AC1 v0.4.0-beta.1: dist holds kawaro-0.4.0.beta.1-1.x86_64.rpm with the rpm content
  FAIL AC1 v0.4.0-beta.1: dist holds kawaro_0.4.0.beta.1_amd64.deb with the deb content
  FAIL AC1 v0.4.0-beta.1: SHA256SUMS names kawaro-0.4.0.beta.1-1.x86_64.rpm, kawaro_0.4.0.beta.1_amd64.deb, the zip and the msi
  FAIL AC1 v0.4.0-beta.1: gh release upload names kawaro-0.4.0.beta.1-1.x86_64.rpm, kawaro_0.4.0.beta.1_amd64.deb, the zip, the msi and SHA256SUMS
  FAIL AC3 scripts/release/github-names.sh exists and is executable
  FAIL AC3 a~b.rpm: exit 0
  FAIL AC3 a~b.rpm becomes a.b.rpm with its content
  FAIL AC3 a~b.rpm is gone
  FAIL AC3 every ~ is replaced: x~y~z.deb becomes x.y.z.deb
  FAIL AC3 the new name a.b.rpm is printed
  FAIL AC3 the new name x.y.z.deb is printed
  FAIL AC3 only the two renamed files are printed
  FAIL AC3 a dir with no ~: exit 0
  FAIL AC3 a~b next to an existing a.b exits 2
  FAIL AC5 shellcheck clean on scripts/release/github-names.sh (no script)
  FAIL AC5 one CHANGELOG.md line names S-073
  41 checks, 23 failed
  ```
- 2026-10-26, implementer: new `scripts/release/github-names.sh DIR` (set -eu, executable): collects every `~` name in DIR first and exits 2 with a stderr message, renaming nothing, when a new name exists already or two files would get the same new name (checked by hand: `a~b.c` next to `a.b~c` gives rc 2, both kept); then renames each `~` to `.` and prints the base name; no output and exit 0 when nothing needs renaming; exit 2 on misuse. `packages.yml`: the release job has a step `GitHub asset names` (`scripts/release/github-names.sh dist`) before Checksums; update-manifest runs on `ubuntu-24.04` and has the same step after the two downloads, before Manifests; no `mv` in either job's run texts; the windows job, environments and secrets are unchanged. ADR 0006 has one sentence after the `build-packages.sh` paragraph; one CHANGELOG line in English and Arabic. Gates: `PATH=/usr/bin:$PATH bash tests/release/run_all.sh` rc 0 (manifest 46/0, sign 31/0, workflow 37/0, packages 19/0, docs 38/0, beta_versions 6/0, beta_manifest 10/0, beta_workflow 82/0, beta_notes 54/0, beta_dryrun 86/0, s073_asset_names 41/0 checks/failed); `shellcheck scripts/release/github-names.sh` rc 0; `scripts/design-lint.sh` rc 0 (clean). No test file edited; no crates change, so no cargo gates.
- 2026-10-26, reviewer (fresh context): PASS. Ran `PATH=/usr/bin:$PATH bash tests/release/run_all.sh`: rc 0, every file 0 failed (manifest 46, sign 31, workflow 37, packages 19, docs 38, beta_versions 6, beta_manifest 10, beta_workflow 82, beta_notes 54, beta_dryrun 86, s073_asset_names 41). `shellcheck scripts/release/github-names.sh` rc 0, `shellcheck -x` on the two changed tests rc 0, `scripts/design-lint.sh` clean. No crates change, so no cargo gates. Probes of the script in a scratch dir: `my file~beta 1.rpm` becomes `my file.beta 1.rpm`, `a~~b~c.deb` becomes `a..b.c.deb`, `-x~y` becomes `-x.y` (printed, `mv --`), all rc 0; empty dir rc 0 and no output; missing dir, no args, two args, a file instead of a dir: usage, rc 2; `a~b.c` next to `a.b~c` rc 2 and both kept; `a~b` next to `a.b` and `z~z` rc 2, nothing renamed; a dir named `-dash` and `tr/` work; a second run on a renamed dir is rc 0 and silent. Real failure: `podman` ubuntu:24.04 `apt-cache policy minisign` gives candidate 0.11-1 from noble/universe, ubuntu:22.04 gives no package, so `runs-on: ubuntu-24.04` fixes the update-manifest death. Fed `make-update-manifest.sh` an assets.json with the `.` names GitHub reports: rc 3 (`no local file for kawaro-0.4.0.beta.1-1.x86_64.rpm`) before the rename, rc 0 with rpm, deb and msi entries after it; `sha256sum -c` passes on the renamed files. The secrets are still only whole env values of the Sign step (the new step has no env; workflow.sh AC4 checks pass); the release upload keeps `--clobber`. Minor findings, not blocking, doc only: (1) `docs/engines.md` minisign row still says "22.04 on `update-manifest`", now 24.04; (2) ADR 0006 "Still open" keeps "`minisign` availability in the ubuntu-22.04 runner image (assumed in universe)", now settled (22.04 has none, the job runs on 24.04); both are one line edits the manager can fold in before merge. (3) `scripts/release/dry-run.sh` and `tests/release/beta_dryrun.sh` still produce `~` file names and a self made assets.json with `~` names; consistent with each other and with what `build-packages.sh` writes, so acceptable here, but the dry run no longer models the listing CI reads and does not rehearse the rename; worth a follow-up story. (4) Script edge cases outside `dist` use: a directory whose name has `~` is renamed too, hidden `.x~y` files are skipped by the glob, and a dangling symlink at the new name is overwritten because `-e` is false for it.
- 2026-10-26, manager: CI on 4282788 failed one check, `AC5 shellcheck clean`: Ubuntu 24.04 ships shellcheck 0.9.0, which reports SC2015 on the usage line `[ ... ] && [ ... ] || { ...; }` (the local 0.10 does not). The usage guard is now an `if`; shellcheck 0.9.0 in an ubuntu:24.04 container and the local one both exit 0, and `s073_asset_names.sh` is 41 checks, 0 failed.
