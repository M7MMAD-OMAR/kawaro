[English](CODE_SIGNING_POLICY.md) | [العربية](CODE_SIGNING_POLICY.ar.md)

# Code signing policy

Kawaro applied to the SignPath Foundation on 2026-10-27. Approval and trusted signing are pending. Version 0.4.0 uses a self-signed test certificate for its executables and MSI. The ZIP contains those executables and is not itself Authenticode signed. This page describes the intended signing policy.

## What is signed

Only artifacts built from this repository's own source code are eligible. The Windows job in `.github/workflows/packages.yml` is prepared to submit `kawaro.exe` and `kawarod.exe` to SignPath before packaging, and the MSI after packaging, once SignPath is approved and configured. Every SignPath request requires manual approval. No third-party binaries are signed.

## Team and code signing roles

Kawaro is maintained by one person, so all three roles are held by the same
member:

| Role | Member |
| --- | --- |
| Authors (commits to the repository without additional review) | Muhmad Omar Haj Hmdo |
| Reviewers (reviews changes proposed by people who are not committers) | Muhmad Omar Haj Hmdo |
| Approvers (approves each signing request) | Muhmad Omar Haj Hmdo |

Every SignPath request must be approved by the Approver. Before using SignPath, the maintainer must confirm multi-factor authentication on GitHub and SignPath.

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
