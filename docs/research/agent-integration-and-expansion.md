# Agent integration and product expansion: study and recommendation

Status: decision input for the owner, not a decision. Date 2026-11-04.
This note synthesises four parallel research files and a repo audit. Estimates are guesses and are labelled as such.

Companion files:
- `docs/research/mcp-agent-integration.md`: MCP protocol, hosts, security, design.
- `docs/research/pdf-feature-benchmark.md`: PDF features, engines, licences, order.
- `docs/research/image-editing-and-preview.md`: image capabilities, preview pipeline, stage 2 UI.
- `docs/research/converter-product-scan.md`: 25 competitor products, pricing, UX, completeness.

## 1. The question

The owner asked three things in one: (a) support MCP so any local AI app can use Kawaro as its file hands, with high quality, speed, a result in the chat and the file next to the originals, switchable by the user; (b) expand the product with more file types, image editing and per-case customization, including live preview and the resulting dimensions and ratio, shown only when the user wants it; (c) harvest ideas from many online platforms so the project becomes complete, especially PDF, without losing the modern, fast hive.

## 2. Verdict

| Question | Answer | Why |
| --- | --- | --- |
| Is MCP a good idea for Kawaro? | Yes, and it is the strongest differentiator available in 2026 | The competitors that offer MCP upload the file to their servers. Kawaro can offer the same capability with no upload, no account and no network listener |
| Is it possible? | Yes, with a small server that is a thin client of `kawarod`, like the CLI | `Registry::matching`, `kawaro run`, the queue, progress, cancel and the never overwrite output rule already exist |
| Is it good as the first priority? | No. Per-run options come first, because they block both MCP arguments and the customization UI | Actions still carry fixed `[action.params]`; `Request::Run` cannot carry a value |
| Is the customization UI possible? | Yes, inside the same hive window, not a new editor | The preview budgets are reachable with scaled decode and pre-masking, but only off the UI thread |
| Is the product expansion possible? | Yes. PDF is the widest and cheapest gap | The existing lopdf, pdf-writer and Ghostscript stack covers most of P0 |
| Does it stay true to the brand? | Yes if Kawaro never embeds a model and never calls the network; the agent brings the intelligence, Kawaro brings deterministic file operations | No model download, no telemetry, no server, no account |
| What is the real risk? | Protocol churn, host variance, security review, MSRV, and scope creep | MCP changed twice in nine months and the last change removed the handshake |

Recommended shape: **do both, in a fixed order, and do not promise interactive chat panels in the first release**.

1. Options as data in core (one story, no UI).
2. Preview worker and shared geometry, measured against a budget.
3. Stage 2 customization for resize only.
4. `kawaro mcp` stdio server, read-only first, then write tools.
5. PDF P0 breadth.
6. OCR, MCP Apps, HEIC and the rest, only after the base is trusted.

## 3. Why the base makes this cheap

Facts from the repo at 0.5.0:

| Asset | What it means for this plan |
| --- | --- |
| 45 actions in `actions/*.toml`, 5 crates, one `Engine` trait | The MCP tool list can be built from the same registry; no second catalog |
| `Registry::matching(files, engine_ok)` | "Which actions fit these files" is already answered; `kawaro_plan` is a thin wrapper |
| Queue with progress and cancel, daemon over a user-only socket or named pipe | The MCP process does not process files; it submits to the daemon and polls, so it is small and safe |
| Output naming with hard link placement, never overwrite, atomic rename | The agent can never destroy a file; a retry produces a numbered second file |
| Heavy engines as child processes, probe hides missing engines | The server can report what this machine can do, and refuse honestly |
| Hive 393 by 362 px, 41 to 118 ms through the daemon, 28 MB idle, software renderer | The stage 2 plate and the preview must respect a CPU pixel budget, not a GPU one |
| `crates/ui/src/preview.rs` already decodes a PNG row by row into the centre hexagon | The preview worker starts from working code, not from zero |
| Design tokens, no shadows, no gradients, RTL by hand | Any new stage must reuse tokens; no new visual language |

