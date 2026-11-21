# Searchable-PDF feasibility: desktop research and staged decision

Date: 2026-11-15 (extended and completed 2026-11-21). Status: Adopt decision recommended for parent review (proposed ADR 0017, draft story S-150). No runtime qualification, dependency install, or adapter implementation has been executed.
Source baseline: main 43ca8f5 (initial desktop research) and 2026-11-21 workspace source (`crates/core/src/unsupported.rs`, `crates/core/src/queue_impl.rs`, `crates/core/src/output_impl.rs`, `crates/engines/src/process.rs`, `crates/engines/src/ocr.rs`, `crates/engines/src/ocr/child.rs`). Story: S-147.
Spec: docs/specs/2026-11-15-next-features.md, section 10.

## Decision

**Adopt** the optional system OCRmyPDF child-process route (`ocrmypdf >= 14.4.0`) as Kawaro's architecture for a future searchable-PDF action, subject to parent review of proposed [ADR 0017](../adr/0017-searchable-pdf-ocr.md) and later execution of the Arabic and safety acceptance matrix before any quality or support claim.

Do not implement a new PDF text-layer writer inside Kawaro, and do not rasterize every PDF through the image OCR adapter. Keep OCRmyPDF and all of its runtime dependencies strictly outside Kawaro's base install, discovered at run time as an optional system engine like `ffmpeg`, `soffice`, `gs`, `pdftotext`, `qpdf`, and `tesseract`.

No command, installer, OCR engine, build, test, or PDF generation was run in this study. This decision is a documentation-only architectural recommendation, not proof that any machine has a compatible installation or that Arabic search and copy order pass at run time.

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

## Version-specific dependency and licence inventory (2026-11-21)

All primary sources below were accessed on **2026-11-21**. Kawaro is licensed under GPL-3.0-or-later (`docs/engines.md`). Every component below stays an external system program or system library invoked solely through a child process (`ocrmypdf` and `tesseract`); nothing is linked into Kawaro's binaries and nothing is redistributed by Kawaro.

### Core and optional external components

