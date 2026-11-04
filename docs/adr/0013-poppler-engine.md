# ADR 0013: PDF to text through a poppler child process

Status: accepted (owner approval 2026-11-04; implemented in S-126)
Date: 2026-11-04

## Context

Kawaro converts files locally and today handles PDFs with lopdf (merge,
rotate, split, page operations, metadata) and Ghostscript (compress, pages to
images). The PDF benchmark (`docs/research/pdf-feature-benchmark.md`, P0 item
6) asks for PDF to text with the page layout kept, so a user can search, copy
and reuse what a PDF contains without opening a viewer.

lopdf can join decoded strings, but the benchmark section 6 records how weak
that is: it is weakest with subset fonts that carry no ToUnicode map, which is
exactly the common case in Arabic PDFs from InDesign and LibreOffice, and it
does no layout work at all. Writing a pure Rust text extractor with layout is a
project of its own, so this story takes the text from an established tool.

poppler `pdftotext` is the tool the benchmark names. Poppler is
GPL-2.0-or-later, which is compatible with Kawaro GPL-3.0-or-later. Kawaro
runs it as a system program, never links it: the poppler libraries stay out of
the binary and out of the install, and the base install stays light, as with
Ghostscript and LibreOffice.

## Options

| Option | Cost | What it gives | Verdict |
| --- | --- | --- | --- |
| Link the poppler C++ libraries through a Rust binding | A C++ dependency and a build system on every target, larger binary | Direct API access | Rejected: the base install stays light and the other heavy engines are child processes too |
| Write a text extractor on lopdf | Months of layout and font work, poor Arabic results per the benchmark | No new program | Rejected: worse output than the established tool for more code |
| Run the system `pdftotext` as a child process | One more optional system program, hidden when absent | Layout text, wide format support, GPL compatible, no link | Chosen |
| Add OCR (Tesseract or Ghostscript OCR devices) at the same time | A model, preprocessing, quality claims | Text from scans | Rejected for this story: OCR is its own story (benchmark section 6 quality numbers) |

## Decision

1. **A new engine id `poppler`, one op `to-text`.** `PopplerEngine` lives in
   `crates/engines/src/poppler.rs`, is registered in `kawaro_engines::all()`
   and is `available()` exactly when `pdftotext` is found by
   `locate::find` (`Program::Pdftotext`, PATH first, then on Windows the
   known folder). Nothing is linked and nothing is downloaded.
2. **The command is fixed.** `pdftotext -layout -enc UTF-8 [-f FIRST -l LAST]
   <absolute input> -` in a killable `ProcessTree` (S-030). Standard output is
   captured, so no work directory and no typed output path are needed. The
   input path is absolute, so a name starting with `-` is never read as an
   option. `-layout` keeps the page layout as far as the format allows; the
   range flags come from the single `pages` option.
3. **The output is UTF-8 without a byte order mark and with LF line endings.**
   A leading BOM is dropped and every carriage return is removed, so the file
   is identical on every system. The form feed page separator `pdftotext`
   writes is kept as written.
4. **A missing program is a first class state.** `available()` is false, so
   the action is hidden from the hive and from `kawaro_plan`; `kawaro doctor`
   prints one `pdftotext` line saying missing; a direct run returns
   `EngineError::Unavailable` naming `pdftotext` and the poppler download
   page. The install hint in `docs/engines.md` names the package for Debian,
   Ubuntu and Fedora.
5. **The Arabic limits are documented, not reordered.** `pdftotext` returns
   text in content order. Producers that write Arabic glyphs in visual order
   give reversed or disconnected output, and subset fonts without a ToUnicode
   map give missing characters. Kawaro makes no bidi (UAX #9) reordering and
   no Arabic searchability claim for those files; the story records the raw
   order and this ADR is where the limitation is accepted. Text that is
   stored in logical order round trips unchanged.
6. **No OCR.** A scanned page has no text to extract and yields an empty or
   whitespace only file; the action still succeeds. A searchable text layer
   (OCRmyPDF or the Ghostscript OCR devices) is a different story.
7. **Failures map to the existing variants.** A non-zero exit is
   `EngineError::Failed` with the stderr tail, a timeout is `Failed`, a killed
   child is `Cancelled`, an encrypted PDF and a file that is not a PDF are
   `EngineError::Input`, and a bad `pages` value is `BadParam` named `pages`.
   No partial file is left at `task.output`: the queue keeps the naming and the
   never overwrite rule.

## Consequences

- One action, `pdf.to-text` (`engine = "poppler"`, `op = "to-text"`, ring 1,
  per-file output, extension `txt`, suffix `-text`), with one `pages` option
  (empty for every page, or one 1-based `N` or `N-M` range).
- poppler becomes the fourth child program beside FFmpeg, LibreOffice and
  Ghostscript, with the same lookup, `available()` and `doctor` treatment, and
  the engines crate gains no dependency.
- Recorded limitation: visual-order Arabic and missing ToUnicode maps produce
  content-order text, and no bidi reordering is done here.
- Recorded limitation: page count for the range check comes from lopdf only
  when a range is requested, and a file that lopdf cannot parse is left to
  `pdftotext` to report.
