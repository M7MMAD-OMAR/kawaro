# S-055 Beta release pipeline: 0.4.0-beta.1 for Linux and Windows
Status: review

First public beta, tag `v0.4.0-beta.1`, a GitHub prerelease. Depends on S-053c (signed manifest job, `~` package versions), S-051 (zip and MSI scripts) and the updater stories S-053a to S-054b. One work day: the workflow job, the notes and the dry-run script are small; the version bump is mechanical. If S-054a or S-054b is not done when this starts, the notes say "updates are check only" and the story still ships.

## Story
As the maintainer, I want one tag push to build, check and publish the Linux packages and the Windows zip and MSI as a prerelease with checksums and honest notes, so that testers on both systems can install the beta and receive later betas from inside the app.

## Acceptance criteria
- AC1 (version bump): `Cargo.toml` `[workspace.package]` `version` is `0.4.0-beta.1`, `Cargo.lock` workspace crates match, and `packaging/io.github.m7mmad_omar.SbarHive.metainfo.xml` has a `<release version="0.4.0-beta.1">` entry. `scripts/release/check-versions.sh <version>` prints one line per source and exits 0 only when all agree on: Cargo.toml `0.4.0-beta.1`; RPM `%{VERSION}` and DEB `Version` `0.4.0~beta.1`; MSI `ProductVersion` `0.4.0`; zip folder name and MSI file name containing `0.4.0-beta.1`; manifest `version` `0.4.0-beta.1`. Any mismatch exits 1 and names the source.
- AC2 (workflow `windows` job in `.github/workflows/packages.yml`): runs on `ubuntu-22.04`; installs `mingw-w64` and `msitools`; runs `scripts/windows/build.sh`, `make-zip.sh`, `make-msi.sh` with `CI=true`; uploads an artifact `windows` holding exactly one `.zip` and one `.msi`. It has no `environment` and no `secrets.` reference.
- AC3 (workflow `release` job): `needs` contains `test` and `windows`; the `if` stays `startsWith(github.ref, 'refs/tags/v')`; it downloads both artifacts into one folder, writes `SHA256SUMS` there (one line per file, exactly 4 lines: rpm, deb, zip, msi), creates the release when absent with `--prerelease` when the tag name contains `-` and without it otherwise, using `--notes-file docs/release-notes-<version>.md`, then uploads everything with `--clobber`. `update-manifest` needs `release`.
- AC4 (release notes): `docs/release-notes-0.4.0-beta.1.md` and `docs/release-notes-0.4.0-beta.1.ar.md` exist; the Arabic file starts with the switch line `[English](release-notes-0.4.0-beta.1.md) | [العربية](release-notes-0.4.0-beta.1.ar.md)` and the English file with the same line. Each has the headings Status, What works, Known gaps, Updates, Privacy, Verify your download. Known gaps names, in both languages: Windows unsigned with the SmartScreen steps ("More info", "Run anyway"); no drop hive (S-047) and no notifications (S-049) on Windows; Linux focus tests on real GNOME and KDE still pending. Updates names the signed manifest, the channels `beta` and `stable`, and `update_check = false`. Privacy says no telemetry. Zero long dashes and no personal data (design-lint clean).
- AC5 (Linux packages): with `Cargo.toml` at `0.4.0-beta.1`, `packaging/linux/build-packages.sh` produces `sbarhive-0.4.0~beta.1-1.x86_64.rpm` and `sbarhive_0.4.0~beta.1_amd64.deb`; `rpm -qp --qf '%{VERSION}'` prints `0.4.0~beta.1`; `dpkg --compare-versions 0.4.0~beta.1 lt 0.4.0` exits 0. The `packages.yml` `test` matrix still installs both on its 5 images.
- AC6 (windows artefacts): the zip and MSI for the version exist with names `sbarhive-0.4.0-beta.1-windows-x64.zip` and `.msi`; `msiinfo export <msi> Property` shows `ProductVersion` `0.4.0`; the zip's own `SHA256SUMS` verifies with `sha256sum -c`.
- AC7 (`scripts/release/dry-run.sh <version> [--no-build]`): copies tracked files to a temp dir (no branch, no worktree), sets the workspace version there, builds Linux packages, the Windows zip and MSI, a throwaway minisign key pair signs a manifest built by `make-update-manifest.sh`, runs `check-versions.sh`, writes `SHA256SUMS`, prints the final file list, and never calls `gh`, `git push` or any network tool. Exit 0 on success; exit 2 with a message naming the missing tool (`rpmbuild`, `dpkg-deb`, `wixl`, `minisign`, `mingw` target); a version that is not semver exits 2. `--no-build` reuses existing release trees (`BIN_DIR`, `CARGO_TARGET_DIR`) so tests finish in under 60 s. The source tree is unchanged afterwards (`git status --porcelain` is empty).
- AC8 (workflow structure): a script test reads `packages.yml` and asserts AC2 and AC3 facts: job names, `runs-on`, `needs`, the `--prerelease` condition for `v0.4.0-beta.1` and its absence for `v0.4.0`, the artifact names, and that `UPDATE_SIGNING_KEY` and `UPDATE_SIGNING_PASSWORD` appear only inside `update-manifest`.
- AC9 (gates): `shellcheck` on the new scripts exits 0; design-lint clean; one CHANGELOG line, English and Arabic; `docs/PLAN.md` lists 0.4.0-beta.1.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | Run `check-versions.sh 0.4.0-beta.1` on a dry-run output: exit 0 and 6 lines. Change one spec define to `0.4.0-beta.1` and expect exit 1 naming the RPM | integration |
| AC2, AC3, AC8 | Parse the YAML (python or yq): compare every asserted field with the expected value, for both tag names | unit |
| AC4 | Grep both notes for each heading, each gap phrase, `update_check = false`, the switch line, and `grep -c` of long dashes gives 0 | unit |
| AC5 | Build with the beta version; query `rpm -qp` and `dpkg-deb -f`; `dpkg --compare-versions` | integration |
| AC6 | `msiinfo export` Property row text; `unzip -p` then `sha256sum -c` | integration |
| AC7 | Run with `--no-build` on fixture trees: exit 0 and the exact file list; with `wixl` hidden from PATH exit 2 and the message names it; `foo` as version exits 2; `git status --porcelain` is empty; grep of the script gives 0 for `gh `, `push` and `curl` | integration |
| AC9 | shellcheck exit codes, design-lint, `grep -c` in CHANGELOG | review |