What is missing, in order of importance: typed per-run options; a preview worker and stage 2; an `[mcp]` config section with permissions; PDF and image breadth; HEIC input; OCR.

## 4. MCP: what to build and what to refuse

### 4.1 Protocol state on 2026-11-04

The current revision is **2026-07-28**. It is a hard break from 2025-11-25: stateless requests with per-request `_meta`, no `initialize` handshake, `server/discover`, required `resultType`, multi round-trip requests instead of server-initiated requests, no `ping` and no `logging/setLevel`. Tasks moved out of the core into the Beta `io.modelcontextprotocol/tasks` extension. MCP Apps is Stable, Skills over MCP is Beta, and the official registry is live.

Two consequences:

1. A hand-written stdio server is now small. The escape hatch exists if the SDK cannot be used.
2. `rmcp` 3.8.0, the Tier 1 official Rust SDK (Apache-2.0), needs Rust 1.88 while the workspace is on 1.85. This is a real decision, not a detail: raise the MSRV, or hand-write the message loop over serde_json. Decide it in the ADR.

### 4.2 Transport and hosts

stdio only for the MVP. The host starts `kawaro mcp` as a child process. No port, no authentication, no network code, the process lives and dies with the chat app, and the privacy claim needs no explanation.

Verified to start local stdio servers: Claude Desktop, Claude Code, Cursor, LM Studio. Reported but not verified here: VS Code and GitHub Copilot, Zed, Windsurf, JetBrains AI, LibreChat, Raycast, n8n. ChatGPT connectors and the Apps SDK are remote, so a local file server is out of scope there, which is also the right privacy answer.

Loopback HTTP is not worth adding until a host that matters refuses stdio. If it is ever added, it must be loopback only, off by default, with a same-user check and a shared secret, and it opens a port that the privacy claim must explain.

### 4.3 What "preview in the chat and the file next to the originals" really means

A tool result can carry all of these together:

| Block | Content | Reliability |
| --- | --- | --- |
| `image` | A small PNG preview of the result | Some hosts draw it; no guarantee |
| `text` | The absolute path, bytes, page count or dimensions, and a readable sentence | Always drawn |
| `resource_link` | A `file://` URI with a name and MIME type | Rendered as a link at best |
| `structuredContent` | Machine fields: outputs, bytes, pages, dimensions, elapsed, overwritten=false | Model facing in most hosts |

So the reliable deliverable is the file next to the originals, which Kawaro already guarantees. The chat preview is decoration and must never be promised as a panel. Keep the preview small (**guess**: 512 px wide, under 200 KB base64, cached), and only rasterise a PDF page when the tool asks for it.

MCP Apps is the standard for a richer card (thumbnails, an open folder button, even an approve button). It is Stable and the ext-apps repository lists Claude, ChatGPT and VS Code as supported clients (the community support matrix is stale), but it is still an extension with churn and a web bundle to maintain. Put it after the plain tools are trusted, not in the MVP.

### 4.4 Tool set

Keep 6 to 10 tools. Models choose better from a short list.

| Tool | Kind | Job |
| --- | --- | --- |
| `kawaro_formats` | read | Formats and engines on this machine, plus the action catalog version |
| `kawaro_plan` | read, dry run | Actions that fit these paths, output name, estimate, and the preview of what would change |
| `kawaro_convert` | write | Run one action id on paths, with the resolved options and the result |
| `kawaro_images_to_pdf` | write | The headline case, named so the model does not have to guess an action id |
| `kawaro_pdf` | write | merge, split, rotate, compress, to_images, then page ops and text later |
| `kawaro_image` | write | convert, resize, rotate, flip, grayscale, compress, then crop and target size |
| `kawaro_job_status` | read | Poll a job started in job mode |
| `kawaro_job_cancel` | write | Cancel a job |
| `kawaro_inspect` | read | Dimensions, pages, metadata, no writes (later) |

