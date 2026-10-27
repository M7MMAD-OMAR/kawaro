[English](CODE_SIGNING_POLICY.md) | [العربية](CODE_SIGNING_POLICY.ar.md)

# Code signing policy

Free code signing provided by [SignPath.io](https://about.signpath.io), certificate by [SignPath Foundation](https://signpath.org/).

Windows releases of Kawaro (`kawaro.exe`, `kawarod.exe`, the portable zip and
the MSI installer) are code signed by the SignPath Foundation certificate
through [SignPath.io](https://about.signpath.io). This page is the code signing
policy of the Kawaro project.

## What is signed

Only artifacts built from this repository's own source code are signed. The
Windows job of `.github/workflows/packages.yml` builds `kawaro.exe` and
`kawarod.exe` from the Rust workspace in this repository on a GitHub Actions
runner, submits them to SignPath for signing before `make-zip.sh` packs them
and `make-msi.sh` embeds them, signs the MSI the same way, and verifies every
signed artifact before upload. No third party binaries are signed with this
project's certificate. Every signing request requires a manual approval before
the release is signed.

## Team and code signing roles

Kawaro is maintained by one person, so all three roles are held by the same
member:

| Role | Member |
| --- | --- |
| Authors (commits to the repository without additional review) | Muhmad Omar Haj Hmdo |
| Reviewers (reviews changes proposed by people who are not committers) | Muhmad Omar Haj Hmdo |
| Approvers (approves each signing request) | Muhmad Omar Haj Hmdo |

Every signing request is approved by the Approver before any artifact is
signed. All team members use multi-factor authentication on GitHub and on
SignPath.

## Privacy

This program will not transfer any information to other networked systems
unless specifically requested by the user or the person installing or
operating it.

Kawaro processes files only on the user's machine: no uploads, no telemetry,
no accounts. The full privacy statement is in [PRIVACY.md](PRIVACY.md) and on
the website: <https://kawaro.sbarah.com/en/privacy/>. The only network access
of the application is an optional update check that reads a signed manifest
from GitHub Releases; it can be turned off in `config.toml`. Third party
components shipped or used: the Slint UI library (MIT) is compiled into the
binaries and makes no network calls, and the optional external engines
(FFmpeg, LibreOffice, Ghostscript) are taken from the user's system or
installed by the user's own request, and are used as local processes only.

## Releases and verification

Published artifacts and their `SHA256SUMS` are on the GitHub releases page
(<https://github.com/M7MMAD-OMAR/kawaro/releases>), and the in-app update feed
is separately signed with minisign (`docs/update-signing.md`). The SmartScreen
warning and how to verify a download are covered in
`docs/windows-smartscreen.md`.
