# S-125 PDF metadata: clear it, or set title, author, subject and keywords
Status: done (2026-11-04)

## Story
As a user who shares a PDF, I want to clear its metadata or set a clean title, author, subject and keywords, so that the file carries what I choose and not the authoring program's leftovers. This is P0 item 5 of `docs/research/pdf-feature-benchmark.md` (metadata read, write and strip on lopdf), behind the existing pure Rust `pdf` engine. One new action, one new op, no new engine and no new dependency.

## Facts found while planning
- The `pdf` engine (`crates/engines/src/pdf.rs`) already loads a document with `load_pdf` (which refuses an encrypted file and flattens inherited page attributes) and writes it with `save` to `task.output`. It handles merge, rotate, split, extract-pages, delete-pages and reorder-pages; metadata is the next lopdf op in the same file.
- The document Info dictionary is reached through the trailer `/Info` key and the XMP stream through the catalog `/Metadata` key. lopdf writes a text string either as a literal string or as a hexadecimal string; a non-ASCII value such as an Arabic title must be written as a PDF text string, which is UTF-16BE with a byte order mark, in the hexadecimal form.
- Readers disagree about which source wins: some show the Info values, some prefer XMP. Leaving stale XMP after a set would show the old title in those readers, so a write removes the XMP stream and lets the Info dictionary be the single source. This is a decision of this story, recorded in the review log.
- The option schema from S-117 is in the tree. A block with fields needs `kind = "form"` (ADR 0012 decision 6). The engine parses validated values from `task.params`, so a hand written TOML, the CLI `--set` of S-120 and the MCP `options` argument reach it the same way.
- After S-123 the registry has 48 built-in actions and 12 of them carry options. This story adds one action with options, so the registry has 49 and the option count is 13; the S-117 test that counts them is updated by the tester and its story is not rewritten.
- The action output is a new PDF next to the original, named by the core queue with the `-metadata` suffix and the next free number on a collision. The input is never modified.