Annotations: `readOnlyHint` for reads, `destructiveHint: false` everywhere, `openWorldHint: false`, and `idempotentHint: false` for writes, because a retry creates a numbered second file. Say that in the description. A `plan` tool before a write tool is the cheap trust move.

### 4.5 Long jobs

Default is a bounded wait with progress notifications for short jobs. Above a few seconds, return `mode: "job"` with the Kawaro job id and let the model poll `kawaro_job_status`. Do not build the Tasks extension: it is Beta and no host in the matrix declares it. Do not hold a host connection open for a 300 page scan.

### 4.6 Security and consent

The official Local Server Security page says it plainly: stdio is not a sandbox, the host and server share one trust domain, and a local server can read anything the user can read. Kawaro's advantage is a small capability: read an input, write a new file, nothing else.

Proposed controls, all cheap because the app is already local and non-destructive:

1. Off by default. `[mcp] enabled = false`; a host config alone cannot turn it on.
2. Read-only first, write tools behind a second switch.
3. `allowed_roots` list, default to Documents, Pictures, Downloads; canonicalise every path, resolve symlinks, refuse escapes, refuse system directories and dot files. An empty list means no writes.
4. No delete, no overwrite, no in-place operation, ever.
5. Limits on file count, bytes, wall clock and preview size, mapped to the existing `CancelToken`.
6. One audit JSON line per call in the log directory: tool, paths, result, bytes, client name, duration.
7. Server logs to stderr only. `stdout` is reserved for JSON-RPC.
8. Tool descriptions come from the shipped `actions/*.toml`, never from user files. File names are returned escaped as data, never interpreted.
9. The MCP process talks to `kawarod` over the existing user-only socket or named pipe. It never exposes that socket to the host.
10. Install helper shows the exact command before writing (`--dry-run`, then `--write`), following SEP-1024, and copies a snippet for hosts that do not take bundles.

Settings: one switch on the existing Integration page plus a tool list with per-tool toggles and the roots list. No new window, no focus stealing.

### 4.7 Effort and conditions

**Guess**: 8 to 15 working days for a read-only plus basic write MVP, 20 to 30 with the approval UX, audit and install helper. Hand-driven testing on each host is the real schedule risk and is not in those numbers. Protocol churn deserves one maintenance story per revision.

GO conditions: one host is the acceptance target (recommend Claude Desktop or Claude Code), read-only ships first, the plan tool exists before the write tool, the ADR settles roots, limits, audit and the MSRV, and nobody promises a live chat panel.

## 5. Options as data: the linchpin

The single largest gap is not UI. `ActionDef.params` is an untyped `toml::Table`, values are baked into `actions/*.toml`, and `Request::Run` carries only an action id. No user, script or agent can change one value on one run. This blocks the customization UI, the CLI and MCP at the same time.

Proposal, as one story (S-117) behind an ADR:

- Keep `[action.params]` as the defaults a plain click runs. Add an optional `[action.options]` block that describes what may change.
- A closed type set: `percent`, `px`, `px_pair`, `enum`, `bool`, `int`, `float`, `string` (the last only where an engine needs text, for example a page range). An unknown type is a startup error, never a free text field.
- A field carries `id`, `type`, `label {ar, en}`, `default`, `min`, `max`, `step`, `unit`, `visible_when {field, equals}`, `maps_to` and `no_enlarge`.
- `kind = "form"` opens the stage 2; `readout = ["dimensions", "ratio", "percent", "output_name"]` declares what the preview must show.
- Presets are data: `[[action.options.preset]]` with an id and values, rendered as chips (Web, Share, Print). Remembering the last values is a config map keyed by action id, not a TOML write.
- Merge order is defaults, remembered values, explicit request. Validation lives in `core`: range, step, enum membership, cross-field rules. Engines keep their own validation.
- The wire change is additive: `Request::Run { action, files, params: Option<toml::Table> }` with `#[serde(default)]`. The daemon merges and rejects unknown keys as `BadParam`.
- One schema export feeds three consumers: JSON Schema for MCP, `kawaro actions --id X --schema` for scripts, and `kawaro run X --set percent=25` for the CLI. The UI parses the same schema in core. Only `core` reads TOML.
- Naming uses `suffix_template`, rendered from resolved values and passed through the existing conflict free naming in `crates/core/src/output.rs`.

