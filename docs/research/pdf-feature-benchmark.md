# PDF feature benchmark and engine plan

Status: research note for the PDF expansion. Date 2026-11-04. Scope: which PDF features Kawaro should build, in what order, on which engine, under which licence.

Method: web_search was unavailable in this session, so every external fact below comes from a direct fetch of a primary page (official docs, project documentation, repository licence file) listed in Sources, all accessed 2026-11-04. Pages that could not be read are marked unverified instead of guessed. Effort sizes, memory figures and timings are estimates and are labelled as such.

## 1. Where the project stands, and the constraints that decide the plan

Shipped in v0.5.0:

| Action | Engine | Params | Known limit |
| --- | --- | --- | --- |
| pdf.compress | ghostscript (child) | preset ebook | re-distils the file, no DPI control exposed |
| pdf.merge | pdf, lopdf (pure Rust) | none | drops outlines, AcroForm, tagged structure |
| pdf.to-images | ghostscript (child) | dpi 150 | PNG only, all pages, zipped |
| pdf.rotate | pdf, lopdf | degrees 90 | page rotation only |
| pdf.split | pdf, lopdf | none | one PDF per page, zipped, copies the whole document per page |

Constraints that shape every recommendation below:

1. An engine task writes exactly one output file, so multi-file results are zipped (docs/engines.md). A folder output would be a core contract change and needs an ADR.
2. Params are fixed values in actions/*.toml and there is no per-run option UI, CLI flag or MCP argument. Per-case customization (owner question b) and every agent call (owner question a) need one declarative option schema in core first, shared by UI, CLI and MCP.
3. An engine is a pure Rust adapter or a child process behind one trait; action logic never lives in the UI.
4. Licence: GPL-3.0-or-later. Permissive code can be linked. AGPL code stays a separate process (Ghostscript is the precedent). GPL-2.0-or-later code such as poppler is compatible with GPLv3.
5. The original is never modified, output is temp then atomic rename, progress goes to the tray, no focus stealing.
6. ADR 0010: base engines are system programs and are never downloaded; only opt-in runtime components and model weights are downloadable, with a signed catalog and one explicit click.

## 2. Master feature table

P = priority for Kawaro. E = effort guess for one experienced Rust developer, including fixtures and tests: S up to 3 days, M 1 to 2 weeks, L 3 to 6 weeks, XL more. "lopdf" means pure Rust, no new process.

| Feature | Offered by | Recommended engine | E | P |
| --- | --- | --- | --- | --- |
| Merge | all platforms | lopdf, exists | done | - |
| Split every page | all | lopdf, exists | done | - |
| Split by ranges, every N pages, by size | Stirling, Sejda, iLovePDF, PDF24 | lopdf | S | P0 |
| Split by bookmarks | Sejda | lopdf outline walk | M | P2 |
| Extract, delete, reorder, insert, duplicate pages | Stirling, Sejda, iLovePDF, PDF24, pdfcpu pages | lopdf | S | P0 |
| Rotate | all | lopdf, exists | done | - |
| Crop, scale to page size, margins | Stirling, Sejda, PDF24, pdfcpu crop/resize/zoom | lopdf boxes, pdfcpu for content scaling | M | P1 |
| N-up | pdfcpu nup, Stirling, Sejda | pdfcpu child | M | P1 |
| Booklet imposition | pdfcpu booklet, Stirling, Sejda, PDF2Go | pdfcpu child | M | P1 |
| Poster, cut, grid, ndown | pdfcpu, PDF2Go | pdfcpu child | M | P2 |
| Compress presets | all | ghostscript, exists | done | - |
| Compress with DPI and downsample targets | gs parameters, iLovePDF levels | ghostscript params | S | P0 |
| Lossless object cleanup | qpdf, pdfcpu optimize, gs | qpdf child | S | P0 |
| Linearize (fast web view) | qpdf --linearize, gs -dFastWebView | qpdf child | S | P1 |
| Encrypt, set permissions | all, pdfcpu, qpdf | lopdf encrypt or qpdf | M | P1 |
| Decrypt, remove password | all, pdfcpu, qpdf | lopdf authenticate_password, qpdf fallback | M | P1 |
| Watermark, stamp, overlay | all, pdfcpu, Stirling | pdf-writer for Latin, pdfcpu for the rest | M | P1 |
| Page numbers, headers, footers, Bates | Sejda, Stirling, PDF24, pdfcpu | pdf-writer with embedded font, or pdfcpu | M | P1 |
| Metadata read, write, strip | all, pdfcpu properties | lopdf | S | P0 |
| Attachments, portfolio | pdfcpu, qpdf, Stirling | pdfcpu child | M | P2 |
| Form fill and flatten | pdfcpu form, Stirling, Sejda, Acrobat | pdfcpu fill, flatten needs content work | L | P2 |
| Redaction (true removal) | Acrobat, Stirling, iLovePDF, PDF24, PDF2Go | none safe locally yet, see section 5 | XL | P2 |
| PDF/A conversion | gs -dPDFA, OCRmyPDF, Stirling, iLovePDF | ghostscript child | M | P1 |
| PDF/A validation | verapdf, PDF2Go | veraPDF child, licence unverified | M | P2 |
| OCR text layer | OCRmyPDF, gs pdfocr devices, Acrobat, Stirling | OCRmyPDF child, gs pdfocr as fallback | L | P1 |
| Deskew and clean scans | OCRmyPDF --deskew, unpaper, Sejda, Stirling | OCRmyPDF child | L | P1 |
| PDF to images, DPI, range, format | all | ghostscript, exists, extend range and format | S | P0 |
| Images to PDF with page size, fit, margin, DPI, orientation, grid | all, pdf-writer | pdf-writer, exists as v1 | M | P0 |
| PDF to plain text | all, pdftotext, gs txtwrite | poppler child for layout, lopdf for short files | S | P0 |
| PDF to Markdown | iLovePDF, PDF2Go, Stirling | no cheap option, ML layouts only, see section 9 | L | P2 |
| PDF to DOCX, XLSX, PPTX | iLovePDF, Acrobat, Sejda, Stirling | no reliable local engine, LibreOffice cannot | XL | never for now |
| Bookmarks and outline editing | pdfcpu bookmarks, Sejda, Stirling | pdfcpu child, lopdf helpers | M | P2 |
| Compare two PDFs | Stirling, iLovePDF, PDF24, PDF2Go | pdftotext diff, gs raster for visual diff | M | P2 |
| Repair a damaged PDF | gs rewrite, qpdf --check, pdfcpu validate | ghostscript rewrite then qpdf check | S | P1 |
| Digital signature verify | pdfsig, pdfcpu, Stirling, Acrobat | pdfsig child (poppler) | M | P2 |
| Digital signature sign | Acrobat, Stirling, pdfsig, pdfcpu removal only | no Rust path, pyHanko (MIT, Python) or OpenSSL child | XL | P2 |
| Strip JavaScript, sanitize | Stirling, Acrobat | lopdf remove OpenAction and JavaScript names | S | P1 |
| Font embedding and subsetting | gs rewrite, pdffonts, qpdf | ghostscript, report with pdffonts | S | P2 |
| PDF/X print output | Acrobat, gs | ghostscript, unverified | L | P3 |
| Accessibility tagging, PDF/UA | Acrobat, veraPDF | no local engine | XL | never for now |
| Batch and saved recipes | all, Stirling Automate, Sejda workflows | selection already batches, recipes later | M | P2 |

## 3. Platform snapshots

- Stirling PDF: "50+ PDF tools", desktop app, browser UI and server, MIT licence with proprietary subdirectories, and its own MCP server (documentation navigation). Tools include a PDF text editor, page editor, redaction, certificate signing, RFC 3161 timestamp, sanitize, flatten, PDF/A, OCR, compare, booklet, multi-page layout, attachments, portfolios, auto rename, repair and JavaScript inspection.
- iLovePDF: merge, split, compress, organize, PDF to Word, PowerPoint, Excel, PDF/A, JPG, Word and HTML to PDF, OCR, repair, rotate, page numbers, watermark, crop, forms, unlock, protect, sign, redact, compare, and newer AI entries: summarizer, translate, PDF to Markdown. A Windows and macOS desktop app exists.
- Adobe Acrobat online: "25+ tools" with AI features (chat with PDF, summaries, flashcards, quizzes, mind maps, presentation generation) plus conversion in and out of PDF for Office, image and design formats.
- Sejda: split by pages, bookmarks, half, size and text, Bates numbering, headers and footers, N-up, deskew, OCR, workflows, edit metadata, extract images, flatten, grayscale, flip, repair.
- PDF24: offline desktop creator plus online tools, merge, split, compress, OCR, overlay, compare, web optimize, redact, page numbers, and it advertises that files stay on the PC in the desktop version.
- PDF2Go: booklet, poster printing, page layout, PDF/A validation, redaction, PDF cleaner, compare, chat with PDF, summarize, translate, spellcheck.
- Smallpdf: page fetched on 2026-11-04 rendered client side and returned no tool text, so its feature list is unverified here.
- pdfcpu: a single Go binary with validate, optimize, split, merge, trim, crop, resize, rotate, zoom, pages, collect, nup, booklet, poster, cut, grid, ndown, stamp, watermark, page numbers, bookmarks, boxes, annotations, form, attachments, portfolio, encrypt, decrypt, permissions, properties, keywords, signatures (validate and remove), import, create and extract.
- qpdf: structural transformation, --linearize, --pages, --split-pages, --encrypt, --decrypt, --optimize-images with --jpeg-quality, --compress-streams, --remove-restrictions, JSON output, attachments. No raster, OCR or PDF/A option appears in its manual option list.
- Ghostscript: pdfwrite with presets screen, ebook, printer and prepress, image downsampling controls, -dPDFA, -dFastWebView, plus png, jpeg, pdfimage8/24/32 and pdfocr8/24/32 devices, the txtwrite text device, and OCR devices that drive Tesseract with -sOCRLanguage, for example "eng+ara".
- OCRmyPDF: adds a searchable text layer to scans, with PDF/A output modes, --optimize levels, --deskew, unpaper preprocessing and optional JBIG2; it drives Tesseract and Ghostscript or qpdf underneath.
- poppler: the reference command line set, pdftotext with -layout, -raw, -f, -l and crop options, pdftoppm, pdfimages, pdfinfo, pdftohtml and pdfsig, which verifies and can add ETSI CAdES signatures.
- pdfium: the Chrome PDF engine, BSD-3-Clause, exposes a text API (public/fpdf_text.h) and full rendering; pdfium-render wraps it for Rust. It is a C++ shared library, so shipping it is a packaging cost, not a code cost.
- LibreOffice: already the documents engine; its PDF import opens a file in Draw for limited editing, and it is not a PDF to DOCX converter.
- Typst (Apache-2.0) and pandoc (GPL-2.0-or-later) are good HTML and Markdown to PDF engines but do not read PDF.

## 4. Engines and licences

| Engine | Licence (verified) | Can Kawaro link it | How to use it | Notes |
| --- | --- | --- | --- | --- |
| Ghostscript | AGPL-3.0 | no | child process, already used | keep the existing rule: separate process, never linked |
| qpdf | Apache-2.0 | yes, but C++ | child process first | static, small, no daemon, best structural tool |
| pdfcpu | Apache-2.0 (Go) | no, Go binary | child process | single binary, covers the widest command surface |
| OCRmyPDF | MPL-2.0 | no, Python | child process | pulls Tesseract, Ghostscript, qpdf at run time |
| Tesseract | Apache-2.0 | yes, but C++ | child process, already planned | Arabic traineddata needed per language |
| poppler | GPL-2.0-or-later | yes, compatible | child process utilities first | pdftotext, pdftoppm, pdfimages, pdfsig |
| pdfium | BSD-3-Clause | yes | link through bindings, C++ shared lib | fast rendering and text, packaging cost |
| MuPDF | AGPL-3.0 | no | not planned | cannot be linked into a GPLv3 binary, and gs already covers the raster role |
| lopdf | MIT | yes | pure Rust, already used | read, edit, merge, split, encrypt, extract_text |
| pdf-writer | MIT OR Apache-2.0 | yes | pure Rust, already used | writing new pages, stamps, N-up, images to PDF |
| pdf-extract | MIT | yes | pure Rust | text extraction helper over lopdf |
| Typst | Apache-2.0 | yes | pure Rust | HTML and Markdown to PDF, later |
| pandoc | GPL-2.0-or-later | separate process only | child process | document conversion, not a PDF reader |
| LibreOffice | MPL-2.0 | no | child process, already used | office to PDF |
| docling | MIT | no, Python plus models | opt-in component only | layout aware PDF to Markdown, heavy |
| marker | Apache-2.0 | no, Python plus models | opt-in component only | same role as docling |
| pyHanko | MIT | no, Python | child process, later | PDF signing, needs a certificate story |

Mixing rules that follow: nothing AGPL is linked; Ghostscript, MuPDF and PyMuPDF stay out of the binary. qpdf and pdfcpu are Apache-2.0, so they could be linked or bundled, but as system programs they keep the base install light and match the existing engine model.

## 5. Honest gap list against lopdf, pdf-writer and Ghostscript

Cheap on lopdf, no new process:

- Page extract, delete, reorder, insert and duplicate, plus split by ranges, every N pages and by size. lopdf already has delete_pages, page_iter, save_modern, add_bookmark and replace_text.
- Metadata read, write and strip; XMP is a separate stream and needs its own handling.
- Decrypt with a password: lopdf has authenticate_password and was_encrypted, so this is a param and a UI path, not a new engine. Encrypt and permissions are also in lopdf but need careful testing against readers.
- JavaScript and OpenAction removal for the sanitize action.
- Crop and page box changes (MediaBox, CropBox, rotation) without touching content.
- Outline and bookmark editing, roughly.

Cheap on pdf-writer, pure Rust:

- Page numbers, headers, footers and Bates stamps, stamps and overlays, N-up and booklet layouts. All of these are "place existing pages as form XObjects on a new page", which is exactly what the current images to PDF path already does.
- Images to PDF options: page size, fit, margins, DPI, orientation, one per page or a grid, never enlarge.
- Cost: Latin text needs one embedded font; Arabic text needs shaping (rustybuzz, MIT) and a font subsetter, so Arabic stamps and watermarks are a separate M item.
- Linearization cannot be written by pdf-writer; that is qpdf's job.

Needs a new child process:

- qpdf for linearize, lossless object cleanup, robust encrypt and decrypt, attachments and --check repair of structure. Apache-2.0, small, the safest first new engine.
- pdfcpu for N-up, booklet, poster, cut, grid, ndown, stamps, page numbers, paper resize, bookmarks, properties, forms, portfolios, image and font extraction. Apache-2.0, but it also overlaps with what lopdf can already do, so it is the second choice after qpdf unless page layout work lands first.
- OCRmyPDF for the OCR text layer with deskew and PDF/A output. MPL-2.0, Python, and it needs Tesseract plus Ghostscript at run time. This is the highest value scan feature and the heaviest dependency.
- poppler utilities for layout aware text extraction and for pdfsig verification.

Not acceptable, or not now:

- Linking MuPDF or PyMuPDF (AGPL) into the binary.
- True redaction with the current stack. Neither lopdf, pdf-writer nor Ghostscript removes text from content streams and from all copies of an image. pdfcpu has no redact command in its command list (checked 2026-11-04). A real implementation is a content stream rewrite plus a verification step, and a bad one is worse than none. Keep it out until there is a dedicated plan and fixtures.
- PDF to DOCX, XLSX or PPTX, and PDF/UA tagging: no reliable local engine, and a poor result damages trust more than a missing button.

## 6. Arabic, RTL and the OCR text layer

- Extraction order. PDF content streams place glyphs in the order the producer chose. Arabic producers often write glyphs in visual order, so a naive extractor returns reversed or disconnected text. Correct output needs the visual to logical mapping plus the Unicode Bidirectional Algorithm (UAX #9, Unicode 18.0.0). Treat every Arabic fixture as a producer specific test case, not as one generic case.
- lopdf extract_text joins decoded strings and is weakest with subset fonts that carry no ToUnicode map; that is exactly the common case in Arabic PDFs from InDesign and LibreOffice. pdftotext with -layout is better but still returns text in content order; poppler documents -raw as content stream order.
- Searchability matters more than display. A PDF text layer is placed word by word with invisible text (render mode 3). If the layer is written in visual order, Arabic search and copy fail even though the page looks right.
- OCR quality is the second problem. This project already measured Tesseract Arabic character error 0.18 on book pages and 0.35 on noisy scans, and rejected the LFM3 vision model (ADR 0010 part B). So plan OCR around Tesseract plus preprocessing, and set expectations in the UI, not around a new model.
- Two possible OCR paths. OCRmyPDF already solves layer placement, PDF/A and deskew. Ghostscript's own pdfocr8/24/32 devices also add a text layer and accept -sOCRLanguage="eng+ara", but only in builds compiled with OCR support, which distribution builds may omit: test gs -h for the ocr device before promising it. Either way, the text layer must be checked with a copy and paste test and a search test in a reader.
- Generated Arabic text (watermarks, stamps) needs shaping and bidi at build time, which is real work: rustybuzz plus a subsetter, plus fixtures in both directions.
- Minimum Arabic fixture set for any PDF story: a text PDF from LibreOffice, a scanned book page, a two column page, a page with a table, a form with Arabic labels, a right to left page with Latin inserts, and one encrypted file.

## 7. Speed and memory reality

Estimates, not measurements:

| Trap | Why it matters | Mitigation |
| --- | --- | --- |
| Full document in memory | lopdf, qpdf and pdfcpu all hold the whole file; a 500 page scanned PDF can be several hundred MB | prefer Ghostscript or pdfium for raster paths, warn above a size threshold, keep the work in the daemon |
| Rewrite invalidates signatures and IDs | every save rewrites objects and breaks signatures | detect signatures first, say so before running, never silently re-sign |
| Incremental save | lopdf loads and writes the full file, so incremental append is not the default path | acceptable for now, document it |
| Encrypted input | lopdf refuses when it cannot open; there is no password UI today | password as an explicit option, never a prompt that blocks the hive |
| Malformed files | strict parsers fail where Ghostscript often succeeds | repair action first, then report which engine failed |
| Ghostscript re-distils | pdfwrite re-encodes fonts and can change appearance in edge cases | fixture set per preset, keep the "smaller or keep original" rule |
| Zipping many pages | one output file per task means a ZIP for split and to-images | keep stored entries, existing rule is correct |
| Arabic shaping in generated text | shaping and bidi are CPU work per page | precompute, cache the font subset once per job |

## 8. Prioritised plan

P0, table stakes, next:

1. PDF page operations on lopdf: extract, delete, reorder, insert, duplicate; split by ranges, every N pages and by size. Engine: pdf, pure Rust. E: M. Fixtures: page counts 1, 2, 500, mixed page sizes, Arabic content.
2. Images to PDF options: page size, fit, margin, DPI, orientation, grid. Engine: pdf-writer. E: M. Fixtures: 1 and 40 images, portrait and landscape mixes, one huge PNG.
3. Compress with honest controls: preset plus a DPI target exposed from Ghostscript parameters. Engine: ghostscript. E: S. Fixtures: text PDF, scan, mixed.
4. Lossless cleanup and linearize: qpdf as a new child engine adapter, with --check as a bonus. E: M plus packaging and licence line in docs/engines.md.
5. Metadata read, write, strip on lopdf. E: S.
6. PDF to text with layout via poppler pdftotext, as a child adapter. E: S.
7. Split by bookmarks: lopdf outline walk. E: S, after P0 item 1.
8. The option schema in core: typed per-op options, defaults, ranges, labels in Arabic and English, so the UI, CLI and MCP share one definition. E: M, needs an ADR before code.

P1, differentiators:

1. Password decrypt and encrypt with permissions, lopdf first, qpdf for the cases lopdf rejects. E: M.
2. Watermark, stamp, page numbers and Bates on pdf-writer with one embedded font. E: M, Arabic variant M more.
3. Crop, scale to page size and margins. E: M.
4. N-up and booklet, pdfcpu child. E: M.
5. OCR text layer, OCRmyPDF child after a check that the system has it or offers it as an optional component. E: L, includes the Arabic fixture set.
6. Sanitize: JavaScript and OpenAction removal, plus attachment handling. E: S.
7. Repair: Ghostscript rewrite, then qpdf check, then a clear report. E: S.
8. Overlay two PDFs and print style N-up later in P1.

P2, later: forms fill and flatten, attachments and portfolios, bookmarks and outline editing, compare, signature verification with pdfsig, PDF/A conversion and validation, PDF to Markdown as an opt-in component. Never for now: signing, redaction, PDF/UA, office reverse conversion.

## 9. What stays deterministic, and what would need local AI

Deterministic primitives to expose over MCP and the CLI, with the agent supplying the intelligence: page range strings, page size and orientation targets, DPI, output directory and naming pattern, grouping strategy (one output per file or one combined), compression preset, OCR language list, redaction rectangles by page and coordinates. The agent decides what to ask for; Kawaro never guesses.

Genuinely AI shaped, and only as opt-in components under ADR 0010: layout aware PDF to Markdown and to DOCX, OCR improvement on hard scans and handwriting, and redaction candidate detection. Note that iLovePDF and PDF2Go already market "chat with PDF" and summaries, which are agent features, not file features, and Kawaro should serve them through MCP rather than embed a model.

## 10. Open questions and unverified items

- Unverified: Smallpdf feature list (client rendered page), exact distro Ghostscript builds with OCR devices, veraPDF licence and a maintained release, pdfcpu redaction absence beyond the command list, pdfium-render licence, ISO numbers for PDF/A parts 2 to 4 (part 1 is confirmed as ISO 19005-1:2005).
- Needs a spike: qpdf versus pdfcpu for merge and bookmark output quality (outlines, forms, tags), and Ghostscript versus pdfium for to-images quality and speed on 300 DPI scans.
- Needs an owner decision: whether a second child engine (qpdf) may be added to the base install, and whether pdfcpu is an optional component.
- Needs an ADR: the option schema, the multi-output contract if ZIP is ever dropped, and any new engine id.

## Sources

All accessed 2026-11-04.

- Stirling PDF documentation, all tools: https://docs.stirlingpdf.com/Tools/
- Stirling PDF repository and licence: https://github.com/Stirling-Tools/Stirling-PDF and https://raw.githubusercontent.com/Stirling-Tools/Stirling-PDF/main/LICENSE
- iLovePDF tools: https://www.ilovepdf.com/
- Adobe Acrobat online tools: https://www.adobe.com/acrobat/online.html
- Sejda PDF tools: https://www.sejda.com/pdf-tools
- PDF24 tools: https://tools.pdf24.org/en/
- PDF2Go tools: https://www.pdf2go.com/
- pdfcpu README and command list: https://github.com/pdfcpu/pdfcpu and https://raw.githubusercontent.com/pdfcpu/pdfcpu/master/README.md
- qpdf manual, options and limits: https://qpdf.readthedocs.io/en/stable/cli.html
- qpdf licence: https://raw.githubusercontent.com/qpdf/qpdf/main/LICENSE.txt
- Ghostscript high level devices (pdfwrite, presets, -dPDFA, -dFastWebView): https://ghostscript.readthedocs.io/en/latest/VectorDevices.html
- Ghostscript output devices (txtwrite, png, jpeg, pdfimage, pdfocr, Tesseract): https://ghostscript.readthedocs.io/en/latest/Devices.html
- Ghostscript licence: https://raw.githubusercontent.com/ArtifexSoftware/ghostpdl/master/LICENSE
- OCRmyPDF documentation: https://ocrmypdf.readthedocs.io/en/latest/ and https://ocrmypdf.readthedocs.io/en/latest/advanced.html
- OCRmyPDF licence (MPL-2.0): https://raw.githubusercontent.com/ocrmypdf/OCRmyPDF/main/LICENSE
- Tesseract licence: https://raw.githubusercontent.com/tesseract-ocr/tesseract/main/LICENSE
- poppler project and utilities: https://poppler.freedesktop.org/ and https://man.archlinux.org/man/pdftotext.1.en
- poppler licence (GPL-2.0): https://gitlab.freedesktop.org/poppler/poppler/-/raw/master/COPYING
- pdfium licence (BSD-3-Clause): https://raw.githubusercontent.com/chromium/pdfium/main/LICENSE
- pdfium text API: https://pdfium.googlesource.com/pdfium/+/refs/heads/main/public/fpdf_text.h
- MuPDF licence (AGPL-3.0): https://raw.githubusercontent.com/ArtifexSoftware/mupdf/master/COPYING
- lopdf crate: https://crates.io/crates/lopdf and https://docs.rs/lopdf/latest/lopdf/struct.Document.html
- pdf-writer crate: https://crates.io/crates/pdf-writer
- pdf-extract crate: https://crates.io/crates/pdf-extract
- printpdf crate: https://crates.io/crates/printpdf
- Typst licence (Apache-2.0): https://raw.githubusercontent.com/typst/typst/main/LICENSE
- pandoc licence (GPL-2.0-or-later): https://raw.githubusercontent.com/jgm/pandoc/main/COPYRIGHT
- docling licence (MIT): https://raw.githubusercontent.com/docling-project/docling/main/LICENSE
- marker licence (Apache-2.0): https://raw.githubusercontent.com/datalab-to/marker/master/LICENSE
- pyHanko licence (MIT): https://raw.githubusercontent.com/MatthiasValvekens/pyHanko/master/LICENSE
- Unicode Bidirectional Algorithm, UAX #9, Unicode 18.0.0, 2026-09-01: https://www.unicode.org/reports/tr9/
- ISO 19005-1:2005 (PDF/A-1): https://www.iso.org/standard/38920.html
- veraPDF: https://verapdf.org/
- In repo: docs/engines.md, crates/core/src/action.rs, actions/pdf.toml, crates/engines/src/pdf.rs, docs/adr/0010-opt-in-model-downloads.md, docs/research/ai-edition-tiers.md
