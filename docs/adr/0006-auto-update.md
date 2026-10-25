# ADR 0006: In-app updates for Linux and Windows

Status: accepted
Date: 2026-10-24

## Context

The owner requires that every user can update Kawaro from inside the app, easily and automatically, without leaving it to download a file by hand. This is part of the beta for Linux and Windows. The limits from `CLAUDE.md` stay: no telemetry, no processing servers, no file leaves the machine, a light base install, and no window that takes focus for background work.

Facts read in the repository on 2026-10-24 (verified):

- `Cargo.lock` has no HTTP or TLS crate and no signature crate. It does have `semver` 1.0.28 (MIT OR Apache-2.0), `sha2` 0.11.1, `base64` 0.23.1 and `serde_json`, and the core crate already depends on `serde_json`.
- `packages.yml` builds the RPM and DEB once on ubuntu-22.04 and attaches them to the GitHub release on a `v*` tag. The release files are unsigned. The package version is `Cargo.toml` `version` verbatim, which is illegal for RPM when it holds a hyphen (`0.4.0-beta.1`) and sorts above the final release in dpkg.
- There is no code signing certificate. The Windows MSI (S-051, ADR 0005) and the Windows daemon build, tray and installer (S-045 to S-051) do not exist yet. Windows steps below are written against ADR 0005 and are unverified on a real machine.
- The tray, notifications and `kawarod: notify:` log lines exist on Linux (`crates/daemon/src/{tray,notify}.rs`). Settings live in `config.toml` (`crates/core/src/config.rs`), where newer keys use an underscore and an invalid value sends the whole file back to defaults with a warning.

## Decisions

### 1. Trust root: an embedded signing key

Without a code signing certificate, the trust root is one ed25519 key pair in the minisign format. The public key is committed in `packaging/keys/kawaro-update.pub` and embedded in the binary with `include_str!`. A second slot, `kawaro-update-next.pub`, may be present: a signature from either key is accepted, so a rotation ships inside a normal update. The private key is generated once by the maintainer on a real machine, protected by a passphrase, kept offline, and copied to the GitHub secrets `UPDATE_SIGNING_KEY` and `UPDATE_SIGNING_PASSWORD`. It never enters the repository. Only a tag-triggered job in `M7MMAD-OMAR/kawaro`, bound to the protected environment `release-signing`, can read it; never a pull request or a fork. The trust root must exist before the first updater build ships: a beta built with a placeholder key could never update itself. If the key leaks, in-app recovery is impossible (the attacker holds the same key), so users reinstall by hand and the key is rotated; this limit is documented in `docs/update-signing.md` (S-053c).

The signature library is `minisign-verify` (assumed MIT and dependency free; not in `Cargo.lock` or the local cargo cache, so unverified). S-053a verifies the licence and dependency list with `cargo info` before adding it and records it in `docs/engines.md`. Fallback: `ed25519-dalek` 2.2.0 (BSD-3-Clause, verified in the local cache) with a hand written minisign parser over the `base64` already in the tree; it adds `curve25519-dalek`, `ed25519`, `signature`, `subtle`, `zeroize` and a second `sha2` (0.10).

### 2. The feed: signed pointer files on a rolling release

One extra GitHub release, tag `updates`, is created as a prerelease and never marked latest, so `/releases/latest`, README links and the stable resolution are not affected. It holds `stable.json`, `beta.json` and a `.minisig` for each. CI uploads them after the version's assets are all published, so a manifest never names a missing file. A stable tag updates both pointers; a tag with a prerelease part (`v0.4.0-beta.2`) updates `beta.json` only. Clients fetch `https://github.com/M7MMAD-OMAR/kawaro/releases/download/updates/<channel>.json` and `.minisig`: two plain GET requests, no GitHub API, no rate limit, no JSON from GitHub to trust. The manifest is JSON, schema 1: `schema`, `channel`, `version`, `published`, `notes_url`, and `assets[]` with `kind` (`msi`, `rpm`, `deb`), `os`, `arch`, `name`, `url`, `size`, `sha256`. Asset names and URLs are read from `gh release view --json assets` after upload, never composed, because GitHub may rewrite characters such as `~` in names.

