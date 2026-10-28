# SignPath Foundation application draft: Kawaro / كوارو

Status: submitted on 2026-10-27 and declined by the Foundation on the same day. The
Foundation asks for public visibility signals (community adoption, external articles
or discussions, institutional backing, sustained engagement) before it issues a
certificate in its name; this project did not have them yet. The application can be
resubmitted once it does. This file keeps the prepared application data and the
evidence map the Foundation review checks against
(<https://signpath.org/terms.html>) so a resubmission is a copy and paste.

## Form fields

| Field | Value |
| --- | --- |
| Project name | Kawaro (كوارو) |
| Repository URL | <https://github.com/M7MMAD-OMAR/kawaro> (public) |
| Website | <https://kawaro.sbarah.com> |
| Download / release page | <https://kawaro.sbarah.com/download/> and <https://github.com/M7MMAD-OMAR/kawaro/releases> |
| License | GPL-3.0-or-later (OSI-approved, no commercial dual licensing, `LICENSE` in the repository) |
| Language and build | Rust workspace (Slint UI). The Windows artifacts are built in GitHub Actions on the `windows` job of `.github/workflows/packages.yml`, cross compiled for `x86_64-pc-windows-gnu` with mingw-w64 on `ubuntu-22.04` runners; the MSI is built with `wixl` in a `registry.fedoraproject.org/fedora:44` container and the portable zip by `scripts/windows/make-zip.sh` |
| What is signed | `kawaro.exe` and `kawarod.exe` (one signing request, before packing) and `kawaro-<version>-windows-x64.msi` (a second request, after it embeds the signed executables). The portable zip contains the signed executables and is not itself Authenticode signed |
| Code signing policy | <https://github.com/M7MMAD-OMAR/kawaro/blob/main/CODE_SIGNING_POLICY.md> (linked from the README as "Code signing policy") |
| Privacy policy | <https://kawaro.sbarah.com/en/privacy/> and `PRIVACY.md` in the repository |
| Contact | the maintainer's email, added at submission time |

## Project description (paste ready)

Kawaro (Arabic: كوارو) is a free, open source, local file converter for Linux
and Windows. The user selects one or more files in the file manager, a
hexagonal hive shows the actions that fit the file type and count, and the
result is saved as a new file next to the untouched original. All processing
happens on the user's machine: no uploads, no processing servers, no
telemetry. The project is a Rust workspace with a Slint UI, released under
GPL-3.0-or-later, with Arabic and English user interfaces and full RTL.

## How each SignPath Foundation condition is met

| Condition (signpath.org/terms.html) | Evidence |
| --- | --- |
| No malware | A file converter; no bundling of third party installers, no advertising software |
| OSI-approved license, no commercial dual licensing | GPL-3.0-or-later (`LICENSE`), every crate and the site |
| No proprietary code | All source is in the public repository; the vendored Slint Material components are MIT (`docs/adr/0002-settings-components.md`), dependencies are open source and recorded in `docs/engines.md` |
| Actively maintained | Main is under daily development; the maintainer answers issues |
| Already released in the form to be signed | 0.4.0 (2026-10-27) publishes the MSI and the portable zip on the releases page |
| Documented functionality | <https://kawaro.sbarah.com/> and the README describe what it does; the download page covers install, checksums and SmartScreen |
| Sign your own projects only | The maintainer owns the repository and builds the artifacts from it |
| Sign your own binaries only | The signed files are `kawaro.exe`, `kawarod.exe` and the MSI built from this repository's source in its own CI; no upstream binaries are signed |
| No hacking tools | Not a security tool of any kind |
| Respect user privacy and security | No user data is collected at all (`PRIVACY.md`), so there is nothing to disable; the only network access is an optional update check reading a signed manifest from GitHub Releases, off with `update_check = false` |
| Announce system changes | The MSI shows the standard install UI and installs per user; the portable zip changes nothing until the user runs `kawaro integration install` |
| Provide uninstallation | The MSI uninstalls through Windows; the portable install has `scripts/uninstall.sh` and `kawaro integration` remove |
| MFA for all team members | Owner step: MFA on GitHub and on SignPath before the first signing request |
| Code signing roles assigned | `CODE_SIGNING_POLICY.md`: Author, Reviewer and Approver are all Muhmad Omar Haj Hmdo (solo maintainer) |
| Code signing policy on the home page | `CODE_SIGNING_POLICY.md` with the "Code signing policy" heading and the required attribution, linked from both READMEs |
| Privacy policy specified | The exact statement "This program will not transfer any information to other networked systems unless specifically requested by the user or the person installing or operating it" is in `CODE_SIGNING_POLICY.md`, plus `PRIVACY.md` and the website privacy page |
| Artifact metadata restrictions | The version resource of both executables sets ProductName `Kawaro | كوارو` and ProductVersion from `Cargo.toml`, the same value in every artifact of one build (`S-085`). If the artifact configuration's product-name restriction must match "Kawaro" exactly, either widen the restriction or align the version resource; this is decided with SignPath at project setup |
| Verifiable builds, manual approval per release | Builds run only from this repository in GitHub Actions; the Approver manually approves every signing request |

## Owner steps after approval (nothing here is done yet)

1. Enable multi-factor authentication on SignPath and confirm it on GitHub.
2. Create the project in SignPath (handle `Kawaro` or `kawaro`), link the
   repository, and create the signing policy used for releases (the workflow
   reads its slug from a variable, any slug works). Configure the artifact
   metadata restrictions in the artifact configuration; see the metadata row
   above.
3. In the GitHub repository settings, under Secrets and variables / Actions:
   add the secret `SIGNPATH_API_TOKEN` (SignPath REST API token; the windows
   job reads it from the `release-signing` environment, so an environment
   secret there works) and the variables `SIGNPATH_ORGANIZATION_ID`,
   `SIGNPATH_PROJECT_SLUG` and `SIGNPATH_SIGNING_POLICY`.
4. Approve each signing request when a release is tagged (every release needs
   manual approval; this is a SignPath constraint, not an oversight).
5. Until then the workflow notices that SignPath is inactive and signs with
   the S-083 certificate fallback; a tag build with neither path fails
   loudly.

## Workflow wiring (already in the tree)

`.github/workflows/packages.yml`, windows job: build, upload the unsigned
executables as a throwaway artifact, submit the SignPath signing request
(`signpath/github-action-submit-signing-request@v3`, `wait-for-completion`,
signed files installed over the build output), then `make-zip.sh`,
`make-msi.sh`, the same for the MSI, verification of every signature (SignPath
artifacts against the system trust store, S-083 artifacts with the
certificate), and the `windows` upload the release job publishes from. The
SignPath steps are gated on `SIGNPATH_API_TOKEN` and a tag build; without them
the S-083 certificate path signs everything as before.
