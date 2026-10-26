# S-083 Windows install trust: a signing pipeline and honest SmartScreen guidance
Status: approved under manager delegation (2026-10-26); the signing part needs the maintainer's certificate first

## Story
As a Windows user downloading Kawaro, I want the installer signed and the download page to tell me exactly what the warning means and what to click, so that installing takes one informed decision instead of a scary dead end.

## Research (2026-10-26, sources: Microsoft, "SmartScreen reputation for Windows app developers", learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation, and the Microsoft file submission page, wdsi/filesubmission)
SmartScreen weighs two signals: publisher reputation (is the file signed, and does the certificate have history) and file hash reputation (has this exact file been downloaded without trouble). Unsigned files get "Windows protected your PC" with "Unknown publisher" and the only way on is "More info" then "Run anyway". An OV or EV signed file can still warn until reputation accumulates, but the dialog shows the verified publisher name instead. EV no longer bypasses SmartScreen (Microsoft's own 2026 guidance), so paying the EV premium for this is wasted money. Microsoft Store re-signs with Microsoft's certificate and never warns, but Store distribution is out of scope here. Submitting every release to Microsoft's malware analysis portal can clear the warning much faster. A timestamp on every signature is mandatory, without one the signature dies with the certificate. Do not modify a file after signing it, and keep one signing identity across releases so reputation accumulates on the certificate.

## Files
`.github/workflows/packages.yml` (windows job signing steps), `scripts/windows/sign-local.sh` (extended with a certificate-source argument), `docs/windows-smartscreen.md` and `docs/windows-smartscreen.ar.md` (new), `site/download/index.html` and `site/en/download/index.html` via `site/build.py`, `docs/engines.md` (licence note for new signing tooling).

## Needs from the maintainer
An OV code signing certificate (individual or organisation), stored in the GitHub environment `release-signing` next to the update key. The code, pipeline and docs land and are tested with a dry-run certificate (AC1); only AC6, the real signed release, waits for this certificate.

## Acceptance criteria
- AC1: the signing pipeline signs `kawaro.exe`, `kawarod.exe`, the MSI and the zip contents with the certificate named by the workflow, each signature carries an RFC 3161 timestamp, and the job fails loudly when a release build has no certificate. The pipeline position is fixed: exes are signed in the windows job before `make-zip.sh` packs them and before `make-msi.sh` embeds them, and before artifact upload (the update-manifest job downloads artifacts and must never re-sign or hash unsigned bytes); the zip carries signed exes and its own archive signature where the tooling allows. The certificate moves to the windows job under the `release-signing` environment (today only update-manifest sees signing secrets). On `workflow_dispatch` runs the sign step is skipped with a notice, not a failure; only tag builds of a release fail loudly without a certificate. The signing entry point extends `scripts/windows/sign-local.sh` with a certificate-source argument (one audited signing path with its own timestamp and verify behavior) rather than adding a third signing script. The story is executable without the real certificate: a dry run with a throwaway test certificate proves sign, timestamp and verify end to end (that is what the tester runs). Running the pipeline with the maintainer's real OV certificate in `release-signing` is an owner step (AC6), not a code dependency. The workflow verifies every signed artifact with `osslsigncode verify` (or `signtool verify`) and the job fails on any verification failure.
- AC2: `SHA256SUMS` is generated after signing, so the published hashes match exactly the bytes users download; the update feed and `scripts/sign-update-manifest.sh` behavior is unchanged (minisign stays the update signature). Ordering is fixed: signing happens before any checksum or manifest generation, so S-078's winget manifests take their hash from the signed MSI and never from an unsigned one; a release that regenerates manifests must regenerate them from the signed artifacts only.
- AC3: `docs/windows-smartscreen.md` and `docs/windows-smartscreen.ar.md` exist as an English main file and an Arabic copy beside it, each starting with the language switch line. They show what the dialog displays before and after signing (unknown publisher versus the publisher name), the exact clicks ("More info", "Run anyway"), how to verify the download against `SHA256SUMS`, and never claim that EV removes the warning.
- AC4: the guide contains a per-release section for the maintainer: submit the built MSI to Microsoft's malware analysis portal (the `wdsi/filesubmission` URL), keep the submission confirmation, and expect the warning to fade as downloads accumulate on the same certificate.
- AC5: the Arabic and English download pages carry a two-line honest note next to the Windows download button and link the guide; the note never says the build is safe to run unverified. This story owns the note text and the guide link; S-088 includes them and writes no second copy. `scripts/design-lint.sh` and the no-long-dash check pass.
- AC6: owner step, run once the real OV certificate exists: a real release build is signed with it, the signed artifacts verify, and the maintainer submits that MSI to Microsoft's analysis portal. Until then the story reports the certificate as pending, never as done.
- AC7: one CHANGELOG line in both languages when the story lands.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1, AC2 | a dry run of the extended `sign-local.sh` with a throwaway self-signed certificate proving sign then verify passes and a missing certificate fails; a step-order check of `packages.yml` (sign before pack, embed, upload and hash) written as a new static test under `tests/release/` alongside the existing workflow checks | integration, static |
| AC3, AC4 | static checks that both guide files exist with the switch line, the dialog wording, the clicks and the portal URL, in both languages | static |
| AC5 | download page note and link checks in both languages; design lint | static |
| AC6 | owner-run: the signed release verifies and the MSI submission confirmation is kept | owner checklist |

## Out of scope
- Microsoft Store packaging (it changes the installer and the AUMID), winget (S-078 owns free channels), the Windows 11 modern Explorer menu (it needs the signed sparse package), measuring or promising SmartScreen reputation outcomes.

## Review log
- 2026-10-26, planner: written from the owner Windows beta report; three goldfish reviews (design, readiness, zoom-out) applied to the text.