| Component | Version facts (upstream and distros) | Licence (primary source) | Role in OCRmyPDF route | Required in Kawaro smallest viable set? |
| --- | --- | --- | --- | --- |
| **OCRmyPDF** (`ocrmypdf`) | Minimum safe floor for Kawaro: **>= 14.4.0** (digital-signature detection and refusal added in 14.4.0). Debian 12 Bookworm: `14.0.1+dfsg1-1` (below floor, refused). Ubuntu 24.04 LTS Noble: `15.2.0+dfsg1-1` (universe). Debian 13 Trixie: `16.7.0+dfsg1-3`. Fedora 42: `16.7.0+`, Fedora 43: `16.11.0+`, Fedora 44/45: `16.13.0+`. Upstream latest line: `17.0.0+`. | **MPL-2.0** (`pyproject.toml`, `LICENSE`) | Top-level PDF OCR orchestrator invoked as a child process | **Yes** |
| **Tesseract** (`tesseract`) | Required `>= 4.1.1` (Fedora `ocrmypdf` spec) / `>= 4.0.0` (Ubuntu `ocrmypdf` package); current upstream is 5.x. Depends on **Leptonica** (`BSD-2-Clause`). | **Apache-2.0** (Tesseract); **BSD-2-Clause** (Leptonica) | OCR recognition engine invoked by OCRmyPDF and probed by Kawaro via `tesseract --list-langs` | **Yes** (already an optional Kawaro system engine under S-146) |
| **Tesseract language packs** (`ara`, `eng`; optional `osd`) | Fedora: `tesseract-langpack-ara`, `tesseract-langpack-eng`, `tesseract-osd` (from `tesseract-tessdata`). Debian/Ubuntu: `tesseract-ocr-ara`, `tesseract-ocr-eng`, `tesseract-ocr-osd`. | **Apache-2.0** (upstream `tesseract-ocr/tessdata`, `tessdata_fast`, `tessdata_best`; third-party traineddata may carry separate licences) | Language models for explicit `eng`, `ara`, or `ara+eng` selection (`osd` not required when `--rotate-pages` is off) | **Yes** for requested `ara`/`eng` (`osd` optional, not required in v1) |
| **pikepdf** (`python3-pikepdf`) and **qpdf** (`qpdf` / `libqpdf`) | Debian Bookworm / Ubuntu Noble: `python3-pikepdf >= 5.0.1`. Debian Trixie: `>= 8.14.0`. Fedora 44/45: `>= 10`. Wraps system `libqpdf`. | **MPL-2.0** (`pikepdf`); **Apache-2.0** (`qpdf`) | PDF structural parsing, signature detection, and PDF reassembly without re-encoding page images | **Yes** (pulled automatically by distro `ocrmypdf` package) |
| **Ghostscript** (`gs` / `gswin64c`) | Fedora `ocrmypdf`: `ghostscript >= 9.15`. Ubuntu `ocrmypdf`: `ghostscript >= 9.18~dfsg~`. Required rasterizer in OCRmyPDF 14.4 through 16.x; optional in OCRmyPDF 17.0.0+ when `pypdfium2` is installed and `--output-type pdf` is used. | **AGPL-3.0-or-later** (or Artifex Commercial License) | PDF page rasterizer for OCR input on OCRmyPDF 14.4 to 16.x (not used for PDF/A conversion under `--output-type pdf`) | **Yes on OCRmyPDF 14.4 to 16.x**; optional on 17.x if `pypdfium2` is installed (already an optional Kawaro system engine) |
| **pypdfium2 / PDFium** (OCRmyPDF 17.x alternative rasterizer) | Introduced as optional rasterizer in OCRmyPDF 17.0.0+, making Ghostscript optional for non-PDF/A runs. | **Apache-2.0 OR BSD-3-Clause** (`pypdfium2`) | Optional rasterizer on OCRmyPDF 17.x | **No** (alternative to Ghostscript only on 17.x) |
| **fpdf2** (OCRmyPDF 17.x default text renderer) | Default PDF invisible-text-layer renderer in OCRmyPDF 17.0.0+ (`--pdf-renderer auto` / `fpdf2`), adding improved RTL and Arabic shaping support compared to legacy `sandwich`/`hocr` renderers in 15.x and 16.x. | **LGPL-3.0-or-later** (`py-pdf/fpdf2`) | Invisible text-layer rendering on OCRmyPDF 17.x | Pulled by OCRmyPDF 17.x only (not required on 14.4 to 16.x) |
| **Python runtime & support libraries** | System `python3` plus distro dependencies (`python3-pillow`/`python3-pil`, `python3-img2pdf`, `python3-pdfminer`, `python3-pluggy`, `python3-packaging`, `python3-rich` in 14.4+, `python3-deprecation` on Fedora, `python3-reportlab` on Debian/Ubuntu 14 to 16). | Permissive / weak-copyleft system Python packages managed by the OS distribution | Required runtime of the system `ocrmypdf` package | **Yes** (installed as system dependencies of `ocrmypdf`, never bundled by Kawaro) |
| **unpaper** (`unpaper`) | Packaged natively in Fedora 42 to 44, Debian, and Ubuntu (`Recommends` on Debian/Ubuntu). Used only by `--clean` / `--clean-final`. | **GPL-2.0-only** (`COPYING`; individual files MIT/Apache-2.0) | Optional scan cleanup and deskew filter | **No** (excluded from v1 argv) |
| **jbig2enc** (`jbig2`) | Packaged as `jbig2` in Debian Trixie and Ubuntu 24.04+. **Not packaged natively in official Fedora repositories** (OCRmyPDF operates without it and falls back to CCITT compression). Used only during optimization. | **Apache-2.0** (`COPYING`) | Optional monochrome JBIG2 image compression | **No** (excluded by `--optimize 0`) |
| **pngquant** | `Recommends` on Debian/Ubuntu; optional on Fedora. Used only when `--optimize >= 2`. | GPL-3.0-or-later (commercial option) | Optional lossy PNG quantization | **No** (excluded by `--optimize 0`) |

