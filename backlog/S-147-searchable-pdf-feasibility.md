# S-147: Searchable-PDF feasibility and design

Status: desktop feasibility research written; version qualification and final architecture decision planned. No PDF OCR adapter implementation. Executable checks deferred by owner order.
Date: 2026-11-15.
Spec: ../docs/specs/2026-11-15-next-features.md
Implementation packet: ../docs/superpowers/plans/2026-11-15-next-features.md

## Story
I want a dependable searchable-PDF feature planned from actual engine capabilities before we commit to a text-layer implementation.

## Current facts
- Desktop result: [searchable-PDF feasibility](../docs/research/searchable-pdf-feasibility.md) conditionally prefers a system OCRmyPDF child; supported-version, deployment and runtime correctness gates remain open.
- Image OCR and existing Poppler extraction do not by themselves produce a correct searchable PDF.
- OCRmyPDF sidecar text is OCR-only and does not include all existing PDF text.
- Adding an external PDF OCR route introduces dependencies, licensing and document-integrity questions.

## Acceptance criteria
- AC1: Write docs/research/searchable-pdf-feasibility.md comparing a system OCRmyPDF route with existing-tool alternatives using primary sources.
- AC2: Inventory executables/language data/licenses and optional-install boundaries; propose architecture ADR after checking its number is free.
- AC3: Specify a single new PDF output and separate plain-text extraction; no implicit sidecar/multi-output contract.
- AC4: Define first-scope signed/encrypted refusal, mixed-page/rotation/order handling, cancellation/temp/timeouts, existing text and incomplete-job behavior.
- AC5: Define future Arabic search and copy order, mixed direction/numbers, existing text, columns, rotation and original visual-content validation.
- AC6: Return Adopt/Defer/Reject with an explicit dependency/correctness rationale. Only a later reviewed Adopt decision creates an implementation story.

## Future focused cases, not run
- A document containing both existing text and scanned pages has explicit text-preservation/skip semantics.
- Signed/encrypted/corrupt files never undergo silent forced rasterization or signature removal.
- No PDF/A/accessibility/searchable-Arabic quality promise without its own acceptance evidence.
- The decision and exact future fixture matrix are reviewable without installing/running an engine now.

## Boundaries
No product implementation, dependency install/pin, output generation, benchmark or release claim.

## Delivery and review
Parent owns architecture, packet approval and final review. Selected Pi/MiniMax-M3.1-Flash executes only a later authorized bounded packet. Native platform qualification follows stable feature behavior. Source implementation, executed acceptance and public release are recorded separately.

## Planning log
- 2026-11-15: parent authored this story from current source research and the owner request. No product code or executable check changed or ran.
