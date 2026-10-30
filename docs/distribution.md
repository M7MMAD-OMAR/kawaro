[English](distribution.md) | [العربية](distribution.ar.md)

# How Kawaro is distributed

This page explains how a Kawaro release reaches users, which command installs it from each channel, and how a maintainer adds a channel or fixes a bad release. Nothing here is a promise that a channel is live: the state of each one is in the table in [distribution-status.md](distribution-status.md).

## 1. How a release travels

```
git tag vX.Y.Z  ->  CI build  ->  release assets + SHA256SUMS  ->  signed update manifest  ->  channel jobs
```

1. **Tag.** The maintainer pushes a tag `vX.Y.Z`. A tag with a hyphen, such as `v0.5.0-beta.1`, is a prerelease.
2. **CI build.** The `Packages` workflow builds the RPM and DEB, the Windows ZIP and MSI, and installs the Linux packages in clean containers as a smoke test. Windows files are signed in the protected environment `release-signing`.
3. **Release assets and checksums.** The `release` job publishes exactly four files (RPM, DEB, ZIP, MSI) and a `SHA256SUMS` file to the GitHub release. It stops if the count is not four. Every channel takes its hash from this `SHA256SUMS`, never from a second source.
4. **Signed update manifest.** The `update-manifest` job writes `stable.json` and `beta.json` and signs them with a minisign key (see [update-signing.md](update-signing.md)). The app updates itself from this feed. A prerelease only changes the beta feed.
5. **Channel jobs.** When the release and the manifest are done, the `distribute` job calls `.github/workflows/release-distribute.yml`. It runs one job per channel: `winget`, `scoop`, `copr`, `aur`, `apt`. The jobs are independent: a failure in one does not stop the others. Then a `status` job renders the table for the run.

The pipeline never pushes to `main`. The generated table is shown in the run summary; copying it into `docs/distribution-status.md` is a normal reviewed change.

## 2. Channels and the one command

Replace `X.Y.Z` with the version. `OWNER` is the account that owns the channel repository or project. A channel that is not set up yet cannot be used, and the table says so.