### Distribution and Windows availability

- **Fedora (42, 43, 44):** `ocrmypdf` (`16.7.0+` to `16.13.0+`), `tesseract`, `tesseract-langpack-ara`, `tesseract-langpack-eng`, `ghostscript`, `python3-pikepdf`, and `qpdf` are all in official repositories (`dnf install ocrmypdf tesseract-langpack-ara tesseract-langpack-eng`). Every supported Fedora release meets the `>= 14.4.0` floor. `jbig2enc` is absent from Fedora repos and is not needed under `--optimize 0`.
- **Debian and Ubuntu:** Ubuntu 24.04 LTS Noble (`15.2.0+dfsg1-1`, universe) and Debian 13 Trixie (`16.7.0+dfsg1-3`) ship `ocrmypdf >= 14.4.0` with `ghostscript`, `tesseract-ocr`, `python3-pikepdf`, and separate `tesseract-ocr-ara` / `tesseract-ocr-eng` packages (`apt install ocrmypdf tesseract-ocr-ara tesseract-ocr-eng`). **Debian 12 Bookworm** ships `14.0.1+dfsg1-1`, which predates default digital-signature refusal (`14.4.0`) and is intentionally rejected by Kawaro's version probe unless the user installs a newer `ocrmypdf >= 14.4.0`.
- **Windows:** Upstream provides no single-file native Windows installer. Official OCRmyPDF installation docs recommend WSL2 with Ubuntu (`apt install ocrmypdf`) or Docker, or a manual native setup requiring Python 3, Tesseract (with Arabic/English data), Ghostscript on `PATH`, and `pip install ocrmypdf`. Consistent with Kawaro's Linux-first platform order (`CLAUDE.md`, `docs/specs/2026-11-15-next-features.md` section 11), Windows native discovery (`ocrmypdf.exe` on `PATH`) is optional later-platform work and no Windows runtime support is claimed here.

### Optional-install boundaries

Kawaro **never** bundles, vendors, links, downloads, or installs `ocrmypdf`, Python, `tesseract`, `ghostscript`, `pikepdf`, `qpdf`, `unpaper`, `jbig2enc`, or language `traineddata`. Detection follows the exact same read-only system-engine pattern already established in `crates/engines/src/locate.rs`, `crates/core/src/unsupported.rs` (`ProbeCache`, `UnavailableReason`, `InstallHint`), and `docs/engines.md`:

- Missing `ocrmypdf` hides the action from the active hive and reports `UnavailableReason::MissingExecutable { program: "ocrmypdf" }` with a curated read-only package/page hint.
- Missing `tesseract` reports `UnavailableReason::MissingExecutable { program: "tesseract" }`.
- Installed executables missing `ara` or `eng` traineddata report `UnavailableReason::MissingLanguageData { language }` with `help_id: Some("ocr-lang")`.
- A timed-out, malformed, or below-floor (`< 14.4.0`) `ocrmypdf --version` probe reports `UnavailableReason::ProbeUnknown` (never runnable).
- Probing never runs `sudo`, a package manager, `pip`, or any network request.

## Runtime contract and document-handling specification (2026-11-21)

### Single output per task and separate plain-text extraction