This is a contract change to actions, so it needs an ADR (proposed 0012). It unblocks the MCP options, the CLI and PDF options such as page size and DPI.

## 6. Customization stage and live preview

### 6.1 Doors that preserve the one click promise

Stage 1 does not change: the hive appears in under 150 ms, the cells are the actions, one click runs the defaults. Customization is a deliberate second gesture, not a new step for everyone.

- Hovering or focusing a customizable cell shows the result readout in the centre cell, for example "1920 x 1280, 3:2", so the user can decide without opening anything.
- A concentric hexagon outline around that cell (2 px `text-primary`, no fill) marks it customizable. Clicking the cell body runs the default; clicking the ring or Shift+Enter opens stage 2.
- A settings switch "always ask before running actions with options" flips the default for people who want that. It is off by default.

### 6.2 Stage 2 layout

The same window grows from 393 by 362 to about 660 by 420 around its centre, using the existing hive open motion (scale 0.92 to 1, 120 ms, zero under reduced motion). No new window, so focus cannot be stolen.

- The hive plate stays `surface-1`; the stage 2 plate is `surface-3` inset by `space-4` with a full 1 px `border`. That is the entire elevation story.
- Left (right in RTL): a pointy top hexagon well of circumradius 180 with the preview masked to the hexagon, the target rectangle inside, crop handles when cropping, and one footer line with dimensions, ratio, percent change and output file name.
- Right (left in RTL): the option column, one row per field, using the vendored Material 3 slider, segmented buttons, checkbox and text field. Preset chips above the fields.
- Footer: Run (filled `accent`, `accent-ink`), Reset, and a "run with defaults" text button. Escape returns to stage 1 with nothing changed.
- Batch: preview the first file, and the readout adds "and 4 more, same settings". Per-file options are out of scope; they turn the hive into a form editor.

### 6.3 Preview pipeline and budgets

All off the UI thread, one worker thread, no async runtime in the tree:

1. Probe the header only, under 1 ms, publish dimensions before any pixel exists.
2. Decode a draft at reduced resolution: JPEG `Decoder::scale` gives 1/8, 1/4, 1/2, 1; PNG row streaming as `preview.rs` does; SVG renders at the target size; WebP, TIFF and GIF decode then resize, first frame only.
3. Cache the draft and never decode twice for one option change.
4. Debounce 80 ms, resample the draft to the well, mask the hexagon in the worker, hand Slint a ready buffer, cancel older work by generation counter (single flight, newest wins).
5. On drag release, one high quality pass with the engine kernel so the preview matches the file.

Target budgets (estimates to be proven by the bench):

| Stage | Target |
| --- | --- |
| Hive first frame | under 150 ms, unchanged |
| Stage 2 shell after the customize click | 120 ms |
| First preview pixel, 24 MP JPEG | under 100 ms, 150 ms worst |
| Re-render on an option change | 30 to 50 ms |
| Slider drag | 30 fps draft, full quality on release |
| Idle frame paint | under 8 ms |
| Preview memory | 4 MB total, draft capped at 1000 px long side |
| 100 MP JPEG | dimensions instant, draft 300 to 600 ms, never 1/1 |

Hard rule: preview and engine call one function. Move the geometry into core, for example `image_geom::target(source, mode, value) -> (w, h, Option<Crop>)`, and let `crates/engines/src/image.rs` call it. A preview one pixel off the file is a bug report waiting to happen.

Failure is allowed: if a decoder fails or the pixel cap is hit, show the dimensions and "preview not available". A failed preview must never fail the job.

### 6.4 Renderer and design constraints

The Slint 1.18.1 software renderer forbids element rotation and scaling and clipping to a path (per the official renderer table). So the worker pre-rotates, pre-scales and pre-masks the hexagon (alpha 0 outside, 1 px antialiased edge). Never transform the preview with a Slint element.

