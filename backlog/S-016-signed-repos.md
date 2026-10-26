# S-016 Signed APT and COPR repositories

Status: approved (2026-10-26, manager)

## Context

`packages.yml` builds one RPM and one DEB on Ubuntu 22.04, tests them in containers, and attaches them to each GitHub release. ADR 0006 and S-054a to S-054c already provide signed in-app update checks and installation. This story adds native repositories; it does not replace or redesign the updater. Issue 3 also requires authenticated release downloads, a public OpenPGP key, and checksums.[1]

S-016 owns only APT, COPR, their install commands, and OpenPGP authentication of GitHub release assets. S-078 owns AppStream, AUR, and winget.

## Story

As a Fedora, Ubuntu, or Debian user, I want to configure one trusted repository and then install and upgrade Kawaro with my normal package manager, without signature warnings or manual downloads.

## Manager decisions

1. **APT hosting:** use a public Cloudflare R2 bucket named `kawaro-apt` at `https://apt.kawaro.sbarah.com`. Cloudflare documents custom domains as the production public-bucket path and `r2.dev` as rate limited, non-production access.[4] The `r2.dev` endpoint stays disabled. This is separate from the website Worker and does not reopen GitHub Pages. ADR 0007 keeps release downloads on GitHub; the APT repository contains repository packages and metadata, not replacements for the GitHub release assets or update feed.
2. **PPA rejected:** Launchpad describes PPAs as distribution to Ubuntu users, while the 0.4.0 requirement in `docs/PLAN.md` covers Ubuntu and Debian.[5] One signed R2 APT repository serves both and avoids a second publication system.
3. **APT layout and tool:** `stable` suite, `main` component, `amd64` architecture, generated with `apt-ftparchive` and GnuPG. Publish `InRelease` and detached `Release.gpg`, enable by-hash metadata, and retain the three newest stable versions in `pool/`. Clients install `ca-certificates` and `curl`, save the public key as `/usr/share/keyrings/kawaro-archive.asc`, and use deb822 with `Signed-By` naming that file, never `trusted=yes` or an unauthenticated install option.
4. **COPR targets:** the exact x86_64 chroots are `fedora-43-x86_64`, `fedora-44-x86_64`, and `fedora-45-x86_64`. The live COPR API exposes all three and does not expose `fedora-41-x86_64`.[2] Rawhide is not a supported target. Replace the existing `fedora:41` and `fedora:latest` package-test entries with `fedora:43`, `fedora:44`, and `fedora:45`; none of the old entries remains an acceptance target.
5. **COPR source:** build an SRPM from the tagged source with locked, vendored Cargo dependencies and an offline `%build`. Do not repack the Ubuntu-built binary RPM. COPR supports SRPM and SCM source builds and signs its resulting repositories with the project key.[3]
6. **Stable repository rule:** APT and COPR publication runs only for a stable `vX.Y.Z` tag in `M7MMAD-OMAR/kawaro`, after package tests and the GitHub release succeed. A prerelease tag still gets signed release checksums but does not enter the stable repositories. Existing beta in-app update behavior is unchanged.
7. **Release-file signing:** keep minisign as the updater trust root. Additionally sign the existing `SHA256SUMS` with the APT OpenPGP key as `SHA256SUMS.asc`, and attach that signature plus `kawaro-archive.asc` to every `v*` release. The signed checksum list authenticates every RPM, DEB, zip, and MSI by its SHA-256. Separate ad hoc signatures for each asset are unnecessary. This satisfies issue 3 without weakening either signature system.[1]
8. **Command source:** `packaging/install-commands.json` is the only source of repository install commands. S-016 owns its schema, renderer, and `apt` and `copr` entries. S-078 adds only `aur` and `winget`. README, both install guides, and `tests/release/install_commands.sh` consume the same entries; no test may keep a second command transcription.

## Owner checklist before publication jobs run

No secret value belongs in this story, the repository, workflow logs, or shell history.