- **One searchable PDF per task:** The proposed action `pdf.make-searchable` (`engine = "ocrmypdf"`, `op = "make-searchable"`, `OutputMode::PerFile`, ext `pdf`, suffix `-searchable`) writes **exactly one** searchable PDF per input file.
- **Atomic queue finalization:** OCRmyPDF writes to a fixed private output path (`<work>/searchable.pdf`) inside the task's private work directory. After all exit, warning, header, and size checks pass, the adapter transfers that verified file atomically to `task.output` (the hidden temp path allocated by `kawaro_core::output::temp_path`). Only the existing queue (`kawaro_core::queue_impl::run_task` calling `kawaro_core::output::finalize`) performs the collision-safe atomic rename to `<stem>-searchable.pdf` next to the input. The original input file is never modified.
- **Plain-text extraction remains separate:** Extracting plain text from a PDF remains the existing `pdf.to-text` action (`poppler` `pdftotext`, ADR 0013). Kawaro never passes `--sidecar` and never introduces an implicit multi-output contract.
- **Why OCRmyPDF sidecar text is unsuitable as full-document text extraction:** Per the upstream OCRmyPDF cookbook, `--sidecar` emits only text produced by the OCR pass. Pages skipped because they already contain text (`--skip-text`), pages outside `--pages`, and pages skipped due to `--skip-big` or per-page timeouts are omitted from the sidecar (replaced by empty or placeholder text). Using a sidecar would silently lose existing born-digital text on mixed documents.

### Document-integrity and edge-case rules

| Condition | First-scope required behaviour |
| --- | --- |
| **Digitally signed PDF** | **Refuse.** Never pass `--invalidate-digital-signatures`, never strip signatures, and never silently rasterize signed pages. Because the version floor is `ocrmypdf >= 14.4.0`, OCRmyPDF refuses with `DigitalSignatureError` (`ExitCode.input_file` = 2); the adapter maps this to `EngineError::Input` naming the digital-signature refusal. |
| **Encrypted / password-protected PDF** | **Refuse.** OCRmyPDF exits with `ExitCode.encrypted_pdf` (8, `EncryptedPdfError`). Map to `EngineError::Input` stating that encrypted PDFs are not supported in this scope. No password prompt, no decryption, no silent rasterization. |
| **Corrupt or non-PDF file** | **Refuse.** `ExitCode.input_file` (2) or `ExitCode.invalid_output_pdf` (4) maps to `EngineError::Input` or `EngineError::Failed`, leaving no committed output and untouched source bytes. |
| **Mixed text and scanned pages** | Always pass `--skip-text`. Born-digital text pages are copied untouched; scanned/image pages receive the invisible OCR text layer. **Never** pass `--force-ocr` or `--redo-ocr` by default and do not expose them in the first-scope UI. |
| **Page ranges** | Process the entire document in v1 (do not pass `--pages`). A future range option would have to document explicitly that unselected pages are copied without OCR. |
| **Page rotation** | Do **not** pass `--rotate-pages` or `--deskew` in v1 (avoids requiring `tesseract-osd` or `unpaper` and avoids unintended orientation changes on Arabic/mixed pages). Existing PDF `/Rotate` metadata is preserved by OCRmyPDF. |
| **Page order and page size** | Preserved verbatim from the source PDF (`MediaBox`, `CropBox`, and original page sequence). `--optimize 0` and `--output-type pdf` prevent unsolicited image recompression and archival rewrites. |
| **Unavailable language** | Preflight requested `lang` (`eng`, `ara`, or `ara+eng`) component-by-component through the S-145/S-146 `tesseract --list-langs` probe before spawning `ocrmypdf`, returning `UnavailableReason::MissingLanguageData { language }` (and `EngineError::Unavailable` on direct execution). Never guess or fall back to another language. |
| **Per-page timeout / incomplete run** | `--tesseract-timeout 180` stops Tesseract on a stuck page, but upstream OCRmyPDF skips OCR on that page, logs a warning, and can still exit 0. The adapter must inspect stderr for per-page timeout or skipped-page warnings and **fail the task with no committed output** rather than reporting an incompletely OCRed document as unqualified success. |

### Exact child argv

Every argument is passed as a separate `OsString` (never joined through a shell). Paths are absolute inside the task's private work directory so leading `-`, spaces, or Unicode characters cannot be parsed as flags:

```text
ocrmypdf
  --language <lang>
  --skip-text
  --output-type pdf
  --optimize 0
  --jobs 1
  --tesseract-timeout 180
  --quiet
  --
  <abs_input_pdf_path>
  <private_work_dir>/searchable.pdf
```