The dirty region cache helps: only the picture rectangle repaints when the picture property changes. A 228 px well is 52k px and the whole stage 2 plate is 277k px, so fill rate is not the problem; text relayout and layout are. Update the readout at most 20 times a second, do not animate plate colours, and accept a 30 fps draft.

Design rules that stay: tokens only, no shadows, no gradients, no one-sided borders, elevation by `surface-0` to `surface-3`, glass only over a real compositor blur. Arabic and English are two originals; mirror the layout by hand; keep digits Latin inside a bidi isolate; every control's accessible label includes its value and unit; focus is a 2 px `text-primary` outline, never colour alone.

## 7. Product breadth

### 7.1 PDF: the widest gap, mostly cheap

Kawaro ships 5 PDF operations. iLovePDF lists 25 or more, Stirling PDF 60 or more, PDF2Go 50 apps. Five reads as a beta.

P0, next (all cheap on the current stack, mostly lopdf and pdf-writer):

| Feature | Engine | Effort guess |
| --- | --- | --- |
| Page organiser: extract, delete, reorder, insert, duplicate | lopdf | M |
| Split by ranges, every N pages, by size, replacing the per-page full copy | lopdf | S to M |
| Images to PDF options: page size, fit, margin, DPI, orientation, grid | pdf-writer | M |
| Compress with DPI and downsample targets | Ghostscript params | S |
| Metadata read, write, strip | lopdf | S |
| PDF to plain text with layout | poppler pdftotext, child | S |
| Split by bookmarks | lopdf outline walk | S |
| Lossless cleanup, linearize, robust encrypt check, repair | qpdf, new child engine | M plus packaging |

P1: encrypt and decrypt with permissions (lopdf first, qpdf fallback); watermark, page numbers, headers and Bates (pdf-writer plus one embedded font, Arabic variant M more); crop and scale to page size; N-up and booklet (pdfcpu child); OCR text layer with deskew (OCRmyPDF plus Tesseract, L); sanitize JavaScript; repair.

Never for now, with reasons: true redaction (no safe local stack; a bad redaction is worse than none), PDF to DOCX, XLSX or PPTX (no reliable local engine; LibreOffice cannot reverse convert), digital signing (no Rust path, certificate story missing), PDF/UA tagging (no engine). Keeping these absent is more trustworthy than shipping a poor result.

Arabic is a real risk, not a checkbox. PDF content streams often store Arabic glyphs in visual order, subset fonts often lack a good ToUnicode map, and a text layer placed in visual order makes search and copy fail even when the page looks right. Every PDF story needs a fixture set: a LibreOffice Arabic text PDF, a scanned book page, a two column page, a table, a form with Arabic labels, a right to left page with Latin inserts, one encrypted file. Minimal Arabic support in P0 is extraction with UAX #9 handling where the producer allows it, and a copy plus search test per fixture.

Licences: lopdf MIT, pdf-writer MIT or Apache-2.0, Ghostscript AGPL as a child process only, qpdf Apache-2.0, pdfcpu Apache-2.0, OCRmyPDF MPL-2.0, poppler GPL-2.0-or-later (compatible), pdfium BSD-3-Clause, MuPDF AGPL excluded from linking. Speed and memory traps: parsers hold the whole file, rewrites invalidate signatures, encrypted input needs an explicit password, and malformed files need repair before blame.

### 7.2 Images: the owner's own request

P0: resize modes (percent, longest side, exact, fit, fill with crop), crop with aspect presets and freeform, images to PDF with page size, margin and DPI, and the live preview with the readout. This is what the customization question is really about.

P1: straighten; compress to a target size ("under 400 KB") with a bounded quality search and an honest estimate; metadata keep or strip, which needs a copy path rather than a re-encode because every re-encode strips EXIF, XMP and ICC today; pad, border and flatten alpha; PDF to images with DPI and a page range; PNG lossless optimization with oxipng; SVG input with resvg; an info panel with dimensions, ratio, DPI, colour and metadata.

