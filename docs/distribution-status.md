# Distribution status

Last checked: 2026-10-31.

How the release pipeline and each channel work, and what a maintainer does: [distribution guide](distribution.md) ([العربية](distribution.ar.md)).

## Windows

- Current release: 0.5.0 (tag v0.5.0, 2026-10-31) on [GitHub Releases](https://github.com/M7MMAD-OMAR/kawaro/releases/latest). The MSI and executables use a self-signed test certificate, so SmartScreen warns on first run (More info, then Run anyway). The ZIP contains the test-signed executables.
- Checked in a Windows 11 virtual machine: install, the welcome and settings windows, the sample image action with its notification, and the 0.4.5 fixes (S-106, S-107: foreground, cleanup, names). Not checked: physical Windows hardware, the full live checklist, and the Explorer mixed selection check (S-108 M3).
- **Signing.** The maintainer has no code signing certificate and no identity document usable for the Microsoft Store developer verification. The SignPath Foundation declined the application on 2026-10-27 for lack of public visibility. The Microsoft Store, SignPath and any paid certificate are parked, not planned for now. The Windows build stays self-signed until that changes.
- **WinGet.** [winget-pkgs#462805](https://github.com/microsoft/winget-pkgs/pull/462805) passed all automated checks (labels Azure-Pipeline-Passed, Validation-Completed, New-Package) and the contributor license agreement is signed. It has waited since 2026-10-27 for approval by a volunteer moderator; there is nothing the project can do but wait. It is not a published package yet, and the `winget` channel job stays off until the package exists.
- **No cost paths that work now:** WinGet (waiting), Scoop (live), apt (live), and SmartScreen reputation, which builds as the MSI gets downloaded.

## Package channels

Written by `scripts/channels/run-all.sh` (S-103a); a row says what the last run of that channel did, and `skipped` means its secret or account is not configured. `ok` means the channel script succeeded, not that the registry has accepted the package: WinGet in particular stays pending until Microsoft merges the pull request.

<!-- channels:start -->
| Channel | State | Version | Checked |
| --- | --- | --- | --- |
| apt | ok | 0.5.0 | 2026-10-31 |
| aur | skipped | 0.5.0 | 2026-10-31 |
| copr | skipped | 0.5.0 | 2026-10-31 |
| scoop | ok | 0.5.0 | 2026-10-31 |
| winget | skipped | 0.5.0 | 2026-10-31 |
<!-- channels:end -->

What is live (2026-10-31, version 0.5.0, the tag run of the Packages workflow). All five channel jobs reported success, but that only means each channel script exited cleanly: apt and scoop printed `PUBLISHED`, while winget, aur and copr printed a clean `SKIP` line because their secret or switch is not configured (`WINGET_CREATE_GITHUB_TOKEN`, `AUR_ENABLED`, `COPR_CONFIG`). A green job is therefore not a published package for those three. 0.4.3 was published by hand because its tag predates the workflow, and the 0.4.4 tag run skipped apt and scoop until `secrets: inherit` was added:

- **apt:** `https://m7mmad-omar.github.io/kawaro-apt`, signed, served by GitHub Pages. the 0.5.0 run reported `PUBLISHED apt: 0.5.0` (the 0.4.5 to 0.4.8 runs did too); the 0.4.4 `InRelease` was checked signed and `Packages` listed 0.4.4 with the DEB hash of the release `SHA256SUMS`. `apt-get update` itself was not run on this machine for 0.4.4 to 0.5.0. Install line: [apt.md](distribution-channels/apt.md).
- **Scoop:** bucket `https://github.com/M7MMAD-OMAR/scoop-kawaro`. The 0.5.0 run reported `PUBLISHED scoop: 0.5.0` (as did 0.4.5 to 0.4.8); the 0.4.4 manifest was committed by the CI run and its hash equalled the `SHA256SUMS` line of the release ZIP. The 0.4.5 to 0.5.0 hashes were not compared here. Scoop itself was not run (it is not available on this machine).
- **Signed checksums:** the `sign-checksums` job ran for the first time on the 0.4.8 tag and printed `sign-checksums: uploaded SHA256SUMS.asc and kawaro-archive.asc for v0.4.8`, and a local `gpg --verify` of the downloaded `SHA256SUMS.asc` against the downloaded `kawaro-archive.asc` reported a good signature from the Kawaro release signing key (not marked trusted, as expected for a fresh keyring). The 0.5.0 tag run printed the same upload line for v0.5.0; that signature was not verified locally.

Open items, all platform limits:

- **AUR:** needs an account that a person has to create (CAPTCHA sign up). The key, verified host keys and package files are ready; activation is the account's public key plus `gh variable set AUR_ENABLED --body true` ([aur.md](distribution-channels/aur.md)).
- **COPR:** needs an account that a person has to create. Activation is `gh secret set COPR_CONFIG --env copr-publishing < copr-config-file` plus `gh variable set COPR_PROJECT --body ACCOUNT/kawaro` ([copr.md](distribution-channels/copr.md)).
- **WinGet:** waits for the review of [winget-pkgs#462805](https://github.com/microsoft/winget-pkgs/pull/462805). The channel job stays off until it is merged, because `komac update` needs the package to exist.

How a channel plugs in: add `scripts/channels/NAME.sh`; it runs from the repository root with the tag as `$1`, reads secrets from environment variables only, supports `--dry-run`, exits 0 on success, 10 on a clean SKIP (one line `SKIP NAME: VARIABLE not configured`), any other code on failure, and never prints a secret.

## Next steps

1. Wait for a moderator to approve the WinGet pull request, then confirm the package with `winget source update --name winget` and `winget show --id Sbarah.Kawaro --versions`, and turn the channel job on.
2. AUR and COPR need an account that only a person can create (sign up with a CAPTCHA and the person's identity). Until then they stay skipped; the exact steps are in [aur.md](distribution-channels/aur.md) and [copr.md](distribution-channels/copr.md).
3. Keep publishing: apt and Scoop update on every tag. SmartScreen reputation grows with downloads of the MSI.
4. Parked, not planned: Microsoft Store, SignPath resubmission, a paid certificate. Reopen only if the maintainer gets the missing identity or certificate.