- `<lang>` is strictly validated against the closed enum `eng`, `ara`, `ara+eng`.
- `--output-type pdf` explicitly disables PDF/A conversion. **Consequence:** OCRmyPDF 14.4 to 16.x will not default to `--output-type pdfa` (PDF/A-2b) and OCRmyPDF 17.x will not vary output format under `--output-type auto` depending on Ghostscript presence. The result is an ordinary PDF that preserves source structure and avoids `ExitCode.pdfa_conversion_failed` (10), with no PDF/A compliance claim.
- `--optimize 0` disables post-processing image optimization so `pngquant` and `jbig2enc` are not required and original visual image streams are not lossily recompressed.
- `--jobs 1` restricts OCRmyPDF to one internal worker, preventing CPU/RAM spikes alongside Kawaro's single-child OCR concurrency permit.
- `--quiet` suppresses interactive progress-bar output so bounded stdout/stderr drains stay clean while errors and warnings remain capturable. (Exact stderr warning capture under `--quiet` vs default verbosity must be verified against the qualified OCRmyPDF version in the future check stage.)

### Child-tree cancellation, timeouts, temp limits, and no partial commit

- **Child-tree cancellation:** Spawn through Kawaro's shared `ProcessTree` (`crates/engines/src/process.rs`) and bounded child runner (`crates/engines/src/ocr/child.rs`, `run_bounded`). On Unix the child leads a new process group (`process_group(0)`) killed with `killpg(pgid, SIGKILL)`; on Windows it runs inside a kill-on-close Job Object. Every descendant (`tesseract`, `gs`, Python worker) is terminated and reaped on cancel, timeout, or exit, and both stdout/stderr reader threads are joined before returning.
- **Concurrency permit:** Share the single-child OCR permit with `OcrEngine` (`crates/engines/src/ocr.rs`) with cancellable permit waiting so image OCR and PDF OCR never run concurrently inside one daemon process.
- **Timeout policy (proposed initial bounds, not measured guarantees):**
  - Read-only probes (`ocrmypdf --version`, `tesseract --list-langs`): `5 s` wall-clock timeout, `64 KiB` stdout cap (overflow is an error, never silent truncation).
  - Per-page OCR budget: `--tesseract-timeout 180` (`180 s` per page).
  - Whole-job budget: `600 s` (`10 minutes`) enforced by `run_bounded`; timeout kills the process tree, cleans temp files, and returns `EngineError::Failed`.
- **Temp-file isolation and size bounds (under existing temp policy):**
  - Allocate one private task work directory (`kawaro-ocrmypdf-*`) via `work_dir` (`crates/engines/src/office.rs`), honoring Kawaro's configured temp root (consistent with `CLAUDE.md`'s rule never to exhaust a small `/tmp` tmpfs).
  - Create `<work>/tmp` and set child environment variables `TMPDIR`, `TEMP`, and `TMP` to `<work>/tmp` so OCRmyPDF, Ghostscript, and Tesseract write all intermediate rasters and temp PDFs inside the task's private directory. Dropping `work` deletes every temporary file on success, failure, cancellation, or timeout.
  - Proposed admission and output policy bounds (initial policy values, not hard RSS or disk-quota guarantees): max input size `128 MiB`, max input page count `250`, max output PDF size `256 MiB` (polled during execution via the `run_bounded` poll hook and strictly checked before moving to `task.output`), stdout discarded in `8 KiB` chunks during OCR, stderr tail capped at `4 KiB` (`STDERR_KEEP`).
- **No partial commit:** OCRmyPDF never writes to `task.output` or the final path directly. Transfer to `task.output` happens only after exit code 0, zero per-page-timeout warnings, verified non-empty `%PDF-` output within the `256 MiB` cap, and a final `cancel.check()?`. Any error, cancel, or timeout removes `<work>` and `task.output`, leaving zero partial files next to the original.

## Arabic acceptance matrix for a future check stage (NOT EXECUTED)