P2: ICC conversion (moxcms is already in the tree); better JPEG with mozjpeg (IJG, C build); JXL decode with jxl-oxide (the cheapest genuinely new format); HEIC and HEIF input (libheif LGPL-3.0, C, opt-in component, the most expensive new format and the first failure for every iPhone user); animation (gifski is AGPL, do not link); favicon and sprite sheet; upscaling and background removal as opt-in models under ADR 0010; OCR through Tesseract.

Clean licences: image 0.25 MIT or Apache-2.0, jpeg-decoder MIT or Apache-2.0 (needed for scaled preview), oxipng MIT, resvg Apache-2.0 or MIT, tiny-skia BSD-3-Clause, kamadak-exif BSD-2-Clause, jxl-oxide MIT or Apache-2.0 for decode. Hazards: gifski AGPL, jpegxl-rs GPL-3.0-or-later over libjxl, libheif LGPL plus bundled codecs, mozjpeg IJG. Each line goes into `docs/engines.md` before the dependency lands.

### 7.3 Other families and "complete"

Cheap wins outside PDF and images: keep or extract subtitles in video jobs (ffmpeg already has them; dropping them silently is data loss), and target size compression. Medium: ebook conversion through Calibre as a child process, archive unpacking. Not brand: mobile, cloud storage, share links, collaboration, in-app AI chat.

The "complete" bar in 2026 is set by cloud suites, and copying their breadth is a trap for a light local app. The right completeness is the file families a desktop user actually converts, with honest gaps stated in the UI.

### 7.4 Positioning: local hands for local agents

| Cloud converters | Kawaro |
| --- | --- |
| 25 to 60 tools, AI chat, summaries, translate | deterministic operations only; the agent supplies the intelligence |
| Account, credits, subscription, upload limits | no account, no credits, no upload, no watermark |
| Files deleted after processing | files never leave the machine |
| Mobile, web, collaboration, storage | filesystem native, output next to the original, batch as large as the disk |
| Install is a web page | install is a package; the MCP install must become one click or one paste |

The AI split is the strategy: do not embed chat, summarise, translate, upscale or background removal in the app. Expose clean primitives over MCP and let the user's own agent decide. That keeps the base light, the privacy claim true and the licence story simple, and it turns every local AI app into a Kawaro front end.

Competitors already ship MCP (Stirling PDF, CloudConvert, Docling), so "has MCP" is not an edge. The edge is local stdio, zero upload, filesystem native results and Arabic quality. Install friction and trust are the adoption gates: Anthropic's own Desktop Extensions page says install complexity was the top complaint, and LM Studio warns users that MCP servers can run code and read files.

## 8. Guardrails

Performance: the 150 ms hive promise does not move; no preview work before the first paint; processing never blocks the UI; progress stays in the tray; the stage 2 only runs while open; the bench harness and a CI budget script prove the numbers before release.

Design: tokens only, no shadows, no gradients, no one-sided borders, surfaces for elevation, glass only over a compositor blur, RTL mirrored by hand, Arabic and English written as originals, every interactive element labelled, no em dash or en dash anywhere.

Quality: one engine adapter per engine with fixtures; never overwrite; atomic placement; Arabic fixtures for PDF and text; licence lines in `docs/engines.md`; the mutation run stays outside the story gate.

Protocol: pin the SDK, keep one host as the acceptance target, budget one maintenance story per MCP revision, and make every feature degrade to a text answer when a host does not render it.

## 9. Roadmap and effort

Story numbers are proposals from S-117, the next free number, subject to the planner step. Effort is a guess for one experienced Rust developer including tests and fixtures.