## Owner steps, not automated
1. Generate the key pair and commit the public key (S-053c P1), then set the secrets `UPDATE_SIGNING_KEY` and `UPDATE_SIGNING_PASSWORD` and create the protected environment `release-signing` (tag-only deployment rule). Without them `update-manifest` fails and the beta cannot update itself.
2. Run `scripts/release/dry-run.sh 0.4.0-beta.1` inside the 6 GB scope and read the output.
3. Push `main`, then the tag `v0.4.0-beta.1`.
4. Check the Actions run, open the prerelease, read the notes once, and publish it (if the job leaves it as a draft).
5. Run the Windows live checklist (S-052) on a real machine; it is not part of CI.

## Out of scope
- Signing the MSI (`osslsigncode`), signing the RPM and DEB, signed `SHA256SUMS`.
- The drop hive (S-047) and Windows notifications (S-049).
- arm64, X11 packages, signed repositories (S-016).
- The real GNOME and KDE focus test.

## CHANGELOG plan
- First public beta (S-055): 0.4.0-beta.1 for Linux and Windows, built and checked by CI, published as a prerelease. «أول إصدار تجريبي علني: 0.4.0-beta.1 للينكس وويندوز، يُبنى ويُفحص في CI ويُنشر كإصدار تجريبي».

## Review log
- 2026-10-24, planner: written from ADR 0005 section 8, ADR 0006 and S-051/S-053c. Open for the manager: (a) AC3 fixes SHA256SUMS at 4 lines; (b) the release is created by the `release` job today only if absent, previously it had to exist before the tag.

- 2026-10-24, manager rulings on the planner questions: (1) yes, the `release` job creates the release when absent, with `--prerelease` for tags that contain `-`. (2) The icon exists since S-048 (design/brand/sbarhive.ico, carried by the zip and the MSI), so it is no gap. (3) If S-054a or S-054b is not done when the beta is cut, the release notes must say in-app updates are not in this beta; the notes follow what is actually merged. (4) and (5) are settled by the first real CI run: the `windows` job gets rust-cache and a 45 minute timeout; if `minisign` or `msitools` are missing on the runner the job installs them with apt and records the version in docs/engines.md.