> **Status notice:** None of the cases below have been executed. They define the mandatory verification gate for a future check stage after implementation and before any Arabic searchable-PDF quality claim may be made. Because OCRmyPDF 15.x/16.x (`sandwich`/`hocr`) and OCRmyPDF 17.x (`fpdf2`) differ substantially in Arabic RTL text-layer rendering, every run must record the exact versions of `ocrmypdf`, `tesseract`, `ara.traineddata`, and `ghostscript` / `pypdfium2`.

| Check area | Future fixture input | Required observable verification (not executed) |
| --- | --- | --- |
| **1. Searchable words** | Clean printed Arabic scanned PDF with approved ground truth | Searching known whole Arabic words and substrings in a PDF viewer and via `pdftotext` finds the expected hits on the expected page. |
| **2. Logical copy order** | Multi-line Arabic paragraph scan | Copying text from the output PDF produces logical Unicode character and word order (not reversed visual glyph order or broken isolated presentation forms). |
| **3. Mixed Arabic, Latin, and numbers** | Scanned page with Arabic sentences, embedded English words, Western digits (0 to 9), and Arabic-Indic digits | Copied and extracted text preserves logical bidirectional run order and keeps exact recognized digits and characters without silent normalization. |
| **4. Diacritics** | Vocalized Arabic printed scan containing tashkeel | Recognized diacritics are retained as returned by the engine without stripping; actual recognition fidelity and omissions are recorded honestly without blanket accuracy claims. |
| **5. Multi-column layout** | Two-column Arabic printed scan | Text copied or extracted follows column reading order (right column top-to-bottom, then left column) rather than merging lines across columns; any renderer limitation is documented explicitly. |
| **6. Rotated scans** | 90, 180, and 270 degree rotated Arabic pages (both `/Rotate`-flagged pages and sideways image scans) | Original page boxes and `/Rotate` flags are preserved; verify whether text placement aligns with visible glyphs and document sideways-scan limitations under the v1 no-auto-rotate policy. |
| **7. Existing-text pages** | Mixed PDF containing born-digital Arabic/English pages plus scanned Arabic pages | Born-digital pages retain their original searchable text verbatim under `--skip-text`, only scanned pages receive OCR, and page count, order, and dimensions match the source. |
| **8. Visual-content-unchanged check** | Scanned and mixed PDFs with images, vector lines, and text | Rasterized page renders of the output PDF visually match the source PDF (no forced rasterization of vector pages, no visible blur/recompression under `--optimize 0` and `--output-type pdf`), and original input file SHA-256 is byte-identical before and after. |
| **Safety controls** | Digitally signed PDF, encrypted PDF, corrupt file, missing `ara` traineddata, mock `ocrmypdf 14.0.1`, mid-run cancel, per-page timeout | Signed/encrypted/corrupt refused with `EngineError::Input`; missing `ara` yields `MissingLanguageData`; `< 14.4.0` refused by probe; cancel and per-page timeout leave zero temp or committed files and zero surviving child processes. |

Fixtures must carry documented provenance and redistribution rights in a versioned manifest.

## Limitations and explicit non-claims

- **No PDF/A claim:** Explicit `--output-type pdf` produces standard PDFs, not PDF/A-1, PDF/A-2, or PDF/A-3 archival files.
- **No PDF/UA or accessibility claim:** Invisible OCR text layers do not constitute PDF/UA compliance, logical tag trees (`StructTreeRoot`), or screen-reader certification.
- **No archival or digital-signature preservation claim:** Adding an OCR text layer modifies the PDF byte stream and inherently invalidates cryptographic signatures. Signed PDFs are refused in v1.
- **No encrypted-PDF handling:** Password-protected PDFs are refused in v1 with no password prompt.
- **No unmeasured Arabic quality or performance claim:** Arabic word accuracy, copy-paste order across PDF viewers, runtime latency, and peak RSS remain unmeasured until the deferred check stage executes.
- **No native Windows guarantee in v1:** Linux distribution packages are the primary target; Windows remains optional manual setup in the later platform phase.