| Phase | Work | Stories | Effort |
| --- | --- | --- | --- |
| 0. Decision | ADR 0011 local MCP server (stdio, off by default, roots, audit, MSRV). ADR 0012 options as data and the stage 2. Choose MSRV path. | ADRs only | 2 to 3 days |
| 1. Core contracts | Typed `[action.options]`, merge and validation, additive `Run.params`, JSON Schema export, CLI `--set`. No UI. | S-117 | 5 to 10 days |
| 2. Preview groundwork | Preview worker, scaled decode, draft cache, generation cancel, hexagon mask, shared `image_geom`, bench and CI budget. | S-118 | 5 to 10 days |
| 3. Stage 2 | Customization for resize only: readout, presets, Run and Reset, RTL, accessibility, re-measure the 150 ms promise. Crop after it passes. | S-119 | 5 to 10 days |
| 4. MCP read only | `kawaro mcp` stdio, `formats` and `plan`, config, allowed roots, audit, settings switch. Spike against Claude Desktop or Claude Code. | S-121 | 5 to 8 days |
| 5. MCP write | convert, images_to_pdf, pdf, image, job status and cancel, install helper and snippets, docs in English and Arabic. | S-122 | 5 to 8 days |
| 6. PDF P0 | Page organiser, split ranges, images to PDF options, compress DPI, metadata, PDF to text. qpdf spike, then engine. | S-123 to S-126 | 10 to 20 days |
| 7. Breadth | Image crop, target size, metadata; PDF protect and unlock, watermark and page numbers, N-up and booklet; subtitles. OCR last, it is the heaviest dependency. | S-127 onwards | 15 to 30 days |
| 8. Later | MCP Apps preview card, Skills package and registry listing, recipes and pipelines, HEIC opt-in, JXL decode, PDF to Markdown opt-in. | later | after trust is earned |

Order rationale: options and preview are the foundation for both workstreams; MCP read-only can be tested before write tools exist, which keeps the security review small; PDF P0 follows because it is visible and cheap; OCR and MCP Apps wait because they are heavy and host dependent.

A realistic milestone for "a complete local converter with agent integration" is **two to three months of focused work** (guess), excluding hand testing on each host and excluding model components. It is a large piece of work, not a weekend.

## 10. Risks

1. Protocol churn: two revisions in nine months, the last one removed the handshake, sessions and ping. Pin the SDK and budget maintenance.
2. Host variance: timeouts, missing progress UI, images drawn or not, links rendered as nothing. Every feature must degrade to text.
3. Security review: a process that writes user files on behalf of a model needs a written threat model and a reviewer who did not write it.
4. MSRV and tokio: rmcp needs 1.88; raising it touches CI toolchains and distribution. Hand-writing is possible but is surface to maintain.
5. Scope creep: options, preview, MCP, PDF breadth and OCR at once will stall all of them. The order above is the defence.
6. Performance regression: preview work on the UI thread, huge files, text relayout. The budget script is the defence.
7. Arabic quality: extraction order, ToUnicode gaps, text layer order, shaping in generated text. Fixtures are the defence.
8. Schema drift: if the UI, CLI and MCP parse TOML independently, they will disagree. Only `core` reads the schema.
9. Licence hazards: libheif, gifski, jpegxl-rs, MuPDF. Each needs a line and a decision before code.
10. Unrealistic promises: an interactive panel in every chat, instant preview of 100 MP, OCR that beats cloud on hard scans. Say what is true.

## 11. Decisions for the owner

| Decision | Options | Recommendation | Why it matters now |
| --- | --- | --- | --- |
| Rust MSRV | Raise to 1.88 and use rmcp, or hand-write the stdio loop | Raise if CI and distributions allow; otherwise hand-write | Blocks the MCP story |
| MCP default | Off, read-only, or write enabled | Off, read-only first, write behind a second switch | Trust is the adoption gate |
| Allowed roots | Home, or an explicit list | Documents, Pictures, Downloads; empty means no writes | Prevents the first accident report |
| MCP Apps | Now, later, never | Later, after plain tools | Churn and a web bundle |
| HEIC | Base, opt-in, never | Opt-in component under ADR 0010 | Licence and install weight |
| OCR | Base, opt-in, later | Later, OCRmyPDF plus Tesseract as a child process | Heaviest dependency |
| qpdf | New base engine, opt-in, skip | Spike first, then decide | Adds a second child engine |
| Options schema | UI first or data first | Data first (S-117) | It unblocks UI, CLI and MCP |

