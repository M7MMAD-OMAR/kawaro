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
- 2026-10-24, tester: tests written from the ACs, none of them touches the real key. Shell: `tests/release/{manifest,sign,workflow,packages,docs}.sh`, run all with `tests/release/run_all.sh` (needs minisign, jq, shellcheck, python3 with PyYAML, rpmbuild, no compile). Rust: `crates/core/tests/s053c_update_feed.rs` (5 tests, AC1 to AC3, runs both scripts with a `minisign -G -W` key and feeds `Manifest::parse`, `evaluate`, `select_asset`, `verify_manifest`). A prototype of the two scripts made every manifest and sign check pass (then deleted), and a mock job made every workflow check pass (then reverted), so the tests are satisfiable. Failure output before implementation:
  - manifest.sh: `FAIL AC1 script is executable`, `FAIL AC1 exit 0 on good input`, `FAIL AC1 output file exists`; `3 checks, 3 failed` (stops: the script is missing).
  - sign.sh: `FAIL AC3 script exists and is executable`; `2 checks, 1 failed`.
  - workflow.sh: `FAIL AC4 job update-manifest exists`, `FAIL AC3 ci.yml installs minisign with apt-get` (and jq, rpm), `FAIL AC3 ci.yml runs tests/release/run_all.sh`, `FAIL AC4 no publish step to run`; `6 checks, 6 failed`.
  - packages.sh: `FAIL AC6 the spec has a Recommends: curl line`, `FAIL AC5 build-packages.sh succeeds for 0.4.0-beta.1`, `FAIL AC5 rpm VERSION is 0.4.0~beta.1`, `FAIL AC6 rpm Recommends curl`, ...; `19 checks, 13 failed`.
  - docs.sh: `FAIL AC7 docs/update-signing.md exists`, the AC7 section checks, `FAIL AC9 ... scripts/make-update-manifest.sh exists and is executable`, `FAIL AC9 exactly one CHANGELOG line for S-053c`; `38 checks, 29 failed`.
  - cargo test -p sbarhive-core --test s053c_update_feed: 5 failed, `the program must start: Os { code: 2, kind: NotFound ... }` (scripts/make-update-manifest.sh missing).
  Checks that already pass are regression guards of facts the story keeps (postinst, `%post`, 0.4.0 names, design-lint, no secret in git). Deviations and decisions are in the header comments of each test file (marked DECISION). AC8 deviation: the literal `git log --all -p | grep -c 'encrypted secret key'` already prints 1 because this story file quotes the phrase; the test greps for the full first line of a minisign secret key file and the base64 prefix of an encrypted key body instead (spelled out nowhere in the repo). The dpkg only checks (`dpkg-deb -f`, `dpkg --compare-versions`) print SKIP on a machine without dpkg (Fedora); CI runs them.
  CI wiring (tester finding, not in the ACs): `cargo test --workspace` in `ci.yml` will run the Rust half, which needs `minisign` and `jq`, and nothing runs `tests/release/run_all.sh`; the implementer must make `ci.yml` run `apt-get install` of `minisign jq rpm` and then `tests/release/run_all.sh` (checked by `workflow.sh`).
