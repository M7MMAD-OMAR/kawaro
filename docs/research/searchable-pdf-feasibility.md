# Searchable-PDF feasibility: desktop research and staged decision

Date: 2026-11-15. Status: conditional route selected for planning, no runtime qualification or dependency adoption.
Source baseline: main 43ca8f5. Story: S-147.
Spec: docs/specs/2026-11-15-next-features.md, section10.

## Decision

Prefer a separately installed system OCRmyPDF child for a later PDF OCR action. Do not implement a new PDF text-layer writer inside Kawaro, and do not rasterize every PDF through the image OCR adapter. This is conditional on a supported-version contract, dependency/license inventory and actual Arabic PDF acceptance when execution resumes.

No command, installer, OCR engine or PDF generation was run in this research. The choice is an architectural direction, not proof that the current machine has a compatible installation or that Arabic search/copy is correct.

## Why this route

| Route | Reuse | Main correctness/deployment cost | Planning verdict |
| --- | --- | --- | --- |
| System OCRmyPDF child | Existing killable process tree, temp-file output, system-tool discovery; upstream PDF OCR orchestration | Optional Python/system dependencies, renderer/version differences, hidden-text correctness and document preservation need qualification | Conditional preference |
| Tesseract PDF output for rendered page images | OCR engine and existing PDF raster/image operations | Rasterizing existing PDFs can lose existing text, forms, links and other semantics; reassembly/page geometry and mixed-text policy become Kawaro responsibilities | Not the general PDF route |
| New Rust PDF text-layer writer | Existing PDF libraries and local processing | Text coordinates, font embedding, Unicode mapping, Arabic shaping/order, mixed pages and document preservation require a new subsystem | Reject for this programme |
| Cloud OCR | No reusable local execution path | Document upload conflicts with the chosen local-processing feature scope | Out of scope |

Existing Poppler text extraction remains separate. OCR-only sidecars are not a substitute for extracting all existing text.

## Primary evidence

- [OCRmyPDF upstream repository](https://github.com/ocrmypdf/OCRmyPDF) describes adding OCR text layers and identifies MPL-2.0 for the program with separate licenses for other components. This is an upstream license fact, not a complete legal/distribution assessment.
- [OCRmyPDF installation](https://ocrmypdf.readthedocs.io/en/stable/installation.html) distinguishes package installation from the system dependencies and documents changing renderer choices. Ghostscript became optional in version17. A historical fixed dependency list is therefore insufficient for a new adapter.
- [OCRmyPDF cookbook](https://ocrmypdf.readthedocs.io/en/latest/cookbook.html) requires requested language packs and documents that sidecar text omits existing text and unprocessed pages.
- [Signed-document guidance](https://ocrmypdf.readthedocs.io/en/stable/cookbook.html) refuses signed-PDF modification by default; explicitly overriding that invalidates signatures. First-scope Kawaro behavior keeps refusal.
- [Tesseract upstream](https://github.com/tesseract-ocr/tesseract) identifies Apache-2.0 for its program and Leptonica as a dependency. Its PDF output capability does not prove a whole existing-PDF preservation pipeline.

These sources were consulted through primary search results on 2026-11-15. Existing local Tesseract Arabic experiments inform the OCR-engine choice, not hidden PDF-text order or a new accuracy claim.

## Proposed first PDF action contract

- One per-file action, one new searchable PDF output, never overwrite the input. Do not emit a sidecar implicitly.
- A separately installed, capability-qualified system executable. No bundled Python runtime, package-manager invocation or silent engine/language download.
- Explicit ara, eng or ara+eng after installed-data detection. No automatic language guess and no automatic language expansion.
- Keep existing text pages, process eligible scanned pages only, preserve order, page sizes and visual content. No force-rasterize or redo-existing-OCR option in the first UI.
- Ordinary PDF output, no implicit archival conversion. No PDF/A, PDF/UA or accessibility-tagging promise.
- Refuse encrypted/password-protected, digitally signed and corrupt documents in the first scope. No password prompt, decryption, signature invalidation override or claim that signatures survive modification.
- Cancellation terminates the process tree, cleans private temporary storage and commits no partial output. Page-level OCR timeout/skipped-page behavior must not turn an incompletely processed document into an unqualified searchable-success result.
- Existing mixed text/image documents may legitimately have pages with no recognisable text. Report processed/skipped/incomplete status explicitly; a searchable claim refers to processed recognized content, not guaranteed perfect extraction.

## Version and argument qualification gate

Before registration, the implementation packet must select a supported system-package version range from actual dependency evidence, then record the executable's supported argument surface. Intended policies are: one worker, explicit languages, preserve existing text, ordinary PDF output, no optimization/recompression by default, a bounded OCR timeout and no signature override. Candidate upstream switches include language/output/skip-text/optimization/job-limit controls, but exact compatibility must be established against the chosen supported version before argv is frozen.

Do not install latest, copy a command from an old PDF manual or use an unverified version constant. A capabilities mismatch yields unavailable with curated help, not a blind fallback. Arguments are always distinct process arguments, with all input/output paths owned by the existing safe task contract.

## Dependency and license work remaining before adoption

Record the selected OCRmyPDF distribution/version, Python requirements where relevant, PDF parser/renderer, Tesseract, installed language data, optional Ghostscript, temporary-file behavior and licenses/notices. Determine whether Kawaro merely invokes system software or later redistributes components; those are different scopes. Do not infer distribution permission for every transitive component from the top-level MPL/Apache labels.

No new crate or package pin is required for planning. A future adapter is an external Engine implementation and uses the existing one-task/one-output contract; it does not extend that trait with PDF OCR-specific methods.

## Deferred acceptance matrix

| Input | Required observable outcome |
| --- | --- |
| Arabic printed scan with ground truth | Search finds chosen known words; copy output is assessed for logical order and mixed digits, not only appearance |
| English scan | Search/copy works against known text; source remains identical |
| Mixed Arabic/Latin, diacritics, columns | Report actual ordering/recognition limitations; no blanket correctness claim |
| Existing text plus scanned pages | Existing text remains usable, order/geometry preserved, only eligible pages processed |
| Rotated scan and mixed page sizes | Correct orientation/placement and original page boundaries |
| Signed or encrypted input | Explicit refusal, no committed modified copy |
| OCR timeout/skipped page | No unqualified full-success claim; exact incomplete policy is recorded |
| Cancel during processing | Child tree stopped, temp cleanup, no final output |
| Missing executable/renderer/language | S-145 unavailable reason, no silent install/fetch |
| Large document/temp exhaustion | Bounded refusal/failure and no partial finalization; no invented resource guarantee |

Fixtures must be legitimate to use and retain, with a versioned manifest and source/expected-text provenance. Previously restricted external evaluation materials are not automatically redistributable.

## Exit criteria and sequence

Desktop research favors OCRmyPDF conditionally. S-147 still owns final supported-version/dependency policy, exact argv and the later qualification design/decision. Its implementation is not the PDF OCR adapter. A separate adapter story is allocated only when those gates close after the core/image OCR features stabilize. Other-platform deployment matrices remain later work.
