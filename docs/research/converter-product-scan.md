# Converter product scan: what Kawaro should copy, refuse and build next

Status: research note, decision input for the owner question of November 2026.
Date: 2026-11-04. Scope: product, UX, pricing, "completeness" and the MCP and AI-agent angle.
Method: primary product pages fetched with `web_fetch`; DuckDuckGo HTML only to find URLs, because the search APIs failed in this session. 25 products and 6 MCP or AI sources were read. Cells marked **unverified** could not be checked against a primary page today. Effort letters (S, M, L) are guesses, not measurements. No file in this repo other than this one was modified.

## 1. What Kawaro already ships (baseline for the gap analysis)

From `actions/*.toml` (45 actions) and `docs/engines.md` / `docs/architecture.md`:

| Family | Ops today | Formats |
| --- | --- | --- |
| Image | convert, resize (percent or max side), rotate, flip, grayscale, compress, strip, to PDF | png jpg webp avif gif bmp tiff ico |
| PDF | compress (Ghostscript), merge, split to zip, rotate pages, pages to PNG zip | pdf only (no password, no text export) |
| Documents | to PDF, docx/odt, xlsx/ods, pptx/odp | doc(x) odt rtf txt md xls(x) ods csv ppt(x) odp |
| Video | to MP4, WebM, MKV, MOV, GIF, compress, extract audio | no subtitle passthrough |
| Audio | mp3 m4a ogg flac opus wav | first audio stream only |

Hard limits that the scan shows users will hit: no per-run options (params are baked into TOML), no HEIC or HEIF input, no AVIF input, no encrypted PDF handling, subtitles always dropped, one output file per task so multi-page output becomes a ZIP, progress only in the tray.

## 2. Product scan (25 products)

Pricing is list price on the vendor page on 2026-11-04 and may be regional or promotional. "Copy" means a pattern worth adopting; "avoid" means a pattern that conflicts with the brand or the design rules.

