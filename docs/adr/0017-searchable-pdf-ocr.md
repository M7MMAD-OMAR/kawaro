# ADR 0017: Searchable PDF through an optional system OCRmyPDF child process

Status: Proposed (authored 2026-11-21 under S-147; awaits parent approval; no code implemented and no runtime check executed)
Date: 2026-11-21
Story: backlog/S-147-searchable-pdf-feasibility.md (feasibility), backlog/S-150-searchable-pdf-ocr.md (DRAFT, not started)
Research: docs/research/searchable-pdf-feasibility.md
Spec: docs/specs/2026-11-15-next-features.md, section 10

## Context

Kawaro processes files locally without uploading them, keeps its base install light, never modifies the original file, and isolates heavy engines as separate child processes (`CLAUDE.md`, "Non-negotiable principles"). Today Kawaro handles PDF structure with `lopdf` and `qpdf` (ADR 0014), rasterization and compression with `ghostscript`, plain-text extraction with Poppler `pdftotext` (ADR 0013), and still-image OCR with system `tesseract` (S-146).

None of those routes produces a trustworthy searchable PDF from a scanned or mixed-content PDF:

- Poppler `pdftotext` extracts existing text only and cannot OCR scanned pages.
- Image OCR (`tesseract`) handles still images only. Rasterizing every page of a PDF through Ghostscript and reassembling Tesseract PDF output would destroy existing text, vector graphics, links, and forms on born-digital pages and force Kawaro to reimplement mixed-page preservation, geometry, and text-layer placement.
- Writing a custom PDF invisible-text-layer composer in Rust would require font embedding, ToUnicode CMap generation, coordinate mapping, Arabic RTL shaping, and PDF structure preservation, which is a separate subsystem outside Kawaro's scope.

Feasibility research in `docs/research/searchable-pdf-feasibility.md` compared these routes against an optional system `ocrmypdf` child process and audited primary upstream and distribution sources for versions, dependencies, licences, CLI switches, exit codes, and safety boundaries.

## Options

| Option | Cost | What it gives | Verdict |
| --- | --- | --- | --- |
| Optional system `ocrmypdf >= 14.4.0` child process | Optional system Python/OCRmyPDF stack installed by the user; renderer differences across v15/16 vs v17 require explicit Arabic fixture qualification | Preserves existing text pages (`--skip-text`), refuses signed and encrypted PDFs by default, fits `Engine` one-task/one-output contract and `ProcessTree`, zero new Rust crates, zero bundled binaries | Proposed |
| Rasterize PDF pages via Ghostscript + Tesseract PDF output + `lopdf` merge | Loses existing text, vectors, links, and forms on born-digital pages unless Kawaro builds its own page classifier and PDF reassembler | Reuses existing `gs` and `tesseract` binaries | Rejected |
| Build a custom Rust PDF text-layer writer | High complexity in font subsetting, ToUnicode CMaps, Arabic RTL placement, and PDF preservation | No Python dependency | Rejected |
| Cloud OCR service | Violates local-only non-negotiable principle | Offloads dependencies | Rejected |

## Decision

1. **New optional engine id `ocrmypdf`, one op `make-searchable`, one action `pdf.make-searchable`.**
   - The future adapter lives in `crates/engines/src/ocrmypdf.rs`, is registered in `kawaro_engines::all()`, and implements the existing `Engine` trait without changing `Engine::Task` or `TaskOutcome`.
   - It is strictly optional and system-provided. Kawaro never bundles, links, vendors, or installs `ocrmypdf`, Python, Tesseract, Ghostscript, `pikepdf`, `qpdf`, `unpaper`, `jbig2enc`, or language traineddata.
   - Discovery uses `locate::find` (PATH on Linux; PATH on Windows in a later platform phase) plus a bounded, read-only, cached probe in `kawaro_core::unsupported::ProbeCache` (S-145). Missing tools hide the action from the active hive and surface typed `UnavailableReason` diagnostics with curated install guidance (never running `sudo`, a package manager, `pip`, or a download).

