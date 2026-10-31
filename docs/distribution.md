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

Replace `X.Y.Z` with the version. A channel that is not set up yet cannot be used, and the table says so.

| Channel | Command | State today |
| --- | --- | --- |
| Windows direct download | Download `kawaro-X.Y.Z-windows-x64.msi` from the [releases page](https://github.com/M7MMAD-OMAR/kawaro/releases/latest) and double click it | Available. Test signed, so SmartScreen warns ([details](windows-smartscreen.md)) |
| Linux direct download, Fedora | `sudo dnf install ./kawaro-*.x86_64.rpm` | Available |
| Linux direct download, Ubuntu and Debian | `sudo apt install ./kawaro_*_amd64.deb` | Available |
| WinGet | `winget install Sbarah.Kawaro` | Not usable yet. [winget-pkgs#462805](https://github.com/microsoft/winget-pkgs/pull/462805) passed all automated checks and waits for a volunteer moderator since 2026-10-27. The channel job stays off until the package exists |
| Scoop | `scoop bucket add kawaro https://github.com/M7MMAD-OMAR/scoop-kawaro` then `scoop install kawaro/kawaro` | Live since 0.4.3. Installs the portable ZIP, so there is no Explorer menu entry |
| Fedora COPR | `sudo dnf copr enable ACCOUNT/kawaro` then `sudo dnf install kawaro` (`ACCOUNT` is the COPR account, not created yet) | Not usable yet. Waits for a COPR account (platform limit, section 6) |
| Arch AUR | `yay -S kawaro-bin` | Not usable yet. Waits for an AUR account (platform limit, section 6) |
| apt (Debian, Ubuntu, Kali) | One line that adds the keyring and the source, then installs: see [apt.md](distribution-channels/apt.md). The repository is `https://m7mmad-omar.github.io/kawaro-apt` | Live since 0.4.3, signed, served by GitHub Pages |
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
4. Create the environment `NAME-publishing` with a `v*` tag rule (section 5 shows the two API calls).
5. Write `docs/distribution-channels/NAME.md`: the user command, what the channel installs, names only for secrets, what is not verified yet.
6. Add a test `tests/release/s103x_NAME.sh` that runs offline with stubs and checks the golden output, a wrong hash that must fail, the dry run, the clean skip, the rerun, and that no secret appears in output. Add it to `tests/release/run_all.sh`.
7. Run `tests/release/s103a_env_drift.sh`. It fails if the workflow and the script disagree on a variable name or if a script is missing from the matrix.
8. Record the licence of any new tool in `docs/engines.md`.

## 5. Secrets and environments

Names only. No value belongs in the repository, an issue, a log or a chat.

The five environments `winget-publishing`, `scoop-publishing`, `copr-publishing`, `aur-publishing` and `apt-publishing` exist, and each allows deployment only from the tag pattern `v*`. A new environment is made with two calls: `gh api -X PUT repos/OWNER/REPO/environments/NAME-publishing` with the body `{"deployment_branch_policy":{"protected_branches":false,"custom_branch_policies":true}}`, then `gh api -X POST repos/OWNER/REPO/environments/NAME-publishing/deployment-branch-policies -f name='v*' -f type=tag`. Secrets are set in the environment (`gh secret set NAME --env NAME-publishing < file`), never at repository level: the workflow lists every channel's secret names, and only environment scoping stops one channel from seeing another's secrets. The release itself uses the separate environment `release-signing`.

| Channel | Secrets | Variables |
| --- | --- | --- |
| winget | `WINGET_CREATE_GITHUB_TOKEN` | `CANONICAL_REPO` |
| scoop | `SCOOP_DEPLOY_KEY` | `BUCKET_REPO`, `CANONICAL_REPO` |
| copr | `COPR_CONFIG` | `COPR_PROJECT`, `CANONICAL_REPO` |
| aur | `AUR_SSH_KEY` | `AUR_ENABLED`, `CANONICAL_REPO` |
| apt | `APT_SIGNING_KEY`, `APT_SIGNING_PASSPHRASE`, `APT_DEPLOY_KEY` | `APT_REPO`, `CANONICAL_REPO` |
| release signing | `UPDATE_SIGNING_KEY`, `UPDATE_SIGNING_PASSWORD`, and the Windows signing certificate secrets | none listed here |

`SCOOP_DEPLOY_KEY` and `APT_DEPLOY_KEY` are ssh deploy keys, each registered with write access on one public repository only (`scoop-kawaro` and `kawaro-apt`), so a leaked key can touch nothing else. The channel scripts push over ssh with `StrictHostKeyChecking=yes` and GitHub's published host keys, committed in `packaging/release/github_known_hosts`. `APT_SIGNING_PASSPHRASE` is deliberately unset: the apt signing key has no passphrase and lives only in the protected environment and a private backup (details in [apt.md](distribution-channels/apt.md)). `AUR_SSH_KEY` is set already; `AUR_ENABLED` is not, which keeps the AUR channel a clean skip. `CANONICAL_REPO` is `OWNER/NAME` of this project. The channel jobs run only when it equals the current repository, so a fork never publishes.

## 6. Platform limits

Everything the pipeline can do without a person is done. Three items remain, and each is outside what automation may do.

- **AUR.** An AUR account has to be created by a person, because its sign up uses a CAPTCHA. Once it exists, paste the public key of the pair generated for this project into the account and run `gh variable set AUR_ENABLED --body true`. The key, the verified host keys and the package files are ready ([aur.md](distribution-channels/aur.md)).
- **COPR.** A COPR account needs a Fedora Account System login that a person has to create. Once it exists, store the API config with `gh secret set COPR_CONFIG --env copr-publishing < copr-config-file` and set `COPR_PROJECT` with `gh variable set COPR_PROJECT --body ACCOUNT/kawaro` ([copr.md](distribution-channels/copr.md)).
- **WinGet.** The first manifest pull request ([winget-pkgs#462805](https://github.com/microsoft/winget-pkgs/pull/462805)) waits for a Microsoft moderator. The channel stays off until it is merged.

Flathub is a NO-GO for now ([assessment](distribution-channels/flathub.md)). Chocolatey and Homebrew are planned and follow the same recipe as Scoop. Rotating an expiring token or config is routine; rotating the apt signing key means a new committed public key and a new keyring file for users.

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

**Publish by hand.** A tag that predates `release-distribute.yml` cannot be dispatched, because a dispatch runs the workflow file of the chosen tag and that file does not exist there. The channel scripts run the same way locally: put the secrets of that one channel in the environment of one command, for example `BUCKET_REPO=OWNER/scoop-kawaro SCOOP_DEPLOY_KEY="$(cat key-file)" CANONICAL_REPO=OWNER/kawaro scripts/channels/scoop.sh vX.Y.Z --dry-run`, then without `--dry-run`. Version 0.4.3 was published to Scoop and apt this way.

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
