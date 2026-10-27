# Distribution status

Last checked: 2026-10-27.

## Windows

- Kawaro 0.4.0 is available on [GitHub Releases](https://github.com/M7MMAD-OMAR/kawaro/releases/tag/v0.4.0). The MSI and executables use a self-signed test certificate, not a trusted publisher certificate. The ZIP contains the test-signed executables.
- On 2026-10-27, the owner opened the published 0.4.0 MSI in the Windows 11 virtual machine and Kawaro displayed its welcome window. Through Orbit, the sample image action displayed the hive, selecting WebP produced a Windows notification saying `Done: 1 file` and naming `sample.webp`, and the settings window opened. The output file was not independently opened or inspected. The unattended install, full live checklist and physical Windows hardware remain unverified.
- The [SignPath Foundation application](signpath-application.md) was submitted on 2026-10-27. The form confirmed submission. Approval, service configuration and an externally signed release remain pending.
- The [WinGet submission](https://github.com/microsoft/winget-pkgs/pull/462805) is open and ready for review, not an accepted or published package. All twelve automated checks passed on the first push: pull request, manifest, URL, URL domain, manifest policy, catalog content, installer scan, installation validation, installer metadata validation, validation completed, and the contributor license agreement. The manifest also carries an Arabic locale file.
- Local validation ran on 2026-10-27 in the Windows 11 virtual machine with winget 1.29.380: `winget validate --manifest` exited 0, and `winget install --manifest` completed silently, verified the installer hash, and reported `kawaro 0.4.0`.
- The remaining gate is one approving review from a Microsoft moderator. There is no published service level; moderators quote ninety percent of pull requests reviewed within a day, and new packages take longer because identity and provenance are checked without manifest history.
- Microsoft Store developer registration is incomplete. The identity verification flow did not recognize the owner's document. Microsoft support can review the verification session; no Store listing has been submitted.

## Next steps

1. Wait for a Microsoft moderator to approve the WinGet pull request, then confirm the package reaches the community source with `winget source update --name winget` and `winget show --id Sbarah.Kawaro --versions`. Publication usually takes about an hour after the merge.
2. If SignPath approves the application, configure the service, confirm multi-factor authentication and issue a new release signed through the approved service. Verify signatures before describing them as trusted.
3. Record the owner's Windows test details and complete the remaining live checklist. Capture real Windows screenshots for a future Store listing.
4. Resolve Microsoft identity verification with its support team before continuing Store registration. Do not claim the application is listed or certified until Microsoft confirms it.
