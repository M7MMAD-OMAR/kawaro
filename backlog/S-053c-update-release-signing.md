# S-053c Release job: manifest, signature and package versions
Status: approved (manager, by delegation, 2026-10-24)

Third core story of ADR 0006. Depends on P1 (key pair, secrets `UPDATE_SIGNING_KEY` and `UPDATE_SIGNING_PASSWORD`, environment `release-signing`) and P2 (`minisign`, `jq`). CI installs `minisign` itself (assumed in the ubuntu-22.04 universe repository; if absent the job downloads a pinned release and checks its SHA-256). About one work day.

## Story
As the maintainer, I want every tagged release to publish a signed update manifest by itself, so that users get the update without any manual step from me.

## Acceptance criteria
- AC1 (`scripts/make-update-manifest.sh <channel> <version> <assets.json> <out.json>`): `assets.json` is the output of `gh release view --json assets`; the script writes schema 1 JSON with one entry per `.msi`, `.rpm` and `.deb` asset: `name` and `url` copied verbatim from the input (a `~` in a name survives), `size` from the input, `sha256` from `sha256sum` of the local file of that name in `--dist <dir>`. A missing local file makes the script exit non zero and write no output file.
- AC2 (same script): the output parses with `Manifest::parse` and passes `evaluate` for the matching channel; `kind`, `os` and `arch` come from the file extension (`msi` is `windows`, `rpm` and `deb` are `linux`, all `x86_64`).
- AC3 (`scripts/sign-update-manifest.sh <key-file> <manifest>`): writes `<manifest>.minisig` with `minisign -S` and the passphrase from `UPDATE_SIGNING_PASSWORD`; it exits non zero if the key file is missing and never prints the key or the passphrase (the output of a run with a throwaway key contains neither).
- AC4 (`.github/workflows/packages.yml`, job `update-manifest`): `needs: release`; `if` is `startsWith(github.ref, 'refs/tags/v') && github.repository == 'M7MMAD-OMAR/sbarhive'`; `environment: release-signing`; the secrets appear only in this job; the `updates` release is created with `--prerelease --latest=false` when absent; a tag whose version contains `-` uploads `beta.json` and `beta.json.minisig` only, a plain tag uploads `stable.json`, `beta.json` and both signatures; upload uses `--clobber`.
- AC5 (`packaging/linux/build-packages.sh`): the package version is `Cargo.toml` `version` with `-` replaced by `~`. For `0.4.0-beta.1`: `rpm -qp --qf '%{VERSION}'` prints `0.4.0~beta.1`, `dpkg-deb -f ... Version` prints `0.4.0~beta.1`, and `dpkg --compare-versions 0.4.0~beta.1 lt 0.4.0` exits 0. For `0.4.0` the file names are unchanged.
- AC6 (`sbarhive.spec`, `DEBIAN/control`): `Recommends: curl` in both. Postinstall scripts are unchanged.
- AC7 (`docs/update-signing.md`, English): sections for generating the key, where the private key lives, the two secrets and the environment, rotation with `sbarhive-update-next.pub`, and the leak limit of ADR 0006. It contains the key id of `packaging/keys/sbarhive-update.pub`, and a script check compares the two.
- AC8 (secrets): `git log --all -p | grep -c 'encrypted secret key'` gives 0, and the workflow log of the throwaway run has no line matching the passphrase.
- AC9 (gates): `shellcheck` on both scripts exits 0; design-lint clean; one CHANGELOG line, English and Arabic.

## Test cases
The script test generates a throwaway key pair in a temp dir with `minisign -G -W`, so no real key is involved. It runs in CI where `minisign` is installed, and is the only test of this story that needs it.
| Criterion | Test | Type |
| --- | --- | --- |
| AC1, AC2 | Fixture `assets.json` with a `~` name and a dist folder of 3 small files: compare every field with `sha256sum` and `stat`; run the Rust parse on the output; remove one file and assert exit code and no output file | integration |
| AC3 | Sign, then `verify_manifest` with the throwaway public key returns `Ok`; a flipped byte returns `BadSignature`; grep the captured output for the passphrase gives 0 | integration |
| AC4 | A Rust test reads the workflow YAML and asserts the `if`, `needs`, `environment`, the flags and the file lists for `v0.4.0` and `v0.4.0-beta.2` | unit |
| AC5 | Run the script with a temp `Cargo.toml` version; query the built RPM and DEB; `dpkg --compare-versions` | integration |
| AC6, AC7, AC8 | `grep -c` of the lines; the key id comparison; the git history grep | review, unit |

## Out of scope
- Signing the RPM and DEB themselves, and a signed `SHA256SUMS` (S-016 lists them out of scope).
- The Windows MSI build (S-051); this job picks up whatever `.msi` is attached.
- Rotating the key automatically.

## CHANGELOG plan
- Signed update feed (S-053c): each release publishes a signed manifest, and beta packages sort below the final release. «تغذية تحديث موقّعة: كل إصدار ينشر بياناً موقّعاً، وحزم البيتا ترتّب دون الإصدار النهائي».

## Review log
- 2026-10-24, planner: written from ADR 0006; approved by the manager under the maintainer's delegation. Asset URLs come from the release itself because GitHub may rewrite characters in names.
