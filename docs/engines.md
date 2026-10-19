# Engines

One section per engine. Each track owns its own section; append, do not
rewrite other sections. Every new library is listed with its license.

## office

Engine id `office` (`crates/engines/src/office.rs`, story S-009). Runs
LibreOffice headless as a child process.

| op | inputs | output (from `task.output` extension) |
| --- | --- | --- |
| `to-pdf` | doc docx odt rtf txt md xls xlsx ods csv ppt pptx odp | pdf |
| `convert-doc` | doc docx odt rtf txt md | docx or odt |
| `convert-sheet` | xls xlsx ods csv | xlsx or ods |
| `convert-slides` | ppt pptx odp | pptx or odp |

Params: `timeout_s` (seconds, default 180, any positive number).

Behaviour:

- Program: `soffice`, then `libreoffice`, searched once per process in `PATH`.
  `available()` is false when neither exists. Markdown is supported: the
  LibreOffice 26.2 tested here imports `.md` as a real Writer document.
- Each run creates a private work directory in the system temp dir holding a
  copy of the input (`in.<ext>`), a throwaway user profile passed with
  `-env:UserInstallation=file:///...`, and `out/`. The copy means LibreOffice
  never writes `.~lock` files next to the original and odd file names cannot
  be read as options. The private profile means a run never collides with the
  owner's open LibreOffice and never touches their profile. The whole
  directory is deleted after success, failure, cancel and timeout.
- Office Open XML targets use explicit filters (`MS Word 2007 XML`,
  `Calc MS Excel 2007 XML`, `Impress MS PowerPoint 2007 XML`).
- The exit code is not trusted: LibreOffice can exit 0 after a load failure.
  The task fails unless `out/in.<target>` exists and is not empty.
- The result is renamed to `task.output`, or copied when the temp dir is on
  another file system.
- Progress: 0.0 at start and 1.0 at the end; LibreOffice reports nothing in
  between.
- An input of the wrong family (a spreadsheet given to `convert-doc`) is an
  input error. The action schema has no "only these extensions" field, so
  `actions/documents.toml` hides conversions with `skip_ext` listing the other
  families; a mixed selection can still offer one, and the engine then fails
  the files that do not fit.

### Child processes (shared with ghostscript)

`run_child` in `office.rs` starts the tool in its own process group
(`process_group(0)`), streams stdout lines to the caller, keeps the last
4 KiB of stderr for error messages, and polls every 25 ms. Cancel and timeout
send `SIGKILL` to the whole group, which takes the LibreOffice launcher,
`oosplash` and `soffice.bin` down together. The group is also killed after a
normal exit so nothing outlives the task. On Windows the fallback is
`Child::kill` on the direct child only.

## pdf

`available()` is per engine, not per op, so PDF work is two engine ids, both in
`crates/engines/src/pdf.rs` (story S-010):

| engine | op | params | inputs | output |
| --- | --- | --- | --- | --- |
| `pdf` | `merge` | none | n, in selection order | pdf |
| `pdf` | `rotate` | `degrees` 90 (default), 180, 270 | 1 | pdf |
| `pdf` | `split` | none | 1 | zip of `page-001.pdf`... |
| `ghostscript` | `compress` | `preset` screen, ebook (default), printer; `timeout_s` (300) | 1 | pdf |
| `ghostscript` | `to-images` | `dpi` 36..600 (default 150); `timeout_s` (300) | 1 | zip of `page-001.png`... |

`pdf` is pure Rust on lopdf and always available. `ghostscript` runs `gs`
(`gswin64c`/`gswin32c` on Windows) and is hidden when it is missing.

Decisions:

- **split and to-images write one ZIP.** The engine contract writes exactly
  one file per task to a temp path that the queue renames atomically. A folder
  of pages would need a second output kind in core (a directory rename, name
  collision rules for a folder, partial cleanup), which is a contract change
  needing an ADR. A ZIP fits the existing contract and keeps the atomic
  rename. Entries are stored, not deflated: PDF pages and PNG are already
  compressed. Names are zero padded to at least 3 digits and wider for 1000
  pages or more, so they sort in page order.
- **compress keeps the original when it does not shrink.** If Ghostscript's
  result is not strictly smaller, the input is copied to `task.output` and the
  task still succeeds, so the user never gets a larger "compressed" file.
- **Inherited attributes are flattened.** Before merge, rotate and split,
  `MediaBox`, `CropBox`, `Resources` and `Rotate` inherited from the page tree
  are copied onto each page, so pages keep their size and fonts in the new
  tree and rotation adds to the effective value.
- **rotate** adds `degrees` to each page's effective `/Rotate`, modulo 360.
- **merge** drops the inputs' catalogs, outlines and page tree nodes and
  builds one new page tree; unreachable objects are pruned.
- **split** builds each page's PDF from a copy of the document with the page
  tree cut to that page and `Outlines`, `StructTreeRoot` and `OpenAction`
  removed, then prunes unreachable objects.
- Ghostscript always runs with `-dSAFER -dBATCH -dNOPAUSE`, an absolute input
  path (never read as an option), and `%` escaped in output paths. Progress
  comes from its `Processing pages 1 through N.` and `Page n` lines.

Limitations:

- Encrypted PDFs that lopdf cannot open, or that stay encrypted after
  loading, are rejected as input errors; there is no password prompt.
- split copies the whole document once per page in memory; a very large PDF
  with many pages is slow. A page whose links point at other pages keeps
  those pages as extra objects in its file (not shown as pages).
- merge does not carry bookmarks, forms (AcroForm) or tagged structure.

## Libraries and tools

| name | used by | license | notes |
| --- | --- | --- | --- |
| LibreOffice | office | MPL-2.0 | external program, not linked |
| Ghostscript | ghostscript | AGPL-3.0 | external program, not linked |
| lopdf 0.45 | pdf | MIT | `default-features = false`; declares rust-version 1.88, above the workspace's 1.85 |
| zip 9 | pdf, ghostscript | MIT | `default-features = false` (stored only); tests add `deflate-flate2` to read LibreOffice archives |
| libc 0.2 | office, ghostscript | MIT OR Apache-2.0 | unix only, for `killpg` |
| tempfile 3 | office, ghostscript | MIT OR Apache-2.0 | private work directories |

lopdf's own dependencies (flate2, aes, cbc, ecb, md-5, sha2, nom, indexmap,
weezl, rangemap, stringprep, encoding_rs, brotli-decompressor) are MIT,
Apache-2.0 or BSD-3-Clause, all compatible with GPL-3.0-or-later.
