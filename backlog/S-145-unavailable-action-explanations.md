# S-145: Unavailable-action explanations

Status: planned, implementation not started. Executable checks deferred by owner order.
Date: 2026-11-15.
Spec: ../docs/specs/2026-11-15-next-features.md
Implementation packet: ../docs/superpowers/plans/2026-11-15-next-features.md

## Story
I want to understand why an expected operation is missing and how to make it available without an unexpected installation.

## Current facts
- Registry filters, doctor/engine discovery and zero-action explanations already exist.
- System qpdf and Poppler requirements must not be represented as built in.
- Proposed ADR0010 does not authorize a general engine downloader.

## Acceptance criteria
- AC1: Reuse current capability/filter composition and preserve runnable-only hive cells. Provide a secondary explanation entry rather than duplicate missing-tool prompts.
- AC2: Distinguish missing executable, missing language data, unsupported input, invalid options, disabled policy and unknown/unhealthy probe.
- AC3: Expose stable action/tool IDs and localized reasons consistently across UI/CLI/MCP for the same state.
- AC4: Use bounded cached probes off the UI loop; refresh explicitly after installation/PATH changes. No job, package manager, sudo or network download as a probe side effect.
- AC5: Offer curated diagnostic/setup help only. Keep system engines system-provided and existing managed-download allowlists unchanged.
- AC6: Cover existing mixed-selection and no-action semantics as controls, not replacement behavior.

## Future focused cases, not run
- Missing qpdf/Poppler has relevant help and no automatic download.
- Installed OCR executable with ara missing reports language data, not executable absence.
- Disabled MCP and incompatible mixed selection are not called missing software.
- Hung/replaced executable yields unknown then refresh, never false healthy status.

## Boundaries
No new installer, arbitrary shell commands, new cloud service, broad download catalog or native portal change.

## Delivery and review
Parent owns architecture, packet approval and final review. Selected Pi/MiniMax-M3.1-Flash executes only a later authorized bounded packet. Native platform qualification follows stable feature behavior. Source implementation, executed acceptance and public release are recorded separately.

## Planning log
- 2026-11-15: parent authored this story from current source research and the owner request. No product code or executable check changed or ran.