| Product | What it does | Free tier or price | Standout feature or UX | Copy or avoid |
| --- | --- | --- | --- | --- |
| [iLovePDF](https://www.ilovepdf.com/pricing) | 25+ PDF tools: merge, split, compress, OCR, redact, sign, forms, compare, PDF/A, page numbers, watermark, crop, AI summary, translate, PDF to Markdown | Free Basic with limited processing; Premium 4 US$ per month billed yearly (48 US$ per year) or 7 US$ monthly; 2000 AI credits | Desktop app for Mac and Windows and mobile apps under one account; Arabic UI in its language list | Copy: named per-task outcomes ("Compress PDF"). Avoid: account gating of core operations |
| [iLoveIMG](https://www.iloveimg.com/pricing) | Compress, resize, crop, convert, HEIC to JPG, remove background, upscale, blur face, meme generator | Free tier plus paid plans, exact price **unverified** | Single-purpose landing pages per conversion, one "Select images" action | Copy: task-first page naming. Avoid: AI image toys in a converter |
| [Smallpdf](https://smallpdf.com/pricing) | PDF suite, e-sign, AI chat and summary | Free tier with hourly limits, Pro subscription, price **unverified** (page is script rendered) | Very short funnel from drop to result, strong brand polish | Copy: minimal steps to result. Avoid: subscription wall in the middle of a task |
| [Adobe Acrobat online](https://www.adobe.com/acrobat/online.html) | 25+ PDF and e-signing tools, AI Assistant chat, summary, PDF AI | Tools are free to try; AI features are trials inside a paid plan (plan prices **unverified**) | Format authority and deep AI Assistant integration | Copy: clear tool taxonomy. Avoid: cloud AI on user documents |
| [Sejda](https://www.sejda.com/pricing) | PDF editor, forms, OCR, convert from PDF | Free tier with page and hourly limits and tool specific limits, no signup needed; paid plans add Desktop and Web; trial does not auto renew | Desktop plus web under one licence, explicit limits written down | Copy: publishing the limits honestly. Avoid: limit counters that surprise the user mid-task |
| [PDF24](https://tools.pdf24.org/en/) | Large free PDF tool set plus PDF24 Creator for offline use | "100 percent free of charge", "no artificial limits" | Free, no account, offline companion app | Copy: offline app as the trust anchor. Avoid: Windows only offline build |
| [Stirling PDF](https://docs.stirlingpdf.com/MCP/) | 60+ PDF operations, pipelines, server, desktop, built-in MCP server | Open source core (MIT, with a proprietary folder, see its [LICENSE](https://raw.githubusercontent.com/Stirling-Tools/Stirling-PDF/main/LICENSE)); hosted plans exist, prices **unverified** | Built-in MCP server: cloud endpoint runs as your account, self-hosted endpoint off by default with OAuth or API key; docs name Claude, Codex and VS Code as clients | Copy: MCP as a first-class feature. Avoid: making the local endpoint opt-in "off" by default (Kawaro should default to local stdio) |
| [PDF2Go](https://www.pdf2go.com/) | 50 apps: organise, rotate, crop, watermark, redact, OCR, compress, protect, PDF/A, compare, AI chat, translate, extract facts, spellcheck, AI repair | "Free to try", limits **unverified** | AI features flagged "New" across the whole tool list, constant feature velocity | Copy: breadth visible in one page. Avoid: AI features that silently upload the file |
| [CloudConvert](https://cloudconvert.com/mcp-server) | Hundreds of formats across documents, images, audio, video, ebooks, archives; custom FFmpeg and ImageMagick jobs | Free 10 conversion credits per day, then credit packages or monthly plans ([pricing](https://cloudconvert.com/pricing)) | Official MCP server at mcp.cloudconvert.com with ChatGPT and Claude connectors; server side processing on ISO 27001 infrastructure | Copy: multi-step jobs. Avoid: upload based processing as the default |
| [Convertio](https://convertio.co/pricing/) | File conversion across many formats | Credits measure server time; paid plans add unlimited conversion minutes per day | Credit model explained in plain words, only successful conversions charged | Copy: charge or count only on success. Avoid: opaque credit math |
| [TinyWow](https://tinywow.com/) | Free AI writing, PDF and image tools | Free with limits, details **unverified** | Very wide free tool surface aimed at students | Avoid: ad driven, cloud only, low trust signal |
| 11zon | Many small free converters and PDF tools | **Unverified** (the page did not render in a fetch) | Small single-purpose tools | Copy: tool granularity. Avoid: ad heavy pages |
| [Aconvert](https://www.aconvert.com/) | Document, ebook, image, icon, video, audio, archive conversion | Free online, limits **unverified** | Format coverage across unusual families (icon, ebook, archive) | Copy: ebook, icon and archive as first-class families |
| FreeConvert | Converter with many formats | **Unverified** (page blocked the fetch) | Broad format list, server side | Avoid: no offline story |
| [Office Converter](https://www.office-converter.com/) | 400+ formats, document, video, audio, image, ebook, presentation, archive, font, OCR | Paid plans plus limited free, prices **unverified** | 39 interface languages, a full format family map on one page | Copy: the family map as documentation; Arabic is not a differentiator by itself |
| [Canva](https://www.canva.com/) | Design tool with PDF and PNG export | Free tier plus Pro subscription, prices **unverified** | Export presets (size, quality, PDF standard) inside a design workflow | Copy: export presets. Avoid: cloud project storage |
| [Photopea](https://www.photopea.com/) | Browser image editor | Free with ads and premium features | "Fully local: there are no uploads. Photopea runs on your device" | Copy: privacy message that is also true. Avoid: browser only |
| [Squoosh](https://squoosh.app/) | Image compression in the browser | Free | Side by side before and after, live file size, per-format quality slider, "images never leave your device" | Copy: live size and quality readout. Avoid: assuming its maintenance continues (status **unverified**) |
| [XnConvert](https://www.xnview.com/en/xnconvert/) | Batch image converter, version 1.118.0, over 80 actions | Free, cross platform | Batch action list with per-action enable, output tab, "Convert" one button, profile save | Copy: action chain plus per-action toggle. Avoid: dense desktop dialog for a hive product |
| [HandBrake](https://handbrake.fr/) | Video transcoder, version 1.11.2 | Free, open source, GPLv2 per its [LICENSE](https://raw.githubusercontent.com/HandBrake/HandBrake/master/LICENSE) | Built-in device presets, batch scan and queueing, chapter and range selection, subtitle tracks ([features](https://handbrake.fr/features.php)) | Copy: queue with per-job settings. Avoid: preset names as the only explanation |
| [Shutter Encoder](https://www.shutterencoder.com/) | FFmpeg based converter for professional codecs, "open source software without any restriction" | Free, donation supported, licence detail **unverified** | Separate function and options stages, batch processing (screen description unverified today) | Copy: function then options then start. Avoid: long expert option lists in the hive |
| [ShareX](https://getsharex.com/) | Capture, record, share and automate on Windows, 34 million downloads | Free, open source | "Actions" workflows, custom uploaders, OCR, image editor, keybinds, privacy first claim | Copy: workflow automation and keybinds. Avoid: uploaders and cloud destinations by default |
| macOS Preview | System PDF and image viewer with markup and export | Free with macOS, details **unverified** | Instant open, annotate, export page as image | Copy: instant open. Avoid: hidden batch limitations |
| Windows Photos | System image viewer with crop, rotate, filters | Free with Windows, details **unverified** | Trivial edits in two clicks | Copy: two click basics. Avoid: no export control |
| [Docling](https://docling-project.github.io/docling/usage/mcp/) | 2026 AI-native local document conversion to structured data, with an MCP server, MIT per its [LICENSE](https://raw.githubusercontent.com/docling-project/docling/main/LICENSE) | Free, open source | Local MCP server that agents call; conversion and chunking as tools; no cloud needed | Copy: local tools served to agents. Avoid: document model sprawl beyond Kawaro's scope |

## 3. What "complete" means in 2026, by the numbers

Tool counts are marketing, but they set the expectation: iLovePDF lists 25+ PDF tools, Stirling PDF says 60+ PDF operations, PDF2Go says 50 apps, Office Converter says 400+ formats and 39 languages, CloudConvert says hundreds of formats across families. A converter that offers 5 PDF operations reads as a beta next to those.

The 2026 pattern is AI attached to every document action: chat with a PDF, summarise, translate, extract facts, spellcheck, "AI repair" (PDF2Go, Adobe, iLovePDF, TinyWow). All of it runs on the vendor's servers in these products. That is exactly the space Kawaro should leave to the user's own agent, with Kawaro as the local hands.

Desktop is no longer a differentiator by itself: iLovePDF has a Desktop app for Mac and Windows, Sejda has Sejda Desktop, PDF24 has PDF24 Creator, and Stirling PDF documentation has a Desktop section. Local privacy, no account, filesystem integration, speed and Arabic quality are the differentiators that remain.

## 4. Completeness gaps, ranked by how fast users notice

| Rank | Gap | Evidence | Kawaro today | Why it hurts | Verdict |
| --- | --- | --- | --- | --- | --- |
| 1 | PDF page organiser: delete, extract, reorder, crop, insert | iLovePDF Organize, PDF2Go Organize, Stirling page operations | only rotate, split, merge | the second thing anyone does with a PDF | must |
| 2 | PDF to text or Markdown | iLovePDF "PDF to Markdown", Stirling convert | none | blocks local agents and search; no AI needed, just text | must |
| 3 | OCR, Arabic first, searchable PDF | iLovePDF OCR, PDF2Go OCR, Stirling OCR | none | Arabic scans are the home audience | soon (already ranked 2 in `ai-edition-tiers.md`) |
| 4 | Watermark, page numbers, headers and footers | iLovePDF, PDF2Go, Stirling | none | routine office and legal work | soon |
| 5 | HEIC and HEIF input | CloudConvert HEIC page, iLoveIMG HEIC to JPG page | refused in `docs/engines.md` | every iPhone photo fails | soon |
| 6 | Protect and unlock PDF | iLovePDF, PDF2Go, Stirling | encrypted PDFs rejected with an error | receiving a protected PDF is common | soon |
| 7 | Image crop, aspect ratio, target file size | iLoveIMG crop and compress | fixed resize and compress only | the owner's own request to customise | must |
| 8 | PDF to Word, Excel, PowerPoint | iLovePDF, Adobe, Office Converter | none | high demand, but offline quality is limited | later, with honest quality note |
| 9 | Subtitles kept or extracted on video conversion | HandBrake exposes subtitle tracks in its feature list | dropped silently | silent data loss on every video job | soon, cheap |
| 10 | Ebook conversion (EPUB, MOBI) | Office Converter ebook family, Aconvert ebook | none | niche but expected of a "complete" converter | later, Calibre as child process |
| 11 | Archive conversion and unpack | CloudConvert, Office Converter archive | none | low brand value | later |
| 12 | Redaction and signing | iLovePDF, Adobe, Stirling | none | legal value, large effort and liability | later, never in cloud form |

## 5. Pricing and positioning

| Observation | Evidence |
| --- | --- |
| The core is free somewhere. PDF24 is free with no limits; HandBrake, ShareX, Photopea, Squoosh and XnConvert cost nothing; Sejda has a free tier | [PDF24](https://tools.pdf24.org/en/), [Photopea](https://www.photopea.com/), [Squoosh](https://squoosh.app/) |
| What people pay for is convenience, cloud processing, AI and accounts: iLovePDF Premium 4 US$ per month billed yearly or 7 US$ monthly, CloudConvert credits from 10 per day free | [iLovePDF pricing](https://www.ilovepdf.com/pricing), [CloudConvert pricing](https://cloudconvert.com/pricing) |
| Batch size and file size are the usual paid walls, not quality | [Convertio](https://convertio.co/pricing/) sells conversion minutes, [CloudConvert](https://cloudconvert.com/pricing) sells credits, [iLovePDF](https://www.ilovepdf.com/pricing) sells unlimited processing |
| Free local tools win on trust, not features: Photopea and Squoosh both lead with "nothing leaves your device" | [Photopea](https://www.photopea.com/), [Squoosh](https://squoosh.app/) |

Kawaro wins where the cloud product is structurally weak: unlimited size, no upload time, no account, no watermark, offline, batch as large as the disk allows, output next to the original, Arabic UI written as an original. Kawaro loses on mobile, web, collaboration, storage, and AI features that people now expect inside the document tool.

## 6. UX patterns to copy

| Pattern | Best example | How it maps onto the hive without breaking the rules |
| --- | --- | --- |
| Defaults first, one click to run | iLovePDF, PDF24 | keep today's behaviour: a cell click runs with baked params |
| Progressive disclosure of options | Squoosh, Shutter Encoder | a "tune" cell in ring 2 grows the hive into a second ring stage, never a new window |
| Live result readout (size, dimensions, ratio, page count) | Squoosh size readout, iLoveIMG compress | one line under the preview in `text-secondary`; numbers recomputed by a worker thread, never on the UI thread |
| Presets per action and "apply to all" | XnConvert profiles, HandBrake presets | presets become extra `[action]` entries in TOML, so no new UI is needed |
| Batch queue with per-file state | XnConvert, HandBrake queue, CloudConvert jobs | the backend already has a job queue; surface it as a settings page list, not a focus window |
| Remember the last choice per action | XnConvert, Shutter Encoder | store in config; no dialog |
| Drag anywhere, paste a file, open with a shortcut | ShareX, Squoosh paste | already partly there with drop and clipboard; add paste in the hive |
| Chained recipe as one job | CloudConvert multi-step, XnConvert action chain | later: a recipe is a TOML list of (op, params), one MCP tool, one output |
| Honest quality and size tradeoff | Squoosh | show estimated output size and a plain quality sentence, never a fake "best" |
| Privacy message that is verifiable | Photopea, Squoosh, ShareX "privacy first" | one line in the hive footer, checked by a test: no network code in the processing path |

What not to copy: dense expert dialogs (XnConvert, Shutter Encoder), preset names without explanation (HandBrake), ad or credit framing, upload spinners, and any layout that needs a shadow or gradient to read.

## 7. Cloud-only features: refuse or replace

| Cloud feature | Who sells it | Kawaro answer |
| --- | --- | --- |
| Chat with a PDF, summarise, translate, extract facts | Adobe AI Assistant, PDF2Go, iLovePDF | local text export (PDF to Markdown) plus the user's own agent through MCP; Kawaro never calls a model service |
| AI repair or "optimise" | PDF2Go, iLovePDF | deterministic qpdf or Ghostscript rewrite with an honest result |
| Background removal, upscale | iLoveIMG | opt-in local model per ADR 0010, or refuse; never a cloud call |
| Cloud storage, share links, collaboration | iLovePDF, Smallpdf, CloudConvert | refuse; the output is a file next to the original, optionally in a user chosen folder |
| Accounts, credits, subscriptions | all of the above | never in the base app |
| Web page to PDF | CloudConvert capture | later and explicit: it is a network fetch of a public page, not a file leaving the machine, but it adds network code and should need a visible opt-in |
| Telemetry and analytics | most cloud tools | never |

## 8. MCP and the AI-agent angle

Evidence from 2026:

| Source | What it shows |
| --- | --- |
| [Stirling PDF MCP docs](https://docs.stirlingpdf.com/MCP/) | a PDF product now ships a built-in MCP server, cloud endpoint plus self-hosted endpoint that is off by default with OAuth or API key; Claude, Codex and VS Code are named clients |
| [CloudConvert MCP server](https://cloudconvert.com/mcp-server) | a converter sells an MCP endpoint at mcp.cloudconvert.com with ChatGPT and Claude connectors; processing stays on its servers, files "deleted after processing" |
| [Docling MCP](https://docling-project.github.io/docling/usage/mcp/) | a local, MIT licensed document converter exposes its work as MCP tools, no cloud needed |
| [Anthropic Desktop Extensions](https://www.anthropic.com/research/desktop-extensions) | one-click local MCP installs via a bundle file, now `.mcpb` (September 2025 update on that page); install complexity was the main user complaint, and there is an extension directory and a security chapter |
| [LM Studio MCP docs](https://lmstudio.ai/docs/app/mcp) | LM Studio is an MCP host since 0.3.17 and warns: servers "can run arbitrary code, access your local files, and use your network connection... If you don't trust the source, don't install it" |
| [Invariant tool poisoning](https://invariantlabs.ai/blog/mcp-security-notification-tool-poisoning-attacks) | tool descriptions can be poisoned to exfiltrate data; Anthropic, OpenAI, Zapier and Cursor were listed as affected in April 2025 |

What users and vendors say about trust: the caution comes from the hosts themselves, not from privacy advocates. LM Studio puts the warning in its own documentation; Invariant shows the attack works; Anthropic ships a security section inside its one-click install announcement. The lesson for Kawaro is that the install story and the permission story are the product, not documentation extras.

What Kawaro should offer an agent (product shape, not protocol detail; the protocol design is task-1):

1. `kawaro mcp` over stdio, launched by the host, no network listener, logs to stderr only.
2. A `.mcpb` bundle plus copy-paste snippets for hosts that do not take bundles, so install is one click or one paste ([mcpb repo](https://github.com/modelcontextprotocol/mcpb)).
3. Tools named for user intent: list actions, inspect a file, convert, images to PDF, PDF pages, job status, cancel. Options mirror the TOML option schema, so UI, CLI and MCP cannot drift.
4. A result an agent can show in chat: thumbnail as image content, a resource link to the file, and a text line with the path, plus structured fields (bytes, dimensions or pages, engine, parameters, elapsed ms).
5. Trust controls that make enabling it reasonable: per-tool switches, folder allowlist, preview before write, dry run, an audit log page in settings, one-click revoke, no delete or overwrite ever, and no network calls from the MCP path.
6. Long jobs return a job id immediately and the agent polls status, matching the existing queue and the tray-only progress rule.

Nothing here needs a downloaded model, so it stays inside ADR 0010 and the light base install. The full host matrix (Claude Desktop, Claude Code, Cursor, VS Code, Zed, Open WebUI, LibreChat and the rest) is task-1's job; this note only records the hosts whose own documentation I read here.

## 9. Recommendations

Impact and effort are judgement calls; effort S is under one story, M is one to two, L is three or more.

| # | Idea | Impact | Effort | Verdict | Engine and licence | Pillar |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | Options as data: an `[action.options]` schema in `actions/*.toml`, shared by UI, CLI and MCP, with a hive second stage only when the user asks | high | M | must (S-117 candidate) | none new | keeps the hive fast; options are declarative |
| 2 | PDF page organiser in pure Rust: delete, reorder, rotate, extract, crop | high | M | must | lopdf, MIT | local, no new process |
| 3 | PDF to text and Markdown | high | S to M | must | a pure Rust extractor (licence to check) or poppler pdftotext as a child process | unlocks agents without a model |
| 4 | `kawaro mcp` plus a `.mcpb` bundle and the permission model above | high | M to L | must | official Rust SDK, check licence first | local only, no telemetry |
| 5 | Images to PDF with page size, orientation, fit and margin, and a live page count | high | S | must | pdf-writer, MIT | preview inside the hive |
| 6 | Presets and "apply to all" for batches, remembering the last choice | medium | S | must | none | fewer clicks, same speed |
| 7 | Compress to a target size ("under 2 MB") with an honest quality line | high | M | soon | image and Ghostscript, existing | honest tradeoffs |
| 8 | OCR with Tesseract, Arabic first, searchable PDF output | high | M | soon | Tesseract child process, the OCR engine already planned | local, opt-in engine |
| 9 | PDF watermark, page numbers, headers and footers | medium | S | soon | lopdf or pdf-writer | table stakes |
| 10 | PDF protect and unlock (AES), password asked in the hive | medium | M | soon | lopdf, MIT, for unlock; writing password protection needs a check first | never stores the password |
| 11 | HEIC and HEIF input through an opt-in component | high | M | soon | libheif (LGPL, verify build flags) or a bundled decoder, needs a licence decision | ADR 0010 path, not base |
| 12 | Undo: one click removes the outputs a job created, and a job history page in settings | medium | S | soon | none | trust, no focus stealing |
| 13 | Keep or extract subtitles in video jobs | medium | S | soon | ffmpeg, existing child process | stops silent data loss |
| 14 | Recipes: a TOML list of ops run as one job, exposed as one MCP tool | high | L | later | existing engines | one cell, one output |
| 15 | Ebook conversion through Calibre as a child process | medium | M | later | Calibre, GPL-3.0 | local |
| 16 | PDF to Word, Excel, PowerPoint | high | L | later | LibreOffice or a new engine, quality risk | must stay honest about quality |
| 17 | Mobile app, web app, cloud storage, collaboration, in-app AI chat | low for the brand | L | never | none | violates the privacy pillar; delegate to the user's agent |

Licence, trademark and patent notes: do not copy competitor names, logos, icons, screenshots or marketing copy; do not imitate a distinctive visual layout such as the iLovePDF or Squoosh chrome, only the interaction idea. Ghostscript stays a separate AGPL program (already the case). Stirling PDF is MIT but with a proprietary folder, so read before reusing any of it. Calibre is GPL-3.0 and safe as a child process. No patent search was done for this note; file conversion steps are long standing practice, but avoid copying distinctive animations or interaction sequences from patented consumer products, and keep the hive the visual identity.

## 10. Where this contradicts the current plan

- `docs/engines.md` treats the missing HEIC decoder as a small note; for photo users it is the first failure they meet, so it deserves a story, not a footnote.
- "No per-run customisation" is the largest product gap against every product in this scan, and the owner already asked for it; it should lead the next block, not follow PDF breadth.
- The one-output-per-task engine contract blocks page ranges and "one page per file" folders; a page organiser must write one PDF to stay inside it, which is acceptable.
- `docs/research/ai-edition-tiers.md` puts OCR second behind speech to text; this scan would put OCR first for the base install because it fixes a completeness gap, while speech is a new family.
- MCP does not require a model download, so it is compatible with ADR 0010 and with the light install; it should not wait for the model work.
- The 150 ms budget and the tray-only progress rule mean the options stage must live inside the same hive surface and the job list inside settings, not in new windows.

## 11. Open questions and things not verified

- Competitor prices are list prices on 2026-11-04 and vary by region and promotion; Smallpdf, Canva, 11zon, FreeConvert, macOS Preview, Windows Photos and the Office Converter plans are marked unverified because their pages did not render or blocked the fetch.
- Whether MCP hosts pass image content to the model, or only show it, was not tested here and changes the preview design.
- Stirling PDF self-hosted MCP authentication details, and whether its desktop build can run its MCP server, are unverified.
- Squoosh maintenance status is unverified; treat its UX, not its code, as the reference.
- Arabic UI support among competitors is partial evidence from their language lists (iLovePDF shows Arabic, Office Converter claims 39 languages); quality was not tested.
- Real demand numbers (search volume, forum complaints) were not collected; the ranking in section 4 is based on product coverage, which is weaker evidence.

## Sources

All URLs accessed 2026-11-04.

- iLovePDF pricing: https://www.ilovepdf.com/pricing
- iLoveIMG pricing: https://www.iloveimg.com/pricing
- iLoveIMG HEIC to JPG: https://www.iloveimg.com/convert-to-jpg/heic-to-jpg
- Smallpdf pricing: https://smallpdf.com/pricing
- Adobe Acrobat online: https://www.adobe.com/acrobat/online.html
- Sejda pricing: https://www.sejda.com/pricing
- PDF24 Tools: https://tools.pdf24.org/en/
- Stirling PDF MCP docs: https://docs.stirlingpdf.com/MCP/
- Stirling PDF site: https://www.stirling.com/
- Stirling PDF licence: https://raw.githubusercontent.com/Stirling-Tools/Stirling-PDF/main/LICENSE
- PDF2Go: https://www.pdf2go.com/
- CloudConvert MCP server: https://cloudconvert.com/mcp-server
- CloudConvert pricing: https://cloudconvert.com/pricing
- CloudConvert HEIC to JPG: https://cloudconvert.com/heic-to-jpg
- Convertio pricing: https://convertio.co/pricing/
- TinyWow: https://tinywow.com/
- Aconvert: https://www.aconvert.com/
- Office Converter: https://www.office-converter.com/
- Photopea: https://www.photopea.com/
- Squoosh: https://squoosh.app/
- XnConvert: https://www.xnview.com/en/xnconvert/
- HandBrake: https://handbrake.fr/
- HandBrake licence: https://raw.githubusercontent.com/HandBrake/HandBrake/master/LICENSE
- Shutter Encoder: https://www.shutterencoder.com/
- ShareX: https://getsharex.com/
- Canva: https://www.canva.com/
- Docling MCP: https://docling-project.github.io/docling/usage/mcp/
- Docling licence: https://raw.githubusercontent.com/docling-project/docling/main/LICENSE
- Anthropic Desktop Extensions: https://www.anthropic.com/research/desktop-extensions
- MCP bundle repository: https://github.com/modelcontextprotocol/mcpb
- LM Studio MCP docs: https://lmstudio.ai/docs/app/mcp
- Invariant tool poisoning: https://invariantlabs.ai/blog/mcp-security-notification-tool-poisoning-attacks
- Repo evidence: `actions/*.toml`, `docs/engines.md`, `docs/architecture.md`, `docs/PLAN.md`, `docs/research/ai-edition-tiers.md`, `docs/adr/0010-opt-in-model-downloads.md`
