# Distribution status

Last checked: 2026-10-30.

How the release pipeline and each channel work, and what a maintainer does: [distribution guide](distribution.md) ([العربية](distribution.ar.md)).

## Windows

- Kawaro 0.4.0 is available on [GitHub Releases](https://github.com/M7MMAD-OMAR/kawaro/releases/tag/v0.4.0). The MSI and executables use a self-signed test certificate, not a trusted publisher certificate. The ZIP contains the test-signed executables.
- On 2026-10-27, the owner opened the published 0.4.0 MSI in the Windows 11 virtual machine and Kawaro displayed its welcome window. Through Orbit, the sample image action displayed the hive, selecting WebP produced a Windows notification saying `Done: 1 file` and naming `sample.webp`, and the settings window opened. The output file was not independently opened or inspected. The unattended install, full live checklist and physical Windows hardware remain unverified.
- The [SignPath Foundation application](signpath-application.md) was submitted on 2026-10-27 and the Foundation declined it on the same day. The reason is the public visibility the Foundation program requires (community adoption, external articles or discussions, sustained engagement), not the quality of the project. The application can be resubmitted once the project has that visibility. The paid SignPath subscription is not a good fit for this project.
- The plan for a trusted Windows publisher is therefore the free path: an MSIX package in the Microsoft Store, where Microsoft re-signs the package with a Microsoft certificate after certification and the install shows no SmartScreen warning. The Store registration for individuals is free, and the remaining step is the owner's identity verification with Microsoft.
- The [WinGet submission](https://github.com/microsoft/winget-pkgs/pull/462805) is open and ready for review, not an accepted or published package. All twelve automated checks passed on the first push: pull request, manifest, URL, URL domain, manifest policy, catalog content, installer scan, installation validation, installer metadata validation, validation completed, and the contributor license agreement. The manifest also carries an Arabic locale file.
- Local validation ran on 2026-10-27 in the Windows 11 virtual machine with winget 1.29.380: `winget validate --manifest` exited 0, and `winget install --manifest` completed silently, verified the installer hash, and reported `kawaro 0.4.0`.
- A separate standard-user check ran on 2026-10-27 in the same virtual machine, with a local standard account and no administrator rights. The install itself could not be driven from that account, because the Windows Installer service refuses a remote procedure call from a non-interactive session, so Microsoft's own pipeline remains the authority for the standard-user install. The check did establish that the 0.4.0 MSI carries no `ALLUSERS` property: `wixl` accepts `InstallScope="perUser"` and drops it, so an elevated install of this MSI registers the program under the machine hive, where winget does not look for a package that declares `Scope: user`. Microsoft's installation validation installs as a standard, non-elevated user, where the install is per-user anyway, which is why the check passed. The next MSI build writes `ALLUSERS=2` and `MSIINSTALLPERUSER=1` explicitly, and `scripts/release/check-winget-scope.sh` keeps the manifest and the installer in agreement.
- The remaining gate is one approving review from a Microsoft moderator. There is no published service level; moderators quote ninety percent of pull requests reviewed within a day, and new packages take longer because identity and provenance are checked without manifest history.
- Microsoft Store developer registration is incomplete. The automated identity verification did not complete. The owner step is to resubmit it with matching account details and, if it fails again, to appeal to Microsoft support. No Store listing has been submitted.

## Package channels

Written by `scripts/channels/run-all.sh` (S-103a); a row says what the last run of that channel did, and `skipped` means its secret or account is not configured. `ok` means the channel script succeeded, not that the registry has accepted the package: WinGet in particular stays pending until Microsoft merges the pull request.

<!-- channels:start -->
| Channel | State | Version | Checked |
| --- | --- | --- | --- |
| apt | ok | 0.4.5 | 2026-10-30 |
| aur | skipped | 0.4.5 | 2026-10-30 |
| copr | skipped | 0.4.5 | 2026-10-30 |
| scoop | ok | 0.4.5 | 2026-10-30 |
| winget | skipped | 0.4.5 | 2026-10-30 |
<!-- channels:end -->

What is live (2026-10-30, version 0.4.5, the tag run of the Packages workflow). All five channel jobs reported success, but that only means each channel script exited cleanly: apt and scoop printed `PUBLISHED`, while winget, aur and copr printed a clean `SKIP` line because their secret or switch is not configured (`WINGET_CREATE_GITHUB_TOKEN`, `AUR_ENABLED`, `COPR_CONFIG`). A green job is therefore not a published package for those three. 0.4.3 was published by hand because its tag predates the workflow, and the 0.4.4 tag run skipped apt and scoop until `secrets: inherit` was added:

- **apt:** `https://m7mmad-omar.github.io/kawaro-apt`, signed, served by GitHub Pages. the 0.4.5 run reported `PUBLISHED apt: 0.4.5`; the 0.4.4 `InRelease` was checked signed and `Packages` listed 0.4.4 with the DEB hash of the release `SHA256SUMS`. `apt-get update` itself was not run on this machine for 0.4.4 or 0.4.5. Install line: [apt.md](distribution-channels/apt.md).
- **Scoop:** bucket `https://github.com/M7MMAD-OMAR/scoop-kawaro`. The 0.4.5 run reported `PUBLISHED scoop: 0.4.5`; the 0.4.4 manifest was committed by the CI run and its hash equalled the `SHA256SUMS` line of the release ZIP. The 0.4.5 hash was not compared here. Scoop itself was not run (it is not available on this machine).

Open items, all platform limits:

- **AUR:** needs an account that a person has to create (CAPTCHA sign up). The key, verified host keys and package files are ready; activation is the account's public key plus `gh variable set AUR_ENABLED --body true` ([aur.md](distribution-channels/aur.md)).
- **COPR:** needs an account that a person has to create. Activation is `gh secret set COPR_CONFIG --env copr-publishing < copr-config-file` plus `gh variable set COPR_PROJECT --body ACCOUNT/kawaro` ([copr.md](distribution-channels/copr.md)).
- **WinGet:** waits for the review of [winget-pkgs#462805](https://github.com/microsoft/winget-pkgs/pull/462805). The channel job stays off until it is merged, because `komac update` needs the package to exist.

How a channel plugs in: add `scripts/channels/NAME.sh`; it runs from the repository root with the tag as `$1`, reads secrets from environment variables only, supports `--dry-run`, exits 0 on success, 10 on a clean SKIP (one line `SKIP NAME: VARIABLE not configured`), any other code on failure, and never prints a secret.

## Next steps

1. Wait for a Microsoft moderator to approve the WinGet pull request, then confirm the package reaches the community source with `winget source update --name winget` and `winget show --id Sbarah.Kawaro --versions`. Publication usually takes about an hour after the merge.
2. Publish the MSIX package to the Microsoft Store through Partner Center, which signs it with a Microsoft certificate and removes the SmartScreen warning. The owner registers as an Individual developer (free) and completes the identity verification with Microsoft support; the package and the store listing details are prepared in the repository. Do not claim the app is listed or certified until Microsoft confirms it.
3. Build the public visibility the SignPath Foundation program asks for (community discussion, external references, sustained releases) and resubmit the application later. That is the free path to a trusted publisher name in a direct download.
4. Record the owner's Windows test details and complete the remaining live checklist. Capture real Windows screenshots for a future Store listing.
5. Resolve Microsoft identity verification: resubmit it with matching account details, and if it is rejected again appeal through the Partner Center appeal path or email storesupport@service.microsoft.com. Do not claim the application is listed or certified until Microsoft confirms it.
