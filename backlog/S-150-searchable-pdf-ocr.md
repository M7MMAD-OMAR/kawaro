# S-150: Optional searchable-PDF OCR adapter (OCRmyPDF)

Status: source implemented and parent-reviewed on 2026-11-21 (registered in the engine list, fmt and clippy clean, workspace tests run on Linux). The Arabic fixture acceptance matrix and runtime checks with a real OCRmyPDF remain open.
Date: 2026-11-21
ADR: ../docs/adr/0017-searchable-pdf-ocr.md (Accepted)
Feasibility: ../docs/research/searchable-pdf-feasibility.md
Spec: ../docs/specs/2026-11-15-next-features.md, section 10

## Story

As a user with scanned or mixed-content PDFs, I want to generate a new locally processed searchable PDF with an explicit Arabic or English language choice, so that I can search and copy text from scanned pages without uploading the file, modifying the original, or losing existing text pages.

## Acceptance criteria

- AC1: Add an optional system `ocrmypdf` engine adapter (`crates/engines/src/ocrmypdf.rs`, engine id `ocrmypdf`, op `make-searchable`) and bilingual `pdf.make-searchable` action (`actions/pdf.toml`, `OutputMode::PerFile`, ext `pdf`, suffix `-searchable`, typed `lang` enum `eng`, `ara`, `ara+eng`). Do not add any Cargo dependency, bundled Python runtime, bundled binary, or automatic installer.
- AC2: Extend capability discovery and `kawaro doctor` / S-145 `CapabilityReport` so missing `ocrmypdf` reports `UnavailableReason::MissingExecutable { program: "ocrmypdf" }`, missing `tesseract` reports `MissingExecutable { program: "tesseract" }`, missing `ara`/`eng` traineddata reports `UnavailableReason::MissingLanguageData { language }`, and a timed-out, malformed, or below-floor (`< 14.4.0`) `ocrmypdf --version` probe reports `UnavailableReason::ProbeUnknown` (or version-unsupported refusal) with curated read-only help. Probes are bounded (5 s, 64 KiB stdout cap) and cached in `ProbeCache` with explicit refresh.
- AC3: Pass fixed non-destructive argv with every value as a distinct `OsString` (never shell-joined): `--language <lang>`, `--skip-text`, `--output-type pdf`, `--optimize 0`, `--jobs 1`, `--tesseract-timeout 180`, `--quiet`, `--`, `<abs_input>`, `<private_output>`. Never pass `--force-ocr`, `--redo-ocr`, `--invalidate-digital-signatures`, or `--sidecar`. Keep plain-text extraction strictly in the separate `pdf.to-text` action.
- AC4: Enforce first-scope document safety: digitally signed PDFs (`DigitalSignatureError`, exit code 2), encrypted/password-protected PDFs (`EncryptedPdfError`, exit code 8), and corrupt or non-PDF files fail cleanly with `EngineError::Input` and commit no output. Original input bytes remain untouched, and existing text pages, page order, and page sizes are preserved on mixed PDFs.
- AC5: Enforce process-tree lifecycle, timeouts, temp isolation, and atomic single-output commit: spawn via `ProcessTree` and bounded runner (`ocr/child.rs`) under the shared single-child OCR permit; isolate child `TMPDIR`/`TEMP`/`TMP` inside the private task work directory; apply admission/output policy caps (128 MiB input, 250 pages, 256 MiB output, 600 s whole-job timeout); fail with no committed output if any per-page timeout warning occurs or on cancellation; transfer verified output atomically to `task.output` so only the existing queue finalizes `<stem>-searchable.pdf`.
- AC6: Document the adapter, dependencies, licences, version floor (`>= 14.4.0`), and explicit non-claims (no PDF/A, PDF/UA, archival, or digital-signature preservation claim) in `docs/engines.md`. Publish no Arabic searchability or copy-order quality claim until the deferred Arabic fixture acceptance matrix in `docs/research/searchable-pdf-feasibility.md` is executed and recorded.

## Test cases (future check stage, not executed)

| Criterion | Test case (input and exact assertion) | Type (unit/integration/ui/fixture) |
| --- | --- | --- |
| AC1, AC3 | Argv builder with paths containing spaces and leading `-`, and `lang` in `eng`, `ara`, `ara+eng`: assert exact argument vector, no `--force-ocr`/`--redo-ocr`/`--invalidate-digital-signatures`/`--sidecar`, and rejection of invalid `lang` values (`BadParam`). | Unit |
| AC2 | Mock `ocrmypdf` returning `14.0.1` (below floor), `15.2.0` (valid), hang (timeout), and missing binary; mock `tesseract --list-langs` missing `ara`: assert exact `ProbeOutcome` and S-145 `UnavailableReason` values (`MissingExecutable`, `MissingLanguageData`, `ProbeUnknown`) with zero installer or network side effects. | Unit / Integration |
| AC4 | Mock and later real fixtures for digitally signed PDF (exit 2 / `DigitalSignatureError`), encrypted PDF (exit 8 / `EncryptedPdfError`), and corrupt non-PDF: assert `EngineError::Input`, zero committed output, and identical source SHA-256. | Integration / Fixture |
| AC5 | Mock child spawning a sleeper grandchild, cancelled mid-run and timed out; mock child emitting a per-page timeout warning with exit 0; mock child writing > 256 MiB: assert grandchild reaped, private work dir and `task.output` removed, no final PDF created, and non-success return. | Integration |
| AC5 | Mixed text-plus-scanned PDF: assert single new `<stem>-searchable.pdf` via queue `finalize`, existing text preserved, page count/order/boxes unchanged, original unchanged. | Fixture (deferred) |
| AC6 | Future Arabic fixture suite (not executed): printed Arabic words searchable, logical copy-paste order, mixed Arabic/Latin/digits, diacritics, multi-column order, rotated pages, existing-text preservation, and visual-content-unchanged comparison, with `ocrmypdf`, `tesseract`, and `ara.traineddata` versions recorded. | Fixture (deferred) |

## Out of scope

- Bundling or auto-installing `ocrmypdf`, Python, `tesseract`, `ghostscript`, `unpaper`, `jbig2enc`, or language packs.
- PDF/A (`--output-type pdfa*`), PDF/UA, or accessibility tagging claims.
- Digital-signature invalidation override (`--invalidate-digital-signatures`) or signature preservation.
- Password prompts or decryption of encrypted PDFs.
- `--force-ocr`, `--redo-ocr`, `--rotate-pages`, `--deskew`, `--clean`, or `--sidecar` multi-output generation.
- Partial `--pages` selection in v1.
- Native Windows packaging or runtime certification (deferred to later platform phase).

## Review log

- 2026-11-21: drafted from S-147 feasibility study and proposed ADR 0017. Marked DRAFT, not started. No product code written and no checks executed.
- 2026-11-21: ADR 0017 accepted by parent. Implemented AC1 through AC6 in source (`crates/engines/src/ocrmypdf.rs`, `crates/engines/src/locate.rs`, `crates/engines/src/ocr.rs`, `crates/engines/tests/ocrmypdf.rs`, `actions/pdf.toml`, `crates/core/src/unsupported.rs`, `crates/cli/src/capabilities.rs`, `crates/daemon/src/plan.rs`, `docs/engines.md`). Marked source implemented; engine registration in `crates/engines/src/lib.rs` and executable checks stay with the parent/check stage.