| Channel | Command | State today |
| --- | --- | --- |
| Windows direct download | Download `kawaro-X.Y.Z-windows-x64.msi` from the [releases page](https://github.com/M7MMAD-OMAR/kawaro/releases/latest) and double click it | Available. Test signed, so SmartScreen warns ([details](windows-smartscreen.md)) |
| Linux direct download, Fedora | `sudo dnf install ./kawaro-*.x86_64.rpm` | Available |
| Linux direct download, Ubuntu and Debian | `sudo apt install ./kawaro_*_amd64.deb` | Available |
| WinGet | `winget install Sbarah.Kawaro` | Not usable yet. It waits for the first manifest pull request ([winget-pkgs#462805](https://github.com/microsoft/winget-pkgs/pull/462805)) to be merged. The channel job stays off until then |
| Scoop | `scoop bucket add kawaro https://github.com/OWNER/scoop-kawaro` then `scoop install kawaro/kawaro` | Needs the bucket repository and its token. Installs the portable ZIP, so there is no Explorer menu entry |
| Fedora COPR | `sudo dnf copr enable OWNER/kawaro` then `sudo dnf install kawaro` | Needs the COPR project and its API config |
| Arch AUR | `yay -S kawaro-bin` | Needs the first registration of the package by hand |
| apt (Debian, Ubuntu, Kali) | Add the keyring and the source file, then `sudo apt update && sudo apt install kawaro` (full lines in [apt.md](distribution-channels/apt.md)) | Needs the signing key and storage. Skips until the public key is committed |
| Flathub | None | Not planned for now. The decision is NO-GO: the right click entry cannot ship inside the sandbox and LibreOffice cannot run in it ([assessment](distribution-channels/flathub.md)) |
| Chocolatey | None | Planned, not built |
| Homebrew (Linuxbrew tap) | None | Planned, not built |

Commands are written by hand in the per channel pages under [distribution-channels](distribution-channels/README.md). The planned file `packaging/install-commands.json` that would render them does not exist yet, so the commands above must be checked against those pages when either changes.

After any install, verify a direct download with `sha256sum -c SHA256SUMS --ignore-missing` (Linux) against the `SHA256SUMS` of the same release.

## 3. The channel contract

Each channel is one script, `scripts/channels/NAME.sh`, started as `scripts/channels/NAME.sh vX.Y.Z [--dry-run]` from the repository root. `scripts/channels/run-all.sh` runs the scripts it finds and records the result. The rules:

- **Dry run.** `--dry-run` prints `DRYRUN NAME: would publish X.Y.Z` and changes nothing outside the machine. If a name is missing it also prints a `NOTE` line.
- **Exit codes.** `0` means success. `10` means a clean skip: the channel is not configured, or the tag is a prerelease. The script prints one line such as `SKIP NAME: VARIABLE not configured`. Any other code is a failure.
- **Isolation.** One failing channel never blocks another. In CI each channel is its own matrix job with `fail-fast: false`, in its own environment `NAME-publishing`. Locally `run-all.sh` runs each script as a separate process.
- **Idempotency.** Running the same tag twice must not publish twice. Scoop, AUR and apt print `UNCHANGED NAME: X.Y.Z` and create nothing on a rerun. COPR does not report this yet: a rerun submits another build of the same version.
- **Secrets by environment only.** A script reads secrets from environment variables, never from arguments or URLs, and never prints them. Temporary key files are removed by an `EXIT` trap.
- **Status table.** `run-all.sh` writes a table (channel, state, version, checked date) between the markers `<!-- channels:start -->` and `<!-- channels:end -->` in `docs/distribution-status.md`. `ok` means the script succeeded, not that the registry accepted the package.

## 4. Adding a channel

1. Write `scripts/channels/NAME.sh` following section 3. Start from the closest existing script: `scoop.sh` for a repository push, `aur.sh` for an SSH push, `copr.sh` or `apt.sh` for a build service.
2. Generate files from the release `SHA256SUMS`, not from a hand typed hash. Keep templates and a generator in `packaging/NAME/`.
3. Add `NAME` to the matrix in `.github/workflows/release-distribute.yml`, and pass every secret and variable the script reads in that step's `env:`. Pin any new action by full commit SHA and any downloaded tool by version and SHA-256.
4. Create the environment `NAME-publishing` (owner action, section 6).
5. Write `docs/distribution-channels/NAME.md`: the user command, what the channel installs, names only for secrets, owner actions, what is not verified yet.
6. Add a test `tests/release/s103x_NAME.sh` that runs offline with stubs and checks the golden output, a wrong hash that must fail, the dry run, the clean skip, the rerun, and that no secret appears in output. Add it to `tests/release/run_all.sh`.
7. Run `tests/release/s103a_env_drift.sh`. It fails if the workflow and the script disagree on a variable name or if a script is missing from the matrix.
8. Record the licence of any new tool in `docs/engines.md`.

## 5. Secrets and environments

Names only. No value belongs in the repository, an issue, a log or a chat.

Create one environment per channel: `winget-publishing`, `scoop-publishing`, `copr-publishing`, `aur-publishing`, `apt-publishing`. In each, limit deployment to the tag pattern `v*`. Put secrets in the environment, never at repository level: the workflow lists every channel's secret names, and only environment scoping stops one channel from seeing another's secrets. The release itself uses the separate environment `release-signing`.

| Channel | Secrets | Variables |
| --- | --- | --- |
| winget | `WINGET_CREATE_GITHUB_TOKEN` | `CANONICAL_REPO` |
| scoop | `SCOOP_BUCKET_TOKEN` | `BUCKET_REPO`, `CANONICAL_REPO` |
| copr | `COPR_CONFIG` | `COPR_PROJECT`, `CANONICAL_REPO` |
| aur | `AUR_SSH_KEY` | `CANONICAL_REPO` |
| apt | `APT_SIGNING_KEY`, `APT_SIGNING_PASSPHRASE`, `APT_R2_ACCESS_KEY_ID`, `APT_R2_SECRET_ACCESS_KEY` | `APT_R2_ACCOUNT_ID`, `APT_R2_BUCKET`, `CANONICAL_REPO` |
| release signing | `UPDATE_SIGNING_KEY`, `UPDATE_SIGNING_PASSWORD`, and the Windows signing certificate secrets | none listed here |

`APT_R2_ACCOUNT_ID` and `APT_R2_BUCKET` are plain variables, not secrets (the account id is not confidential); `APT_SIGNING_PASSPHRASE` is an optional secret, needed only if the key has a passphrase. `CANONICAL_REPO` is `OWNER/NAME` of this project. The channel jobs run only when it equals the current repository, so a fork never publishes.

## 6. OWNER ACTIONS

These steps need a human with the right accounts. The pipeline cannot do them.

1. **Environments.** Create the five `NAME-publishing` environments and restrict each to `v*` tags. Set `CANONICAL_REPO`.
2. **WinGet.** Wait until the first manifest pull request is merged. Then create a fork of the community repository and a classic token with only `public_repo`, and store it as `WINGET_CREATE_GITHUB_TOKEN`. Before the merge the secret stays unset, because `komac update` needs the package to exist.
3. **Scoop.** Create the public repository `OWNER/scoop-kawaro` and a fine grained token limited to it with contents write. Store it as `SCOOP_BUCKET_TOKEN` and set `BUCKET_REPO`.
4. **COPR.** Create the COPR project, enable the three Fedora chroots, create the API config and store it as `COPR_CONFIG`. Set `COPR_PROJECT`. Record the config's expiry.
5. **AUR.** Register an AUR account and add a dedicated SSH public key. Check the AUR host keys against the fingerprints on the AUR site and commit them as `packaging/aur/known_hosts`. Register `kawaro-bin` once by hand (first push). Store the private key as `AUR_SSH_KEY`.
6. **apt.** On a trusted machine create a signing key with the generic user id `Kawaro release signing`, a passphrase, an offline backup and a revocation certificate. Commit only the public key as `packaging/keys/kawaro-archive.asc` and note its fingerprint in the README. Create the object storage bucket and its public domain, create a token limited to that bucket, and set the `APT_R2_*` names. Store the private key as `APT_SIGNING_KEY`.
7. **After the first stable tag.** Install from every channel in a clean machine or container and report the result.
8. **Rotation.** Rotate every token and config before it expires. Rotating the apt signing key means a new committed public key and a new keyring file for users.
9. **Later.** Flathub only after a GO decision. Chocolatey and Homebrew need an account (Chocolatey is moderated) and the same recipe as Scoop.

## 7. Reviewing a pipeline change

Check each item before merging a change to the workflows or to `scripts/channels/`.

- **Privacy grep.** Search the diff for personal names, email addresses, home directory paths, machine names, and key or token text. Only the public repository URL may appear. Use `OWNER` and `ORG` placeholders.
- **No long dashes.** Search for the em dash and the en dash.
- **Secret handling.** A secret is read from the environment only, never put in a URL, argument or log, and any temporary file with key material is mode 0600 or tighter and removed by an `EXIT` trap. Steps that hold secrets run with `set +x`. No secret is defined at repository level.
- **Least privilege.** `permissions: {}` at the top, `contents: read` per job, no `pull_request_target`, the tag validated before it is used, and the tag reaches the shell only through `env:`.
- **Pinned actions.** Every action is pinned by full commit SHA with the version in a comment. Downloaded tools are pinned by version and SHA-256, and Python tools by hash.
- **Dry run first.** Run the new or changed script with `--dry-run`, then the workflow with `workflow_dispatch` and `dry_run` left on, before any real run.
- **Tests.** Run the `tests/release` tests, including `s103a_env_drift.sh`, plus `shellcheck` on every script and `scripts/design-lint.sh`.
- **Honest docs.** Anything not verified live is written as not verified.

## 8. Troubleshooting and rollback

**Re-run one channel.** Open the workflow `release-distribute` in the Actions tab, choose Run workflow, enter the tag (for example `v0.5.0`), and set `dry_run` to false. A dispatch runs every channel; a channel that already published prints `UNCHANGED` (Scoop, AUR, apt) or, for COPR, submits a new build. The command line equivalent for one channel is `scripts/channels/run-all.sh vX.Y.Z --only NAME` with the channel's environment set. Always try `dry_run` first; the dispatch default is on. In the Run workflow form, "Use workflow from" must also be the tag (for example `v0.5.0`), not a branch: each environment is limited to `v*` tags, so a dispatch from a branch is rejected.

**Reading a result.** `SKIP` means the channel is not configured, or the tag is a prerelease. A red job is a failure of that channel only. The `status` job prints the table in the run summary.

**Yanking a bad release.** There is no automatic yank. Fix forward when you can: publish a higher version, because channels only move forward. When you must withdraw a version:

- **GitHub release.** Edit it to a draft or delete the faulty assets. Also remove the bad version from the update feed, so the app does not offer it. Both are manual steps for the owner.
- **WinGet.** Open a pull request to the community repository that removes the version, or ask the moderators. Users who already installed it keep it.
- **Scoop.** Revert the commit in the bucket repository; users get the old manifest on the next `scoop update`.
- **COPR.** Delete the build in the project (web page or `copr-cli delete-build`).
- **AUR.** Push a new commit with the previous good `PKGBUILD` and `.SRCINFO`. History is never rewritten.
- **apt.** There is no yank script yet. The repository keeps the 3 newest versions; a bad one needs a manual removal and a re-signed index. The safe path is a fixed higher version.

Update this page when a yank step becomes automatic.