## 12. Recommendation

Do it, in two waves, and keep the promises narrow.

Wave one, the foundation: options as data, the preview worker and shared geometry, then stage 2 for resize. This is the owner's customization request, it is the largest product gap against every scanned competitor, and it is the prerequisite for meaningful MCP arguments and PDF options. It needs no new engine, no model and no network.

Wave two, the agent layer: `kawaro mcp` over stdio, off by default, read-only first, six to eight tools, allowed roots, audit, per-tool toggles, an install helper, and one acceptance host. The result shape is an image preview plus a text path plus a resource link plus structured fields, with the file next to the originals as the guarantee. Claim no more than that, and add MCP Apps only when the plain tools are trusted.

Then breadth: PDF P0 first (page organiser, images to PDF options, compress controls, metadata, PDF to text), then image crop, target size and metadata, then protect and unlock, watermark and page numbers, N-up and booklet, then OCR. Never embed a model in the app; never call the network; never offer a cloud answer.

This is a good, practical and feasible plan. It is also a large one: two to three months of focused work for the full milestone. The parts that decide whether it succeeds are not the protocol or the engines, they are the options contract, the preview budget and the trust model. Build those first.

## Sources

The four companion files carry the full source lists. Key primary pages checked on 2026-11-04:

- MCP specification, revision 2026-07-28: https://modelcontextprotocol.io/specification/latest
- MCP key changes for 2026-07-28: https://modelcontextprotocol.io/specification/2026-07-28/changelog
- MCP tools, content blocks, output schema: https://modelcontextprotocol.io/specification/2026-07-28/server/tools.md
- MCP local server security: https://modelcontextprotocol.io/docs/2026-07-28/tutorials/security/local-server-security.md
- MCP extensions, Apps, Tasks, Skills: https://modelcontextprotocol.io/extensions/overview.md
- MCP Apps supported clients: https://raw.githubusercontent.com/modelcontextprotocol/ext-apps/main/README.md
- MCP registry: https://registry.modelcontextprotocol.io/
- Official Rust SDK, rmcp 3.8.0: https://github.com/modelcontextprotocol/rust-sdk
- Claude Desktop local servers: https://modelcontextprotocol.io/docs/2026-07-28/develop/connect-local-servers.md
- Claude Code MCP: https://code.claude.com/docs/en/mcp.md
- Cursor MCP: https://cursor.com/docs/context/mcp
- LM Studio MCP: https://lmstudio.ai/docs/app/mcp
- Anthropic Desktop Extensions: https://www.anthropic.com/research/desktop-extensions
- Stirling PDF MCP: https://docs.stirlingpdf.com/MCP/
- CloudConvert MCP: https://cloudconvert.com/mcp-server
- qpdf manual: https://qpdf.readthedocs.io/en/stable/cli.html
- OCRmyPDF: https://ocrmypdf.readthedocs.io/en/latest/
- Ghostscript devices: https://ghostscript.readthedocs.io/en/latest/Devices.html
- lopdf: https://crates.io/crates/lopdf
- pdf-writer: https://crates.io/crates/pdf-writer
- jpeg-decoder scaled decode: https://docs.rs/jpeg-decoder/latest/jpeg_decoder/struct.Decoder.html
- Slint renderer limitations: https://docs.slint.dev/latest/docs/slint/guide/backends-and-renderers/backends_and_renderers/
- Slint software renderer and dirty regions: https://docs.slint.dev/latest/docs/rust/slint/platform/software_renderer/struct.SoftwareRenderer
- UAX #9 bidi: https://www.unicode.org/reports/tr9/
- In repo: docs/architecture.md, docs/engines.md, docs/design-system.md, docs/adr/0010-opt-in-model-downloads.md, docs/research/ai-edition-tiers.md, actions/*.toml, crates/core/src/action.rs, crates/core/src/ipc.rs, crates/ui/src/preview.rs