- [ ] **GitHub owner `M7MMAD-OMAR`, repository `kawaro`:** keep `release-signing` restricted to `v*` tags, and protect `apt-publishing` and `copr-publishing` so only stable `vX.Y.Z` tags can use them. Enable 2FA on the owner account.
- [ ] **OpenPGP release and APT key:** maintainer generates an Ed25519 or RSA 4096 key on a real machine, keeps an offline backup and revocation certificate, and commits only `packaging/keys/kawaro-archive.asc`. Add its private key and passphrase as `APT_SIGNING_KEY` and `APT_SIGNING_PASSPHRASE` in `release-signing`. Record the 40 hex character fingerprint for the documentation check.
- [ ] **Cloudflare account that owns the `sbarah.com` zone:** create bucket `kawaro-apt`, attach `apt.kawaro.sbarah.com`, leave public development access disabled, and create a token limited to list, read, and write objects in that bucket only. In `apt-publishing`, set secrets `APT_R2_ACCESS_KEY_ID` and `APT_R2_SECRET_ACCESS_KEY`, and variables `APT_R2_ACCOUNT_ID` and `APT_R2_BUCKET=kawaro-apt`.
- [ ] **Fedora Account System user `m7mmad-omar`:** create COPR project `m7mmad-omar/kawaro`, enable exactly the three chroots in decision 4, and enable 2FA. Put the contents of the COPR API config in secret `COPR_CONFIG` in `copr-publishing`; record its expiry and rotate it before expiry. Publish the COPR repository fingerprint in the documentation.

## Execution split

Each substep is at most one work day. Publication steps wait for their owner checklist items.

| Story | Scope | Criteria |
| --- | --- | --- |
| S-016a | Canonical commands, local APT repository builder, signatures, retention, and isolated install tests | AC1 to AC4, AC11 |
| S-016b | R2 publication job and live APT install and upgrade checks | AC5, AC6 |
| S-016c | Source SRPM, vendored offline build, and local COPR-equivalent mock tests | AC7 |
| S-016d | COPR submission job and live Fedora install, signature, and upgrade checks | AC8, AC9 |
| S-016e | Signed release checksums, final bilingual docs, fingerprints, changelog, and gates | AC10 to AC12 |

## Acceptance criteria

- **AC1, APT build:** a reproducible repository builder accepts a DEB and output directory, creates `dists/stable/InRelease`, `Release`, `Release.gpg`, `Packages`, by-hash entries, and `pool/main/`, and verifies both Release signatures with `gpgv` and the committed public key. Building four fixture versions leaves exactly the newest three in the pool and indexes only those three.
- **AC2, isolated APT behavior:** using a temporary key and a local HTTP server, the exact JSON `apt` commands install fixture version N in Ubuntu 22.04, Ubuntu 24.04, and Debian 12 containers. Adding N+1 and rerunning normal `apt-get update && apt-get install -y --only-upgrade kawaro` produces N+1. `kawaro --version` equals the expected package version and `apt-cache policy kawaro` names the test repository.
- **AC3, APT failure closed:** all three containers produce zero `W:` lines, zero `NO_PUBKEY`, zero `not signed`, and zero `skipped OpenPGP checks`. No command uses `trusted=yes`, `--allow-unauthenticated`, or `--allow-downgrades`. A missing key, a repository signed by another key, a modified Release file, and a DEB whose bytes differ from its indexed hash each make update or installation fail nonzero.
- **AC4, signing hygiene:** only the tag jobs in the canonical repository can access real secrets. PRs, forks, and prerelease repository publication receive none. The key is imported into a `mktemp` GNUPGHOME with mode 0700 and removed by an `EXIT` trap. Job output and git history contain no private-key block or passphrase. The public key and its fingerprint match.
- **AC5, live R2 publication:** the job downloads the signed repository artifact produced in `release-signing`, without access to the private key, then uploads pool objects first, metadata and by-hash objects second, and `Release`, `InRelease`, and `Release.gpg` last. Old pool objects are removed only after successful metadata publication, with three versions retained. HTTPS GETs from a fresh runner return 200 for the live Release files and the current DEB, and `gpgv` verifies both downloaded signatures. `r2.dev` is absent from config and docs.
- **AC6, live APT publication:** after the first stable tag, fresh Ubuntu 22.04, Ubuntu 24.04, and Debian 12 containers run the exact documented commands against `https://apt.kawaro.sbarah.com`, install the tag version, and satisfy AC3. After the second stable tag, each container installs the previous version and upgrades to the new tag with the normal package manager. These are live checks, not local fixtures.
- **AC7, COPR source package:** `packaging/linux/kawaro-copr.spec` builds `kawaro` and `kawarod` from the tagged source with `cargo build --release --locked --offline`, stages the same files as the release RPM, and uses `Release: 1%{?dist}`. Its SRPM contains the source and locked vendor dependencies. A mock build with network access disabled succeeds for each target chroot configuration and fails if a dependency is missing from the vendor set.
- **AC8, COPR publication:** the tag job writes `COPR_CONFIG` to a temporary file, submits the SRPM to `m7mmad-omar/kawaro` with `copr-cli`, waits, and fails unless all three exact chroots report `succeeded`. It deletes the temporary config with an `EXIT` trap. A deliberately broken build in a disposable COPR project makes the job fail. A prerelease tag submits nothing.
- **AC9, live COPR publication:** the ordinary RPM package-test matrix and the live matrix both use exactly `fedora:43`, `fedora:44`, and `fedora:45`. Fresh live containers run the exact JSON `copr` commands, install the tag version, and produce zero `skipped OpenPGP checks`. `rpm -q --qf '%{SIGPGP:pgpsig}' kawaro` is not `(none)`, and its key ID matches the published COPR key. After the second stable tag, installation of the previous version followed by `dnf upgrade -y kawaro` reaches the new tag in every container.
- **AC10, authenticated release downloads:** every release produced from a `v*` tag after this story lands has `SHA256SUMS`, `SHA256SUMS.asc`, and `kawaro-archive.asc`. `gpgv --keyring kawaro-archive.asc SHA256SUMS.asc SHA256SUMS` exits 0 and `sha256sum -c SHA256SUMS` verifies all four assets. A wrong public key, a modified checksum line, and a modified asset each fail verification. This job never changes the minisign feed or its keys.
- **AC11, one command source and docs:** `README.md`, `README.ar.md`, `docs/guide/install.md`, and `docs/guide/install.ar.md` show the rendered APT and COPR commands, repository URLs, and both APT and COPR fingerprints. The guides keep their language switch lines. A test compares every rendered block and executed test command with `packaging/install-commands.json`; editing only JSON changes the docs check and the commands under test together.
- **AC12, finish:** add one English and Arabic `CHANGELOG.md` line for S-016. Shell scripts pass ShellCheck, workflow tests prove event, repository, environment, and prerelease guards, and `scripts/design-lint.sh` passes. No changed file contains an em dash or en dash.

