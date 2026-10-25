# ADR 0009: How Kawaro builds and tests on macOS

Status: accepted
Date: 2026-10-26

## Context

macOS is the last platform in the order (Linux, X11, Windows, macOS). Nobody on the team has a Mac, and the codebase has never been compiled for `aarch64-apple-darwin`. Linux only crates (Wayland, D-Bus portals) are known to be in the tree. We need a way to find out what breaks and to keep checking it, at no cost and without breaking the rule that no work blocks main.

## Options

| Option | Cost | What it proves | Verdict |
| --- | --- | --- | --- |
| GitHub-hosted macOS runner (`macos-14`, Apple silicon) | Free for public repositories | Real build and the real test suite on real macOS, no GUI session control | Chosen first |
| Cross compiling from Linux with cargo-zigbuild or osxcross | Free | At best a compile check. Both need the Apple SDK, whose licence limits its use to Apple branded hardware, so we would be using it outside its terms. No tests run. | Rejected as the main path |
| macOS virtual machine on Linux (OSX-KVM, Docker-OSX) | Free | Full macOS | Rejected: the macOS licence allows virtualisation on Apple hardware only |
| A real Mac, or an hourly rental | Hourly rental or hardware | Interactive tests: Finder integration, focus, signing, notarization | Needed later |

## Decision

1. CI on a GitHub macOS runner first. The workflow `.github/workflows/macos.yml` runs on story branches, on demand and weekly, and is informational (`continue-on-error`), so it never blocks main. It uploads the full logs as an artifact.
2. The first run produces `docs/macos-gap.md`: every crate, what fails, one proposed story per gap.
3. Once the gaps are closed, the job can become a required check.
4. A real Mac (owned or an hourly rental) comes later, for Finder integration, the Apple Developer signing and notarization, and focus behaviour, none of which a headless runner can show.

## Consequences

- No Apple SDK leaves Apple branded hardware, so the licence is respected.
- The runner has no interactive GUI session: UI tests that need a window server stay out until the real Mac step.
- Runner minutes are free for a public repository, but macOS runners are slow to queue, which is why the job is not a pull request gate yet.
- The workflow uses no secrets and read-only permissions.