2. **Minimum supported version floor: `ocrmypdf >= 14.4.0`.**
   - Upstream OCRmyPDF added default digital-signature detection and refusal (`DigitalSignatureError`) in version 14.4.0. Versions older than 14.4.0 (such as Debian 12 Bookworm's 14.0.1) invalidate digital signatures without warning.
   - The read-only version probe (`ocrmypdf --version`, 5 s budget, 64 KiB stdout cap) parses the version and refuses any version below `14.4.0` (reporting `ProbeUnknown` or a version-unsupported state with curated help, never ready).
   - Distro packages meeting the floor include Ubuntu 24.04 LTS (`15.2.0`), Debian 13 Trixie (`16.7.0`), Fedora 42 to 44 (`16.7.0` to `16.13.0`), and upstream `17.x`.

3. **Smallest viable external dependency set.**
   - Required on the user's system: `ocrmypdf >= 14.4.0` (MPL-2.0) with its distro Python dependencies including `pikepdf` (MPL-2.0) / `qpdf` (Apache-2.0), `tesseract` (Apache-2.0) with requested traineddata (`ara` and/or `eng`, Apache-2.0 upstream), and one PDF rasterizer (`ghostscript`, AGPL-3.0-or-later, on OCRmyPDF 14.4 to 16.x; `pypdfium2`, Apache-2.0 OR BSD-3-Clause, or `ghostscript` on OCRmyPDF 17.x).
   - Explicitly excluded from the required set: `unpaper` (GPL-2.0-only), `jbig2enc` (Apache-2.0), `pngquant`, and `tesseract-osd`. Kawaro's first-scope argv does not invoke `--clean`, `--deskew`, `--rotate-pages`, or `--optimize > 0`.
   - Because every dependency is executed only as an external system process and never linked or redistributed by Kawaro, its licence does not combine with Kawaro's GPL-3.0-or-later binaries.

4. **Single searchable-PDF output per task; plain-text extraction stays separate.**
   - Action `pdf.make-searchable` uses `OutputMode::PerFile`, extension `pdf`, suffix `-searchable`.
   - Each task produces exactly one new PDF file. The adapter writes into a private work directory, validates the completed output, and transfers it atomically into `task.output`. Only `kawaro_core::queue_impl::run_task` finalizes `task.output` via `finalize` (collision-safe atomic rename next to the original). The original file is never modified.
   - Plain-text extraction stays in `pdf.to-text` (`poppler` `pdftotext`, ADR 0013). Kawaro never passes `--sidecar` and never emits an implicit second file, because OCRmyPDF's sidecar contains OCR-only text and omits existing born-digital text and skipped pages.

5. **Fixed, non-destructive first-scope argv.**
   - Every argument is passed as a distinct `OsString`, never shell-joined:
     ```
     ocrmypdf --language <lang> --skip-text --output-type pdf --optimize 0 --jobs 1 --tesseract-timeout 180 --quiet -- <abs_input_path> <private_output_path>
     ```
   - `<lang>` is restricted to the validated enum `eng`, `ara`, or `ara+eng`, preflighted component-by-component through the S-145/S-146 `tesseract --list-langs` probe (`UnavailableReason::MissingLanguageData { language }`).
   - `--skip-text` preserves born-digital text pages intact and OCRs only pages without text. `--force-ocr` and `--redo-ocr` are never passed by default and are excluded from the first-scope UI.
   - `--output-type pdf` explicitly requests a standard non-archival PDF so OCRmyPDF 14.4 to 16.x never silently performs PDF/A conversion and OCRmyPDF 17.x `--output-type auto` never varies output format based on installed tools.
   - `--optimize 0` disables post-OCR image recompression and keeps visual streams intact without needing `pngquant` or `jbig2enc`.
   - `--jobs 1` limits OCRmyPDF to one internal worker under Kawaro's shared single-child OCR concurrency permit.

6. **First-scope refusal and document-integrity rules.**
   - **Digitally signed PDFs:** refused (`DigitalSignatureError`, exit code 2 -> `EngineError::Input`). Never pass `--invalidate-digital-signatures` and never silently strip signatures or rasterize signed pages.
   - **Encrypted PDFs:** refused (`EncryptedPdfError`, exit code 8 -> `EngineError::Input`). No password prompt or decryption in v1.
   - **Corrupt or non-PDF inputs:** refused (`ExitCode.input_file` 2 or `invalid_output_pdf` 4 -> `EngineError::Input` / `EngineError::Failed`), leaving no output.
   - **Page order, size, and rotation:** source page order, `MediaBox`/`CropBox` dimensions, and existing `/Rotate` metadata are preserved. First scope omits `--rotate-pages` and `--deskew`.
   - **Per-page timeout and incomplete runs:** `--tesseract-timeout 180` bounds single-page Tesseract work. Because OCRmyPDF can skip a timed-out page and still exit 0 with a warning, the adapter must inspect stderr for per-page timeout or skipped-page warnings and fail the task without committing a partial output, rather than reporting an incomplete document as unqualified success.

7. **Process-tree cancellation, timeouts, and temp-file isolation.**
   - Run through the shared `ProcessTree` (`crates/engines/src/process.rs`) and bounded runner (`crates/engines/src/ocr/child.rs`, `run_bounded`), killing the entire Unix process group (`killpg(..., SIGKILL)`) or Windows kill-on-close Job Object on cancel, timeout, or exit so child `tesseract` and `gs` processes never outlive the task.
   - Serialize behind the single-child OCR permit shared with `OcrEngine`, with cancellable wait.
   - Proposed initial policy bounds (not measured guarantees): 5 s probe timeout, 180 s per-page Tesseract timeout, 600 s whole-job wall-clock timeout, 128 MiB input PDF cap, 250-page cap, 256 MiB output PDF cap (polled during execution and checked strictly before transfer to `task.output`), 64 KiB probe stdout cap, 4 KiB stderr tail cap.
   - Set `TMPDIR`, `TEMP`, and `TMP` of the child environment to a private `<work>/tmp` directory inside Kawaro's task work directory (`work_dir`) so intermediate raster/PDF files stay isolated from small system `/tmp` mounts and are deleted on every exit path (success, failure, cancel, timeout).

## Consequences

- **No base-install growth:** zero new Cargo dependencies, zero bundled runtimes, workspace MSRV unchanged.
- **Explicit non-claims:** Kawaro makes no PDF/A archival claim, no PDF/UA or accessibility-tagging claim, no digital-signature preservation claim, and no encrypted-PDF support claim for this action.
- **Arabic RTL qualification gate:** OCRmyPDF 15.x and 16.x use legacy `sandwich`/`hocr` renderers while OCRmyPDF 17.x defaults to the `fpdf2` renderer with updated RTL shaping. No Arabic searchability or copy-order quality claim may be published until the real-fixture Arabic acceptance matrix in `docs/research/searchable-pdf-feasibility.md` is executed and recorded with exact tool and traineddata versions.
- **Platform order preserved:** Linux (Fedora, Debian 13+, Ubuntu 24.04+) is the first target. Debian 12 (`14.0.1`) is rejected safely by the `14.4.0` version floor. Native Windows setup remains manual/optional and is deferred to the later platform phase.
- **Next step:** If the parent accepts this ADR, implementation proceeds under draft story `backlog/S-150-searchable-pdf-ocr.md` as a bounded source packet, followed by deferred runtime and Arabic acceptance checks when executable validation resumes.