## Test cases

| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | Build four small fixture DEB versions; inspect layout, retention, indexes, and both `gpgv` checks | integration |
| AC2, AC3 | Three-container local repository matrix, N to N+1 upgrade, wrong key and tamper cases | integration |
| AC4 | Throwaway-key run captures logs; inspect workflow guards, temp permissions, cleanup, and git history | integration, security |
| AC5, AC6 | After real stable tags, fetch R2 objects and run fresh live container installs and upgrades | live publication |
| AC7 | Build the SRPM and compile offline in each target mock configuration | integration |
| AC8 | Query COPR build and build-chroot APIs until terminal state; include the disposable failure build | integration |
| AC9 | Three live Fedora containers verify version, RPM signature key, warnings, and previous-to-current upgrade | live publication |
| AC10 | Download real release assets, verify signed checksums, then test wrong key and modified inputs | live publication, security |
| AC11 | Parse JSON, compare rendered docs, and execute the same parsed commands in AC2, AC6, and AC9 | unit, integration |
| AC12 | ShellCheck, workflow unit tests, changelog check, and design lint | quality gates |

## Out of scope

- AUR, winget, AppStream, Launchpad, Flatpak, Snap, Microsoft Store, Debian 13, and architectures other than x86_64 and amd64.
- Changes to in-app update policy, minisign keys, or the update feed.
- A Windows code-signing certificate and automatic OpenPGP key rotation.

## Sources

[1] https://github.com/M7MMAD-OMAR/kawaro/issues/3 : Kawaro issue 3
[2] https://copr.fedorainfracloud.org/api_3/mock-chroots/list : COPR mock chroots API
[3] https://docs.copr.fedorainfracloud.org/user_documentation.html : COPR user documentation
[4] https://developers.cloudflare.com/r2/data-access/public-buckets : Cloudflare R2 public buckets
[5] https://documentation.ubuntu.com/launchpad/user/reference/packaging/ppas/ppa : Launchpad PPA documentation