### 3. Client checks, all before anything is applied

Reject, in this order: bad signature; unknown `schema`; `channel` different from the configured one; `version` less than or equal to the installed one (no downgrade, no replay of an old manifest, compared as semver so `0.4.0-beta.1` is below `0.4.0`); an asset `url` that does not start with `https://github.com/M7MMAD-OMAR/kawaro/releases/download/`; no asset for this platform. The download is checked against `size` and `sha256` from the signed manifest. The final host after GitHub's redirect is not pinned, because integrity comes from the signed hash; every hop must be https.

### 4. Transport: the system curl behind a trait

No TLS stack enters the tree. A `Fetcher` trait has one real implementation that runs `curl` by absolute path (`/usr/bin/curl`, then `/bin/curl`; `%SystemRoot%\System32\curl.exe` on Windows 10 1803 and later, which ships it), never found through `PATH`. Arguments are fixed and built by a pure function: https only on every hop, at most 5 redirects, a size cap, a timeout, and the fixed user agent `kawaro-update` with no version, platform or identifier, no cookies. `Recommends: curl` in the RPM and DEB; when it is missing the check reports `unavailable`. Tests use a fake `Fetcher`. Rejected: `ureq` or `reqwest` with rustls, which add `ring` or `aws-lc` and a licence review (ring's OpenSSL-era terms are doubtful with GPLv3) for no gain over curl, since the signed hash carries the trust. A later ADR may swap the implementation without touching callers.

### 5. Schedule, privacy and the off switch

New keys in `config.toml`: `update_check` (bool, default `true`), `update_channel` (`stable` or `beta`, default `stable`), `update_mode` (`ask` or `auto`, default `ask`). With `update_check = false` the daemon makes zero requests; an explicit manual check (`kawaro update --check`, the tray item) still works, because the user asked for it. The daemon checks 30 s after start on a background thread (the hive budget of 150 ms is untouched), then when at least 24 h have passed, polled hourly. A failure does not move the timestamp; retry waits 1 h, then 2 h, then 4 h, up to 24 h. State lives in `update-state.json` next to `config.toml`. The only data GitHub sees is what any download request carries (IP, fixed user agent, URL); the channel is visible in the URL, which is the price of no server of ours.

### 6. Applying on Windows

The package is the per-user MSI of S-051 (stable `UpgradeCode`, major upgrade, relaunches the daemon at its end). Running `msiexec` over a live daemon ends in a pending reboot, so apply copies `kawaro.exe` to a temporary folder, starts the copy detached with `update --apply-msi`, and asks the daemon to quit. The copy waits for the daemon to exit, runs `msiexec /i <msi> /qn /norestart /l*v <log>`, and classifies the exit code: 0 success, 3010 success with a reboot pending, 1602 cancelled, 1603 fatal, 1618 another install running (retry later). On failure the MSI has rolled back by itself; the copy relaunches the old daemon and writes `update-result.json`, which the next daemon turns into one notification. No administrator rights.

### 7. Applying on Linux

Installing a package needs root, and `pkexec` raises the system's authentication dialog, which takes focus. Therefore: `ask` and `auto` both download and verify in the background and post one notification; the install runs only after an explicit "Update now". Auto install exists only on Windows. The plan is pure and decided from facts passed in: a configured Kawaro repository (`*kawaro*.repo` or `kawaro.sources`, S-016) means `pkexec dnf upgrade -y kawaro` or `apt-get install -y --only-upgrade kawaro` with no download of ours; otherwise `dnf` present means the verified RPM, `apt-get` present means the verified DEB. No `pkexec`, or no supported package manager, means a notification with the exact command, never a silent failure. A file in a user-writable folder must not go to root unchecked, so the privileged step (`pkexec kawaro update --install-privileged`) first copies the file to a root-owned 0700 folder, hashes the copy against the signed `sha256`, and installs the copy. `dnf install ./file.rpm` accepts an unsigned local file with a warning (see S-016). The package scripts are transactional, so a failed install leaves the old version. After success the daemon starts the new `kawaro` daemon and quits.

### 8. Timing and surface

No apply while a job runs or the hive is open. A pending apply is retried every 30 s. Progress shows in the tray tooltip ("Downloading update 42%"). The tray menu gets "Update now", enabled only when a verified package is ready. One notification per version and state (ready, updated, failed), counted by the existing `kawarod: notify:` line. Settings window rows for the new keys are a later ui task; until then the keys are edited in `config.toml`.

### 9. Failure and rollback

| Failure | Behaviour |
| --- | --- |
| Offline, DNS error, timeout, no curl | No notification; counts as a failure for the backoff; CLI status `offline` or `unavailable` |
| Bad signature, schema, channel or URL | Reject, log one line, no download |
| Size or hash mismatch | Delete the partial file, apply nothing, one notification per version |
| Install fails (msiexec, dnf, apt) | Old version stays (MSI rollback, transactional packages); the old daemon is relaunched; one notification |

There is no downgrade feature and no copy of the previous package is kept.

### 10. Release workflow

A job `update-manifest` after `release` in `packages.yml`: runs only for a `v*` tag in `M7MMAD-OMAR/kawaro`, in the `release-signing` environment; reads the real asset list, builds the manifest with `scripts/make-update-manifest.sh`, signs it with `scripts/sign-update-manifest.sh`, creates the `updates` release if missing (`--prerelease --latest=false`), and uploads with `--clobber`. `build-packages.sh` maps `-` to `~` in the package version so `0.4.0-beta.1` becomes `0.4.0~beta.1` for RPM and DEB.

## Crates

| Crate | Version | Licence | Where | Note |
| --- | --- | --- | --- | --- |
| semver | 1.0.28 | MIT OR Apache-2.0 | core | already in the tree, now a direct dependency |
| sha2 | 0.11.1 | MIT OR Apache-2.0 | core | already in the tree |
| minisign-verify | to verify | assumed MIT | core | new; verified and recorded in S-053a |

Tools (not shipped): `minisign` (to sign and to make fixtures), `jq`, `gh`.

## Stories

| Story | Scope | Beta minimum |
| --- | --- | --- |
| S-053a | Core pure logic: versions, manifest, signature, policy, schedule, config keys | yes |
| S-053b | Fetcher, check flow, verified download, cache folder | yes |
| S-053c | Manifest and signing scripts, workflow job, package version mapping, key document | yes |
| S-054a | Apply: msiexec helper, Linux plan, privileged install | yes |
| S-054b | Daemon: IPC, scheduler, tray item and tooltip, notifications | yes |
| S-054c | `kawaro update` command | no, but small |

## Preconditions (maintainer, before the beta tag)

- P1: generate the key pair, commit the public key, set `UPDATE_SIGNING_KEY` and `UPDATE_SIGNING_PASSWORD`, create the environment `release-signing`.
- P2: install `minisign` on the development machine (not installed today) so fixtures can be made.
- P3: for Windows, S-045 to S-051 done, in particular the MSI with a stable `UpgradeCode` that relaunches the daemon.

## Still open

- Windows in tomorrow's beta depends on P3; Linux does not.
- Whether the beta default channel should be `beta` for builds with a prerelease version.
- A polkit policy file for a named prompt, or the generic `pkexec` text.
- `minisign` availability in the ubuntu-22.04 runner image (assumed in universe).

## Manager decisions (2026-10-24)

- The default channel is `beta` when the installed version has a prerelease part, otherwise `stable`. A tester on `0.4.0-beta.1` is therefore offered `beta.2` as well.
- A manual check from the CLI or the tray bypasses `update_check = false`, because the user asked for it explicitly. Automatic requests stay at zero while it is off.
- Settings window rows for the update keys wait for a later ui story; for the beta the keys are edited in `config.toml`.
