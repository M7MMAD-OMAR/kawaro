# S-126 PDF to text with layout, through poppler pdftotext
Status: done (2026-11-04)

## Story
As a user, I want the text of a PDF saved next to it as a plain UTF-8 text file with the page layout kept as far as the format allows, so that I can search, copy and reuse what a PDF contains without opening a viewer. This is P0 item 6 of `docs/research/pdf-feature-benchmark.md` (PDF to plain text with layout, through poppler pdftotext as a child process). It is extraction, not OCR: a scanned page has no text to extract, and that is expected in this story.

## What this story will prove
- A small child process adapter behind the existing `Engine` trait can add a real capability without linking the library: poppler stays a separate program, as Ghostscript does, and nothing AGPL is linked.
- A missing `pdftotext` is a first class, honest state: the action is hidden from the hive and from `kawaro_plan`, `kawaro doctor` says it is missing, and a direct run gives a message that names the program and the package to install, never a generic failure.
- `-layout` output is useful for the common cases (prose, forms, simple two column pages) and honest about the rest: the story records where column and table order is imperfect instead of promising a reconstruction it does not do.
- The Arabic risk is measured, not assumed: the fixture set from the benchmark tells which producers store logical order and which store visual order, and the story documents the difference instead of silently returning reversed text.
- Page ranges reduce transfer and output size for long documents, with one small, strict grammar rather than a second page selection language.

## Dependencies
- An ADR for a new engine id, `poppler` (the benchmark section 10 lists any new engine id as needing an ADR), and an owner decision that a second child system program may join the base install. Without both, the story waits.
- The licence line in `docs/engines.md`: poppler is GPL-2.0-or-later, compatible with GPL-3.0-or-later, used as a child process only, never linked. The program is taken from the system and never downloaded (ADR 0010 holds: only opt-in components are downloaded).
- S-117 for the option schema, and S-123 for the pages grammar: the range option reuses that parser restricted to one contiguous group instead of growing a second grammar.
- The Ghostscript adapter is the template: `locate::Program` lookup, `available()`, `run_child` in a `ProcessTree` (S-030), a timeout and a private work directory. The probe, `kawaro doctor` and the MCP engine list then learn about the engine with no MCP work.
- A fixture set from the benchmark section 6: a LibreOffice Arabic text PDF, a scanned book page, a two column page, a page with a table, a form with Arabic labels, a right to left page with Latin inserts, and one encrypted file.

## Proposed decisions (fixed on approval)
1. Engine id `poppler`, op `to-text`, program `pdftotext` (or `pdftotext.exe` on Windows), registered in `kawaro_engines::all()`, with `available()` true only when the program is found. No library is linked and no network is used.
2. One action `pdf.to-text`: `engine = "poppler"`, `op = "to-text"`, `ring = 1`, `order = 50`, `accepts = ["pdf"]`, `min_files = 1`, output `{ mode = "per-file", ext = "txt", suffix = "-text" }`, code `ar = "نص"` and `en = "TEXT"`, icon `text`, bilingual label, and `[action.options]` with `kind = "form"` and one `pages` string field (default empty, `maps_to = "pages"`, bilingual label).
3. The command is `pdftotext -layout -enc UTF-8 [-f FIRST -l LAST] <input> <output>` in a private work directory, and the finished text is moved to `task.output`; the core queue keeps the naming and never overwrite rule. The timeout is the existing `timeout_s` pattern with a 300 s default.
4. The range option is empty (all pages), `N`, or `N-M`, 1-based, `N <= M`, positive, one contiguous group only. Multi group selections such as `1,3-5` are `BadParam` named `pages` with a message that this action takes one range; the grammar itself comes from the S-123 parser.
5. Output is UTF-8 without a byte order mark, with LF line endings (CRLF is normalised on Windows), and the page separator form feed that pdftotext writes is kept as the tool writes it, with no trailing form feed added.
6. Exit codes, a timeout, a killed child and cancellation map to the existing `EngineError` variants; a non-zero exit is `Failed` with the last stderr lines, and a missing program is `Unavailable` with the install hint. No partial text file is left at `task.output` on any failure.

