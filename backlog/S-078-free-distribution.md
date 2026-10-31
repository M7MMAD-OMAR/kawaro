# S-078 AppStream, AUR, and winget distribution

Status: the account free parts of S-078a (AppStream) and the command source are planned in [S-112](S-112-appstream-and-install-commands.md) (approved, 2026-10-31).
Previous status: in progress (2026-10-27). The winget half (S-078d) is submitted and validated; the AppStream half (S-078a) and the AUR half (S-078b to S-078c) are not started, and the PR automation (S-078e) is not built.

## Progress log

- 2026-10-27, the winget half (S-078d): the 0.4.0 manifests for `Sbarah.Kawaro` are open for review at [microsoft/winget-pkgs#462805](https://github.com/microsoft/winget-pkgs/pull/462805). All twelve automated checks pass, including the installation validation, which Microsoft runs as a standard non-elevated user. `winget validate --manifest` exits 0 and `winget install --manifest` completes silently on Windows 11 and reports `kawaro 0.4.0`. The remaining gate is one approving review from a Microsoft moderator; a submitted PR never satisfies AC7. The publisher-controlled evidence is on the pull request as a comment, and the research behind the merge requirements is in `docs/winget-pkgs-merge-requirements.md`.
- 2026-10-27, the scope finding: `wixl` accepts `InstallScope="perUser"` and drops it, so the built MSI carried no `ALLUSERS` property and an elevated install would register the program under the machine hive, where winget does not look for a package that declares `Scope: user`. The `.wxs` now writes the two per-user properties explicitly, and `scripts/release/check-winget-scope.sh` keeps the manifest and the installer in agreement. This changes the next MSI build, not the submitted 0.4.0 manifest.
- 2026-10-27, the DisplayVersion finding: every manifest of the package must make the same choice about `AppsAndFeaturesEntries.DisplayVersion`, or the Microsoft catalog cannot map the installed version to an available one. `scripts/release/check-winget-versions.sh` refuses the mixed set, and `tests/release/s078_winget_versions.sh` covers both shapes and the mixed one.
- Not started: S-078a (the AppStream stock icon, the screenshots and the bilingual release entries), S-078b and S-078c (the AUR package and its publication), S-078e (the idempotent pull request automation), and the S-078f half of the merged-catalog checks. The owner accounts and credentials are still the gate for the AUR half (exclusion R4-accounts).

## Context

The package already installs an AppStream metainfo file and Kawaro icons, but the metainfo has no icon declaration or screenshots and its release descriptions are English only. The GitHub release already provides a per-user MSI. ADR 0006 and S-054a to S-054c already provide signed in-app updates, so these channels complement an existing update path rather than creating one.

S-078 owns AppStream metadata, AUR, and winget only. S-016 owns APT, COPR, OpenPGP release authentication, and the shared command-source schema.

## Story

As an Arch Linux or Windows user, I want to install and upgrade Kawaro through a familiar free community package channel, and as a Linux app-store user I want to recognize Kawaro by its real icon, screenshots, and bilingual release information.

## Manager decisions

1. **Channels (amended 2026-10-30 by the manager: an AUR `kawaro-bin` channel is wanted and is built by S-103e; "do not create `kawaro-bin`" below no longer holds):** publish source-built AUR package `kawaro` and winget package `Sbarah.Kawaro` from stable `vX.Y.Z` tags. Do not create `kawaro-bin` or a beta AUR package in this story. Prerelease tags neither publish nor open a winget PR. The winget identifier prefix is the `Sbarah` namespace, the same one the AppStream and site channels use; the `Publisher` field carries the product name `Kawaro` because it must match the Add or remove programs entry the MSI writes.
2. **AUR form:** `PKGBUILD` downloads the tagged GitHub source archive, verifies its SHA-256, fetches locked Cargo dependencies, and builds with `cargo build --release --locked --offline`. AUR requires an SSH key for write access and requires regenerated `.SRCINFO` with each metadata update.[8]
3. **winget form:** generate version, installer, default-locale, and Arabic-locale manifests for the published x64 MSI. Use schema `ManifestVersion: 1.12.0`, the current documented example, with the real GitHub asset URL and SHA-256, `InstallerType: msi`, user scope, and the MSI product code. `InstallerSwitches` is left out because the client supplies the MSI silent switches itself.[6] CI opens or updates a PR to `microsoft/winget-pkgs`; it never merges its own PR. Microsoft validates submissions and may review them manually.[7]
4. **Publication is not a file upload:** AUR succeeds only when its live RPC metadata and clean install match the tag. winget succeeds only when the PR is merged, the community catalog shows the version, and Windows Sandbox installs it. A submitted but unmerged PR leaves S-078 open.
5. **AppStream:** add stock icon `kawaro`, at least one default screenshot with English and Arabic captions, and bilingual descriptions for every release entry. AppStream supports stock icons and release entries in desktop-application metadata.[9] Screenshot files use HTTPS URLs on `kawaro.sbarah.com` and are checked over the network in a separate live test.
6. **One command source:** extend S-016's `packaging/install-commands.json` with only `aur` and `winget` entries. The existing renderer and `tests/release/install_commands.sh` must consume those entries for README checks, both install guides, and live tests. No command is copied independently into a test.
7. **Documentation paths:** the user guide files are exactly `docs/guide/install.md` and `docs/guide/install.ar.md`, each with its language switch line. README and `README.ar.md` link to those guides rather than maintaining a second AUR or winget command block.

## Owner checklist before publication jobs run

No secret value belongs in this story, the repository, workflow logs, or shell history.

- [ ] **GitHub owner `M7MMAD-OMAR`:** enable 2FA, create fork `M7MMAD-OMAR/winget-pkgs` from `microsoft/winget-pkgs`, and protect environments `aur-publishing` and `winget-publishing` for stable `vX.Y.Z` tags only.
- [ ] **winget token:** create a classic GitHub personal access token with only the `public_repo` scope, without `delete_repo`, and an expiry of at most 90 days. WingetCreate currently requires a classic token and rejects fine-grained tokens.[10] Store it as `WINGET_CREATE_GITHUB_TOKEN` in `winget-publishing`, pass it only through the environment variable of the same name, never through `--token`, and record the expiry for rotation.
- [ ] **AUR account (the maintainer's):** create or confirm the account, register a dedicated Ed25519 SSH public key, and reserve pkgbase `kawaro`. Store only the private key as `AUR_SSH_KEY` in `aur-publishing`. Commit the separately verified public host-key entry as `packaging/aur/known_hosts`; do not disable host-key checking.
- [ ] **Site owner:** place the referenced AppStream screenshot assets on `kawaro.sbarah.com` through the existing Cloudflare site deployment. No additional Cloudflare token is needed in the AUR or winget jobs.

## Execution split

Each substep is at most one work day. External review wait time is not implementation work and does not count as a passed live check.

| Story | Scope | Criteria |
| --- | --- | --- |
| S-078a | AppStream icon, screenshots, bilingual releases, packaged-tree validation, and live screenshot checks | AC1 |
| S-078b | AUR command entry, generator, and offline artifact and install validation | AC2, AC3 |
| S-078c | AUR tag publication and live install and upgrade | AC4 |
| S-078d | winget command entry, manifest generation, and Windows validation | AC2, AC5 |
| S-078e | Idempotent winget PR automation and credential guards | AC6 |
| S-078f | Merged-catalog winget checks, final bilingual guide links, secret review, changelog, and gates | AC7, AC8 |

## Acceptance criteria

- **AC1, AppStream:** `packaging/io.github.m7mmad_omar.Kawaro.metainfo.xml` declares `<icon type="stock">kawaro</icon>`, a default screenshot with English and Arabic captions and explicit image dimensions, and English and Arabic descriptions for every `<release>`. `appstreamcli validate --no-net` and `appstreamcli validate-tree --no-net` on the staged package tree exit 0. A test resolves the installed stock icon to a real staged Kawaro icon. Every screenshot URL returns HTTPS 200 with an image content type and the declared dimensions in a live check.
- **AC2, canonical commands and guides:** JSON entries produce an Arch command that clones `https://aur.archlinux.org/kawaro.git` and runs `makepkg -si`, and a Windows PowerShell command that runs `winget install --id Sbarah.Kawaro --exact`. Both guide paths render those exact commands and retain their language switch lines. README tests parse the same JSON; a deliberate mismatch between a rendered command and JSON fails the test.
- **AC3, AUR artifact:** for a stable tag, CI generates `PKGBUILD` and `.SRCINFO` with pkgbase and pkgname `kawaro`, the tag version, the tagged source URL, its actual SHA-256, and x86_64 architecture. `makepkg --printsrcinfo` reproduces `.SRCINFO` byte for byte. `makepkg --verifysource` verifies the source, and a clean `archlinux:base-devel` container runs `makepkg -si --noconfirm` and gets the expected `kawaro --version`. The build uses the locked dependency set and succeeds with network disabled after source fetching.
- **AC4, live AUR publication:** only a stable tag in `M7MMAD-OMAR/kawaro` can use `aur-publishing`. CI commits generated files to a fresh AUR clone and pushes to `master` without force, then `git ls-remote` matches the pushed commit and `https://aur.archlinux.org/rpc/v5/info/kawaro` reports the tag version. A clean live container installs that version from AUR. After the second stable tag, installing the previous AUR version and then running the new canonical command upgrades to the new tag.
- **AC5, winget artifact:** generated manifests identify `Sbarah.Kawaro`, the stable tag version without `v`, the exact published MSI URL and SHA-256, GPL-3.0-or-later, English and Arabic metadata, x64, per-user scope, and the actual product code. The client supplies the MSI silent switches, so the manifest declares none. `winget validate --manifest <generated-dir>` exits 0 on Windows. Before submission, Windows Sandbox installs from the manifest silently and `kawaro --version` matches the tag. The unsigned-package warning remains documented; this story does not bypass SmartScreen or claim a trusted publisher signature. Every manifest of the package also agrees with the DisplayVersion the MSI writes: either all of them name the release version and declare no `AppsAndFeaturesEntries.DisplayVersion`, or all of them declare their own. `scripts/release/check-winget-versions.sh` exits 0 on the generated set, and `tests/release/s078_winget_versions.sh` covers both shapes and the mixed one. A manifest that declares `Scope: user` also has an MSI that is per-user by construction, with `ALLUSERS=2` and `MSIINSTALLPERUSER=1` written into the MSI, so the Add or remove programs entry lands under the user's own hive for an elevated install too; `scripts/release/check-winget-scope.sh` exits 0 on the generated set and `tests/release/s078_winget_scope.sh` covers the per-user MSI and the one without the property.
- **AC6, winget PR automation:** after the GitHub release exists, a stable-tag job in `winget-publishing` uses `wingetcreate` or the GitHub API to open one PR from the owner fork to `microsoft/winget-pkgs`. WingetCreate receives the credential only through `WINGET_CREATE_GITHUB_TOKEN`, never a command argument. Re-running the same tag updates the existing branch and PR instead of creating a duplicate. PRs, forks, prereleases, and unrelated repositories cannot use the token. Logs contain neither token nor private key material.
- **AC7, live winget publication:** after merge, `winget show --id Sbarah.Kawaro --exact --versions` lists the tag version. A fresh Windows Sandbox runs the canonical install command with source and license agreements accepted, then `kawaro --version` matches the tag. After a second stable version is merged, installing the previous catalog version and running `winget upgrade --id Sbarah.Kawaro --exact` reaches the new version. PR creation alone never satisfies this criterion. The version the installer writes to the registry is the one that maps an installed package to an available version, so the second release carries its own `AppsAndFeaturesEntries.DisplayVersion` in every manifest of the package, the 0.4.0 one included, and `winget upgrade` shows the newer version as available rather than looping.
- **AC8, finish:** add one English and Arabic `CHANGELOG.md` line for S-078. Shell scripts pass ShellCheck, workflow tests prove stable-tag, repository, environment, host-key, and token guards, and `scripts/design-lint.sh` passes. No changed file contains an em dash or en dash.

## Test cases

| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | Validate XML and staged tree, resolve the stock icon, compare bilingual release entries, and fetch live screenshots | unit, integration, live |
| AC2 | Parse JSON, compare both rendered guides and README test inputs, and inject a mismatch | unit |
| AC3 | Generate from two fixture tags, compare `.SRCINFO`, verify source hashes, and build and install in a clean Arch container | integration |
| AC4 | On the first and second real stable tags, check the pushed commit and AUR RPC, then run clean N to N+1 installs | live publication |
| AC5 | Inspect all manifest fields and hashes; run `winget validate` and a pre-submission Sandbox install; build a real MSI with wixl and check every manifest of the set against its DisplayVersion, including the mixed set, and check that a `Scope: user` manifest has a per-user MSI | integration, Windows |
| AC6 | Mock first submission and same-tag rerun; assert one branch and PR, stable-only guards, and no secrets in output | integration, security |
| AC7 | Query the merged community catalog, then install and upgrade exact versions in fresh Windows Sandboxes | live publication |
| AC8 | ShellCheck, workflow unit tests, changelog check, and design lint | quality gates |

## Out of scope

- APT, COPR, Launchpad, Flatpak, Flathub, Snap, Microsoft Store, macOS, and additional architectures.
- Changes to the existing in-app updater, its minisign feed, or repository signing keys.
- A Windows code-signing certificate, automatic merging of winget PRs, and beta channel publication.

## Sources

[6] https://learn.microsoft.com/en-us/windows/package-manager/package/manifest : WinGet manifest documentation
[7] https://learn.microsoft.com/en-us/windows/package-manager/package/repository : WinGet repository submission
[8] https://wiki.archlinux.org/title/AUR_submission_guidelines : AUR submission guidelines
[9] https://www.freedesktop.org/software/appstream/docs/sect-Metadata-Application.html : AppStream desktop application metadata
[10] https://github.com/microsoft/winget-create/blob/main/doc/token.md : WingetCreate GitHub token requirements