- 2026-10-24, manager, carried from the S-053c review (handle in this story): (1) the `update-manifest` job must verify each signature with `minisign -V -p packaging/keys/sbarhive-update.pub` before uploading anything; (2) upload each `.minisig` after its json so a new json never sits next to an old signature; (3) delete the temporary key file with a `trap ... EXIT`, not only on success; (4) narrow the top-level `permissions: contents: write` to the `release`, `windows` upload and `update-manifest` jobs; (5) `make-update-manifest.sh` should also refuse urls outside `https://github.com/M7MMAD-OMAR/sbarhive/releases/download/`; (6) the `windows` job must put the MSI in the `packages` artifact (dist) before `update-manifest` runs, or the script exits 3. The dry-run script must call `scripts/make-update-manifest.sh --dist DIR CHANNEL VERSION ASSETS_JSON OUT_JSON` (options first, VERSION without leading v, exit 2/3/4) and `scripts/sign-update-manifest.sh KEY_FILE MANIFEST` (passphrase from UPDATE_SIGNING_PASSWORD on stdin, an empty value is allowed for -W keys). Owner step to add: restrict the `release-signing` environment to `v*` tags in the repository settings.

- 2026-10-24, tester: tests written from the ACs and the manager rulings, all in `tests/release/` and wired into `run_all.sh`: `beta_versions.sh` (AC1 repo state), `beta_manifest.sh` (carried 5), `beta_workflow.sh` (AC2, AC3, AC8, AC5 matrix, carried 1 to 4, 6), `beta_notes.sh` (AC4, AC9 docs), `beta_dryrun.sh` (AC1 mismatches, AC5, AC6, AC7, AC9 shellcheck; uses `tests/release/shims/dpkg-deb` where dpkg-deb is absent). Failure run before implementation (existing S-053c tests stay green: manifest 46, sign 31, workflow 37, packages 19, docs 38):
  - beta_versions: 6 of 6 fail (Cargo.toml found 0.3.3; Cargo.lock; metainfo has no 0.4.0-beta.1 entry; check-versions.sh exit 127).
  - beta_manifest: 8 of 10 fail (urls outside https://github.com/M7MMAD-OMAR/sbarhive/releases/download/ exit 0, want 2).
  - beta_workflow: 49 of 80 fail (no `windows` job; `release` needs only test; top-level `permissions: contents: write`; no `minisign -V`; no trap; no SHA256SUMS, create or notes file; wrong signature does not fail the steps; key file left behind after a failed signing).
  - beta_notes: 2 of 2 fail (both notes files missing).
  - beta_dryrun: 2 of 2 fail (scripts/release/dry-run.sh and check-versions.sh missing; the remaining 84 checks were validated against a throwaway prototype outside the repo, where they pass).
  Contract decisions are in the header comment of each test file.

- 2026-10-24, implementer: done, status review. Added `scripts/release/check-versions.sh` and `dry-run.sh`, the `windows` job and the changed `release` and `update-manifest` jobs in `packages.yml` (top-level permissions read only, `contents: write` on `release` and `update-manifest`), msitools in `ci.yml`, the url prefix check in `make-update-manifest.sh`, both release notes, the bump to 0.4.0-beta.1 (Cargo.toml, Cargo.lock, metainfo), the CHANGELOG line, PLAN row, README note, engines.md rows and the CLAUDE.md Status section. No test was edited. Notes: (1) `beta_dryrun.sh` check "no branch or worktree was created" asserts `git worktree list` has one line, so it fails on a checkout that has other worktrees (this machine has four under `.claude/worktrees`, which were not touched); it passes in a clean clone and in CI. (2) The manual update check has no menu entry or command yet (S-054c is still planned), so the notes say so.
- 2026-10-24, implementer, test problem (not edited, for the manager): `tests/windows/test_s051.sh` lines 220-221 and 234 compare the MSI `ProductVersion` with the whole workspace version (`VERSION` read at line 39). With the version 0.4.0-beta.1 the script gives `ProductVersion` 0.4.0, which is required by S-055 AC1 and AC6 and by the header of `scripts/windows/make-msi.sh` (MSI versions are numeric), so the test fails 2 of 91 checks ("AC4: ProductVersion: got [0.4.0] want [0.4.0-beta.1]", same with the real exes). The expected value should be `${VERSION%%[-+]*}`. The production script is right; the test needs that one change.