## Final decision and smallest viable dependency set (2026-11-21)

- **Decision:** **Adopt** the optional system `ocrmypdf >= 14.4.0` child-process architecture for a future searchable-PDF feature, recorded in proposed [ADR 0017](../adr/0017-searchable-pdf-ocr.md) and scoped into draft story [S-150](../../backlog/S-150-searchable-pdf-ocr.md) (`DRAFT, not started`).
- **Rationale:**
  1. Preserves all Kawaro non-negotiable principles: strictly local processing, zero base-install growth (zero new crates, zero bundled binaries or models), original file untouched, atomic single-output queue finalization, and full process-tree isolation.
  2. Avoids building a fragile custom PDF text-layer writer or destructively rasterizing born-digital PDFs through image OCR.
  3. Enforcing the `>= 14.4.0` version floor closes the pre-14.4 silent signature-invalidation hazard while remaining natively packaged in current Fedora (42 to 44), Debian 13 Trixie, and Ubuntu 24.04 LTS repositories.
  4. Licensing is clean under Kawaro's external-process boundary (zero linking, zero redistribution).
  5. Open Arabic RTL renderer differences (v15/16 vs v17 `fpdf2`) are managed honestly through an explicit pre-claim Arabic acceptance gate rather than unverified promises.
- **Smallest viable external dependency set (system-installed by the user, never bundled by Kawaro):**
  1. `ocrmypdf >= 14.4.0` (MPL-2.0) and its distro Python runtime packages including `python3-pikepdf` (MPL-2.0) / `qpdf` (Apache-2.0).
  2. `tesseract` (Apache-2.0, with Leptonica BSD-2-Clause) and the requested language pack (`ara` and/or `eng`, Apache-2.0 upstream).
  3. One PDF rasterizer: `ghostscript` (AGPL-3.0-or-later) on OCRmyPDF 14.4 to 16.x, or `pypdfium2` (Apache-2.0 OR BSD-3-Clause) / `ghostscript` on OCRmyPDF 17.x.
  4. Not required: `unpaper`, `jbig2enc`, `pngquant`, `tesseract-osd`.

## Primary sources consulted (accessed 2026-11-21)