## Acceptance criteria
- AC1 (engine op): `PdfEngine::capabilities()` includes `metadata` with `MediaKind::Pdf`, `run` dispatches it, an unknown op still gives `EngineError::UnsupportedOp`, and the op takes exactly one input and writes `task.output`. Progress starts at 0, never decreases and ends at 1.0.
- AC2 (set fields): with `clear = false` (or absent) and non-empty `title`, `author`, `subject` and `keywords`, the output PDF's Info dictionary carries exactly those four values; any other Info key of the input (for example `Creator`, `Producer`, `CreationDate`) is preserved with its value. The page count, page text, MediaBox and inherited page attributes are unchanged, and the output loads with lopdf.
- AC3 (leave unchanged): an absent field, or a field whose resolved value is the empty string, leaves the input value unchanged. `title = ""` does not delete an existing title; clearing a single field is not a feature of this story, `clear = true` removes them all. With nothing set and `clear = false` the action still writes a new PDF that is a rewrite of the input with the same metadata and the same pages.
- AC4 (clear): `clear = true` removes the trailer `/Info` entry, the catalog `/Metadata` entry and the XMP stream they referenced, and prunes the document so the old metadata objects are gone. The output has no Title, Author, Subject or Keywords in Info and no remaining XMP metadata; pages, text and MediaBox are unchanged and the output loads.
- AC5 (mutual exclusion): `clear = true` together with any non-empty text field (in the raw params, because the options clear one another) is `EngineError::BadParam` whose `name` is `clear` and whose reason names the conflicting field and says it cannot be combined with `clear`; no output file is left. `clear = false` with text fields is the normal set path.
- AC6 (encoding): a value that is plain ASCII is written as a PDF literal string; a value with any non-ASCII character (for example an Arabic title or a Latin author name with an accent) is written as a hexadecimal UTF-16BE text string with the `FE FF` byte order mark. Reading the output back gives the exact original string for both cases, and no replacement characters appear.
- AC7 (bad values, no output): a non boolean `clear` (`clear = "yes"`, `clear = 1`) and a non string text field (`title = 5`, `author = true`) are `EngineError::BadParam` with the field name in `name`, a non empty reason, and no output file. A missing input file or a non PDF input is the existing `EngineError::Input` path with the file name.
- AC8 (action contract): `actions/pdf.toml` gains exactly one action, `pdf.metadata`: `engine = "pdf"`, `op = "metadata"`, `ring = 2`, `order = 50`, `accepts = ["pdf"]`, `min_files = 1`, `output = { mode = "per-file", suffix = "-metadata" }` (the extension stays PDF), code `ar = "بيانات"` and `en = "META"`, a bilingual label, icon `file`, `[action.params]` with `clear = false`, `title = ""`, `author = ""`, `subject = ""` and `keywords = ""`, and `[action.options]` with `kind = "form"` and exactly five fields in this order: `clear` (`bool`, default false, `clears = ["title", "author", "subject", "keywords"]`), `title`, `author`, `subject` and `keywords` (each `type = "string"`, default empty, `maps_to` its param key, `clears = ["clear"]`), all bilingual. `OptionsSpec::validate` passes for the action and no other action changes.
- AC9 (registry count and option count): `Registry::builtin()` has 49 actions and exactly 13 of them carry options: the twelve of S-124 plus `pdf.metadata`. `pdf.metadata` matches a single PDF selection through `Registry::matching` with the `pdf` engine available. The tester updates the S-117 count test from 48 to 49 and adds `pdf.metadata` to its options list; the implementer edits no test file except its own new one.
- AC10 (scope and gates): the change is limited to `crates/engines/src/pdf.rs`, `actions/pdf.toml`, the new `crates/engines/tests/pdf_metadata.rs` and the tester owned S-117 count update. The other `crates/engines/tests/pdf_*.rs` tests pass unedited, including `pdf_actions.rs` once its row table has the new action. `cargo fmt --check`, `cargo clippy -p kawaro-engines --all-targets -D warnings`, `cargo test -p kawaro-engines` and `cargo test -p kawaro-core` pass. No new dependency and no new `Cargo.lock` package, no network, MSRV 1.85, no change to core, UI, daemon, CLI, MCP, docs, `CHANGELOG.md` or Cargo files. A pre-cancelled token returns `EngineError::Cancelled` and leaves no output.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | Dispatch, `capabilities` and `UnsupportedOp`; progress shape | unit |
| AC2 | Fixture with Info (Title, Author, Subject, Keywords, Creator, Producer); set all four; read back Info and compare; pages and page text unchanged | unit |
| AC3 | Set one field, leave others unchanged; empty string leaves an existing title; nothing set gives an identical metadata state and page count | unit |
| AC4 | Fixture with Info and an XMP stream; `clear = true`; no `/Info`, no `/Metadata`, no XMP bytes, pages unchanged | unit |
| AC5 | `clear = true` with a title; `BadParam` named `clear` with the field in the reason; no output | unit |
| AC6 | ASCII title round trips; Arabic title and an accented Latin author round trip; the raw output shows a literal string for ASCII and a `FE FF` hexadecimal string for the non-ASCII value | unit |
| AC7 | Wrong types for `clear` and the text fields; missing and non PDF input | unit |
| AC8 | Read `actions/pdf.toml` through `ActionDef`: exact action fields, params, five option fields, defaults, clears and bilingual labels; `validate()` passes | unit |
| AC9 | `Registry::builtin()`: 49 actions, 13 with options, `pdf.metadata` matches one PDF; `pdf_actions.rs` table updated by the tester | unit |
| AC10 | Existing PDF tests, progress and cancel, then `cargo fmt`, clippy and the two test runs | unit, gate |

## Out of scope
Reading or displaying metadata in the UI (no metadata page in this block), XMP editing (the stream is removed, not rewritten), metadata presets, per-page metadata, forms, attachments, outlines, encryption or password handling, PDF/A, OCR and PDF to text (S-126). A `kawaro metadata` CLI or an MCP tool of its own: the action is reachable by id through `kawaro run` and `kawaro_convert`. Docs, `CHANGELOG.md`, `docs/architecture.md` and `docs/engines.md` changes.

## Review log
- 2026-11-04, planner: written from the Lead contract and P0 item 5 of `docs/research/pdf-feature-benchmark.md` (metadata read, write and strip on lopdf). Decisions recorded: (1) one action with a `clear` switch plus four text fields, with mutual `clears` between the switch and the fields, because the contract says clear or set, not both; (2) empty means leave unchanged, and a single field cannot be deleted except through `clear`; (3) a write removes the XMP stream so readers that prefer XMP do not show a stale title, and `clear` removes Info and XMP together; (4) non-ASCII values are UTF-16BE text strings, because an Arabic title is a first class case for this project. Test counts to update are named in AC9.

- 2026-11-04, mcp-impl: metadata op in crates/engines/src/pdf.rs, the pdf.metadata action in actions/pdf.toml, and 10 tests in crates/engines/tests/pdf_metadata.rs; clear removes Info and XMP, text writes UTF-16BE for non-ASCII and drops stale XMP, and clippy is clean.
- 2026-11-04, tester: pdf_actions 9 rows, pdf_engine 7 ops, s117 49 actions and 13 option actions; engines, s117 and registry suites green.
- 2026-11-04, lead: docs and CHANGELOG pending the full gate.
- 2026-11-04, lead: registry 49 actions and 13 option actions, pdf_actions 9 rows and pdf_engine 7 ops; CHANGELOG line added.