## Acceptance criteria (draft, to be fixed when the story is approved)
- AC1 (engine): a `PopplerEngine` with id `poppler` is in `kawaro_engines::all()`; `available()` is true exactly when `pdftotext` is found by `locate`; `capabilities()` declares `to-text` for `MediaKind::Pdf`; an unknown op is `UnsupportedOp`.
- AC2 (action and option): the action and its single `pages` field match decision 2; `OptionsSpec::validate` passes; labels are Arabic and English; the registry count grows by one and the S-117 count test is updated by the tester.
- AC3 (layout): on a prose fixture, the output equals what `pdftotext -layout -enc UTF-8` writes for the same file, line for line, with the whitespace of the layout preserved; this is checked by running the real tool once when it exists, and against a recorded expected file otherwise.
- AC4 (ranges): empty gives every page; `3` gives page 3 only; `2-4` gives pages 2 to 4; the child is invoked with `-f` and `-l` matching the range; `0`, `3-1`, `1,3`, `1-` and a non digit are `BadParam` named `pages` with no output file.
- AC5 (missing tool): with no `pdftotext` on the machine, the action is absent from the hive and from `kawaro_plan`; `kawaro doctor` prints a `pdftotext` line saying it is missing; a direct run gives `Unavailable` whose message contains `pdftotext`, the package name and a URL; no output file is created. The MCP surface reports the engine unavailable and a direct call gives `ENGINE_MISSING`.
- AC6 (encoding and line endings): the output starts without a byte order mark, contains no CRLF and no `\r`, and is valid UTF-8; Latin text and Western digits round trip unchanged.
- AC7 (Arabic and RTL, measured): each Arabic fixture from the dependency list is run and the output is recorded. For the LibreOffice Arabic text PDF the expected Arabic words appear in logical order and the file is valid UTF-8. For fixtures whose producer stores visual order or whose subset fonts lack a ToUnicode map, the test records the raw order and the story documents the limitation; no bidi reordering or searchability claim is made. The scanned page fixture produces an empty or whitespace only text file and the action still succeeds; that is the OCR boundary, and OCR is a different story.
- AC8 (encrypted and malformed): an encrypted PDF gives `EngineError::Input` with the password reason and no password prompt; a non PDF gives `EngineError::Input`; a malformed file that pdftotext rejects gives `Failed` with the stderr tail; no output file remains in any case.
- AC9 (safety and naming): the input is never modified; the only output is `task.output`, written through the private work directory and the core placement, so a second run takes a free name and never overwrites; progress starts at 0 and ends at 1.0; a pre-cancelled token returns `Cancelled` with no output; the child runs in a `ProcessTree` and nothing it starts survives the task.
- AC10 (docs, licence and gates): `docs/engines.md` gains the poppler row (version checked from the system, GPL-2.0-or-later, child only, purpose) before the code lands; the install hint names the package for Debian and Ubuntu and Fedora; `cargo fmt --check`, clippy on `kawaro-engines` with `-D warnings` and the tests of `kawaro-engines`, `kawaro-core` and `kawaro-cli` pass; no new dependency, no network, MSRV 1.85.

## Test cases (draft)
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | Engine registration, `available()` with a fake program and with none, capabilities and dispatch | unit |
| AC2 | Read `actions/pdf.toml` through `ActionDef`: action fields, the `pages` option, `validate()`, bilingual labels | unit |
| AC3 | Real `pdftotext` run on the prose fixture compared with the recorded output; skipped with a printed notice when the tool is missing | integration |
| AC4 | Range table through a fake `pdftotext` script that records its arguments; bad ranges leave no output | unit |
| AC5 | Fake missing program: `Unavailable` text, doctor line, plan exclusion | unit |
| AC6 | BOM, CRLF and UTF-8 checks on a controlled text fixture | unit |
| AC7 | The seven fixture set, recorded output, Arabic word check on the LibreOffice file, scanned page empty | integration, manual review |
| AC8 | Encrypted, non PDF and malformed input | unit |
| AC9 | Input bytes before and after, second run naming, progress shape, pre-cancelled token, no orphan child | unit |
| AC10 | `docs/engines.md` row and install hint, then the cargo gates | gate |

## Out of scope
OCR and any searchable text layer (OCRmyPDF or Ghostscript OCR devices are their own story), PDF to Markdown, DOCX, XLSX or PPTX, bidi visual to logical reordering, exact reconstruction of columns, tables or forms, page thumbnails, password entry or decryption, PDF/A, text extraction in the UI, a dedicated MCP tool or CLI subcommand, packaging poppler or pinning a version, a Windows or macOS check in this story, and any new engine download or model. A new engine id and a second child program are decisions for the ADR and the owner first.

## Review log
- 2026-11-04, planner: draft written from the P0 PDF to text item in `docs/research/pdf-feature-benchmark.md` section 8 and the Arabic and RTL risks in section 6 (visual order in content streams, subset fonts without a ToUnicode map, searchability mattering more than display, the minimum fixture set, and OCR quality being a separate problem). Open before approval: the ADR for the new `poppler` engine id, the owner decision on a second child system program, the package names in the install hint, and how far the Arabic fixture expectations go before they become a reordering feature. Not implemented.
- 2026-11-04, planner: story moved to in progress for this block. ADR 0013 records the `poppler` child engine, the fixed `pdftotext -layout -enc UTF-8` command with `-f` and `-l`, the UTF-8 without BOM and LF output, the `Unavailable` and doctor path, the Arabic and RTL limits from research section 6, and the no OCR boundary. S-117 and S-123 are in the tree, so the one `pages` range option reuses the page grammar, and the action and option counts it changes (50 built-in, 14 with options) go to the tester with the pinned tables named in AC9 and AC10.

- 2026-11-04, mcp-impl: ADR 0013, PopplerEngine with locate::Program::Pdftotext, pdf.to-text action, doctor line, docs and 7 integration tests plus 3 unit tests; real pdftotext 26.01 runs.
- 2026-11-04, tester: pdf_actions 10 rows with the poppler engine set, s117 50 actions and 14 option actions; engines, core, cli and clippy green.
- 2026-11-04, mcp-impl: kawaro_pdf maps all ten PDF ops including to_text; MCP tests green.
- 2026-11-04, lead: docs and CHANGELOG updated; Arabic limits stay explicit.