| Source | URL | Date accessed | What it establishes |
| --- | --- | --- | --- |
| OCRmyPDF `pyproject.toml` | https://github.com/ocrmypdf/OCRmyPDF/blob/main/pyproject.toml | 2026-11-21 | Project metadata and `MPL-2.0` licence declaration |
| OCRmyPDF `LICENSE` | https://github.com/ocrmypdf/OCRmyPDF/blob/main/LICENSE | 2026-11-21 | Mozilla Public License 2.0 text |
| OCRmyPDF Installation docs | https://ocrmypdf.readthedocs.io/en/stable/installation.html | 2026-11-21 | System dependencies, Fedora/Debian/Ubuntu install packages, Windows WSL/manual native steps, optional `unpaper`/`jbig2enc`/`pngquant` |
| OCRmyPDF Cookbook | https://ocrmypdf.readthedocs.io/en/stable/cookbook.html | 2026-11-21 | `--sidecar` omits existing text and skipped pages; digital-signature refusal default and `--invalidate-digital-signatures`; encrypted PDF refusal; `--rotate-pages` |
| OCRmyPDF Release Notes | https://ocrmypdf.readthedocs.io/en/stable/release_notes.html | 2026-11-21 | 14.4.0 digital-signature refusal default and `rich` progress bar; 17.0.0 optional Ghostscript via `pypdfium2`, `--output-type auto`, and default `fpdf2` renderer for RTL/Arabic |
| OCRmyPDF Optimizer and Performance docs | https://ocrmypdf.readthedocs.io/en/stable/optimizer.html and https://ocrmypdf.readthedocs.io/en/stable/performance.html | 2026-11-21 | `--optimize 0` disables optimization; `--jobs` concurrency and memory scaling; `--tesseract-timeout` (default 180 s, skips page with warning); `TMPDIR` temp directory usage |
| OCRmyPDF Exit Codes and API docs | https://ocrmypdf.readthedocs.io/en/stable/errors.html | 2026-11-21 | `ExitCode` values (0 ok, 1 bad_args, 2 input_file / `DigitalSignatureError`, 3 missing_dependency, 4 invalid_output_pdf, 6 already_done_ocr, 8 encrypted_pdf, 10 pdfa_conversion_failed) |
| Fedora `ocrmypdf` package | https://packages.fedoraproject.org/pkgs/ocrmypdf/ocrmypdf/ | 2026-11-21 | Fedora 42 to 45 versions (`16.7.0+` to `16.13.0+`) and runtime dependencies (`ghostscript >= 9.15`, `tesseract >= 4.1.1`, `python3-pikepdf >= 10`) |
| Fedora `tesseract-langpack-ara` and `unpaper` packages | https://packages.fedoraproject.org/pkgs/tesseract-tessdata/tesseract-langpack-ara/ and https://packages.fedoraproject.org/pkgs/unpaper/unpaper/ | 2026-11-21 | Official Fedora language pack and optional `unpaper` availability (`jbig2enc` absent from Fedora repos) |
| Debian `ocrmypdf` packages (Bookworm and Trixie) | https://packages.debian.org/bookworm/ocrmypdf and https://packages.debian.org/trixie/ocrmypdf | 2026-11-21 | Bookworm `14.0.1+dfsg1-1` (below 14.4.0 floor) vs Trixie `16.7.0+dfsg1-3` and Python/system dependencies |
| Ubuntu `ocrmypdf` package (Noble 24.04 LTS) | https://packages.ubuntu.com/noble/ocrmypdf | 2026-11-21 | Noble universe version `15.2.0+dfsg1-1` and dependencies (`ghostscript`, `tesseract-ocr`, `python3-pikepdf`, optional `unpaper`, `jbig2`) |
| Tesseract `LICENSE` and Installation docs | https://github.com/tesseract-ocr/tesseract/blob/main/LICENSE and https://tesseract-ocr.github.io/tessdoc/Installation.html | 2026-11-21 | Tesseract `Apache-2.0` licence and language-pack naming |
| Tesseract `tessdata` `LICENSE` | https://github.com/tesseract-ocr/tessdata/blob/main/LICENSE | 2026-11-21 | Upstream `tessdata` `Apache-2.0` licence |
| Leptonica licence | https://github.com/DanBloomberg/leptonica/blob/master/leptonica-license.txt | 2026-11-21 | `BSD-2-Clause` licence |
| Ghostscript releases and `LICENSE` | https://ghostscript.com/releases/gsdnld.html and https://github.com/ArtifexSoftware/ghostpdl/blob/master/LICENSE | 2026-11-21 | `AGPL-3.0-or-later` / Artifex commercial dual licence |
| pikepdf `LICENSE.txt` | https://github.com/pikepdf/pikepdf/blob/main/LICENSE.txt | 2026-11-21 | `MPL-2.0` licence |
| qpdf `LICENSE.txt` | https://github.com/qpdf/qpdf/blob/main/LICENSE.txt | 2026-11-21 | `Apache-2.0` licence |
| unpaper `COPYING` | https://github.com/unpaper/unpaper/blob/main/COPYING | 2026-11-21 | `GPL-2.0-only` licence |
| jbig2enc `COPYING` | https://github.com/agl/jbig2enc/blob/master/COPYING | 2026-11-21 | `Apache-2.0` licence |
| pypdfium2 and fpdf2 licences | https://github.com/pypdfium2-team/pypdfium2 and https://github.com/py-pdf/fpdf2/blob/master/LICENSE | 2026-11-21 | `Apache-2.0 OR BSD-3-Clause` (`pypdfium2`) and `LGPL-3.0-or-later` (`fpdf2`) for OCRmyPDF 17.x |
