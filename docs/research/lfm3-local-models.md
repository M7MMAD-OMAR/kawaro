# Research: Liquid AI LFM3 as a local model engine for Kawaro

Status: reviewed once by an independent agent, corrections applied; spike results added in section 8b (2026-10-29). Not a decision. A decision needs ADR 0010 (see section 8).

Labels used below: **V** = verified by reading the cited page, **U** = unverified or not found, **M** = marketing figure published by the vendor only. All page text was read through a summarising fetch tool, so numbers are as extracted, not from raw HTML. Re-check any figure before it goes into code or a public doc.

## 1. What LFM3 is

Announced 2026-10-27 by Liquid AI (V, https://www.liquid.ai/blog/lfm3). A family of small open-weight models, all branched from LFM3-1.7B-Base (22T tokens of pre-training).

| Name | Params | Input | Output | Context | GGUF (V unless noted) |
|---|---|---|---|---|---|
| LFM3-1.7B | 1.73B | text | text | 256K | Q4_K_M 1.1 GB, Q6_K 1.43, Q8_0 1.85, QAD-Q4_0 1.05, BF16 3.47 |
| LFM3-VL-2B | 1.92B (LM 1.73B, vision encoder 164M, projector 25M) | text, image, video | text | 128K | same LM sizes, plus mmproj 379 MB (BF16) or 203 MB (Q8_0) |
| LFM3-Audio-2B | about 2B | text, audio | text, speech | 32K | repo linked, docs say "Coming soon" (U) |
| LFM3-Omni-2B | 2.17B | text, image, video, audio | text, speech | 32K | Q4_K_M 1.1 GB, plus mmproj 541 MB (Q8_0) or 950 MB (BF16) |
| d1-2B (decision) | 1.92B | text, image, video | no tokens, single forward pass | 128K (unreliable past about 64K) | 7 quantised variants linked, formats unstated (U); documented route is Transformers with trust_remote_code, vLLM, SGLang |
| d1-Omni-2B (decision) | 2.17B | text, image, audio | no tokens, single forward pass | 128K | linked, not opened (U) |
| LFM3-1.7B-Base | 1.73B | text | text | 256K | not listed |

Other formats (V, https://huggingface.co/collections/LiquidAI/lfm3): ONNX for the six task models, FP8 and NVFP4 for 1.7B and VL-2B, MLX 4-bit and 5-bit for text and VL.

Architecture of the text model (V): 24 layers, 13 gated short-convolution, 5 sliding-window attention (256 token window), 6 global attention; 128K vocabulary. This is why the KV cache is small: 1.6 GB at 128K context (M, vendor number).

## 2. What is verified and what is not

Verified from model cards and the llama.cpp PR:

- Arabic is in the 16 supported text languages: English, Arabic, Chinese, French, German, Italian, Japanese, Korean, Portuguese, Spanish, Vietnamese, Thai, Indonesian, Hindi, Russian, Polish (V, model cards).
- Licence is the **LFM Open License v1.0**, not Apache (V, https://huggingface.co/LiquidAI/LFM3-1.7B/blob/main/LICENSE). See section 6.
- llama.cpp support merged 2026-10-27, PR 31252 (V, https://github.com/ggml-org/llama.cpp/pull/31252). Upstream smoke tests covered F32, Q8_0 and Q4_0 only. Q4_K_M was not tested there. Video input is not supported there yet.
- Minimum build: only the VL-2B card states **b11960** (V). The 1.7B card says "a build with LFM3 support" and the Omni card gives none. Downstream threads quote b11938, b11940 and b11946 (U). Pin b11960 or newer. Distribution packages of llama.cpp older than the merge fail with an unknown architecture error.
- The VL-2B card shows `llama serve -hf ...` (V, that card only). The PR names no binary. Probe both `llama-server` and `llama serve` and pin by hash. Never use the `-hf` flag in Kawaro (section 4).
- VL-2B reports DocVQA val 96.7 and OCRBench v1 86.3 (V as reported, M as quality). No Arabic OCR result exists (U).
- Omni-2B and Audio-2B do speech recognition with the system prompt "Perform ASR in <language>" (V). Audio-2B live ASR is English only (V). Both the Audio-2B card (18 ASR languages) and the Omni-2B card list Arabic, but neither shows a measured Arabic score (V listed, U quality).
- Embeddings and translation are **not** advertised on any card (V as an absence). The d1 models do classification, routing, extraction and reranking, and are not chat models (V). Their documented route is Transformers with `trust_remote_code`, so whether llama.cpp serves them is open (U).

Vendor numbers that no independent party has reproduced (M): average 49.37 against 47.24 for Qwen3.5-9B; IFEval 96.66; BFCL V4 71.39; ToolSandbox 80.36; the 1.8 to 2.8 times decode speed claim (Q4_0, 32K context, unnamed devices). No independent benchmark, no CPU tokens per second on laptops, no Reddit, Hacker News or llama.cpp issue thread was found (U).

Independent signals, weak but useful (one hands-on test at https://www.mindstudio.ai/blog/liquid-ai-lfm3-1-7b-local-agent-model, the author sells a related product):

- Passed: JSON extraction, format constraints, grounded Q&A, tool calls.
- Failed: a bug fix (claimed success, changed nothing), generated web pages, translation across about 80 languages (invented words, wrong scripts).
- Vendor table agrees on the shape: weak at knowledge (GPQA Diamond 63.37 against 80.40; AA-Omniscience accuracy 7.07, high abstention), weak at long-context retrieval (RULER 128K 65.56 against 87.11), strong at instruction following and tool use.

Conclusion: treat LFM3 as a **router and structured-output model**, not a knowledge, coding or translation model.

## 3. Where it could help Kawaro

Ranked by fit with the principles in CLAUDE.md (local only, light base install, hive under 150 ms, original never modified).

1. **Natural language to action (text model, 1.1 GB).** The user types or says "make these smaller WEBP". The model picks an `id` from `actions/*.toml` and fills `[action.params]` as JSON constrained by a grammar. It never runs anything; the registry validates the choice and the engines validate params (`EngineError::BadParam`). Fits the model's measured strengths. Never on the hive path.
2. **Suggest and name (text model).** Better output file names from file content, ordering suggestions for the hive. Optional, off by default.
3. **OCR to text (VL-2B, about 1.3 GB with mmproj Q8_0; the vendor scores used an F16 mmproj of 379 MB, so re-measure with Q8_0).** New action such as `image.to-text` and `pdf.to-text`. Strong English document score; **Arabic untested**. Must be benchmarked against the existing alternatives (Tesseract, Ghostscript text extraction for born-digital PDFs) before any promise.
4. **Speech to text (Omni-2B).** `audio.to-text`, `video.to-subtitles`. Measured in 8c: Whisper small beats it on Arabic. Original note: compare against Whisper class engines before choosing. Audio-2B GGUF is not ready.
5. **Classification of files (d1).** Possible for "what kind of document is this", but the runtime route is unproven. Park it.

Out of scope, with the reason: translation (tested weak, not advertised), summarising long documents (weak long-context retrieval), anything that needs facts from memory.

## 4. How Kawaro would run it

Principle: a model is just another **engine** behind the existing trait (`crates/core/src/engine.rs`: `id`, `available`, `capabilities`, `estimate`, `run`). Heavy work stays a separate child process (S-030 process tree), so the GPLv3 code and the LFM licence never combine in one binary.

- Runtime: a pinned llama.cpp build (b11960 or newer), CPU only as the baseline (GPU builds multiply the artifacts per OS and arch, so they come later). System llama.cpp is used when present and new enough (probe `--version`, same idea as `locate::find`). Otherwise the pinned build is downloaded, with a pinned hash per OS and arch.
- Two run modes. Batch OCR and speech use a **one-shot CLI child** (`llama-mtmd-cli` class), like FFmpeg: no port, no auth surface, no idle lifecycle. Natural language to action may use a **warm server** kept by the daemon with an idle timeout.
- Server rules if used: bind 127.0.0.1 only (Unix sockets are unreliable on Windows), a random `--api-key` per spawn so no other local process can use the port, web UI disabled, killed on cancel and on daemon exit (S-030 process tree).
- Never pass `-hf`. It makes llama.cpp download from Hugging Face by itself, which bypasses our hash pinning and the controlled network rule. Pass local paths only (`-m`, `--mmproj`).
- Prompt injection: text read from a document (OCR) never shares a prompt with the action selector. Constrained output (grammar or JSON schema) is mandatory for the selector.
- No in-process route: the `llama-cpp-2` crate and ONNX Runtime would break the rule that heavy engines run as separate processes and the separation from the LFM licence. Not considered further.
- Windows: the pinned binary and its DLLs ship in the download, and the downloader depends on `curl.exe`.
- `available()` is false until both runtime and weights are installed, so the actions stay hidden, exactly like a missing FFmpeg (`Registry::matching(files, engine_ok)`).
- New `MediaKind` values and output extensions (`txt`, `srt`) are needed for OCR and transcription (`crates/core/src/media.rs`, `crates/core/src/action.rs`).
- Memory: 1.1 GB weights plus KV. Tests run only inside the 6 GB systemd-run scope.

## 5. How it is downloaded and where it appears in settings

Facts about the repository today (checked by reading code, see the integration report in section 9):

- Download on demand is **not implemented**. `docs/engines.md:261` says "Kawaro never downloads an engine". `docs/adr/0005-windows.md:78` is softer ("decided later"), so only engines.md needs amending. The new ADR 0010 makes that decision for opt-in models.
- Reusable machinery exists in the updater (ADR 0006): `crates/core/src/update/fetch.rs` (system `curl`, https only, `.part` file), `flow.rs` (`fetch_and_check`: size check, sha256 check, atomic rename), `verify.rs` (minisign over a signed manifest, key in `packaging/keys/kawaro-update.pub`), `manifest.rs` (schema 1 with `assets[]` of kind, os, arch, name, url, size, sha256).
- There is no data directory yet in `crates/core/src/paths.rs`; only config and cache. Models are not a cache (the system may purge it), so add a data directory function in the same pure `*_with(os, env, known)` style: `$XDG_DATA_HOME/kawaro/models` on Linux, `%LOCALAPPDATA%\Kawaro\models` on Windows.

Proposed flow:

1. A signed **model catalog** (same minisign key and verification order as updates) lists each model: id, display name in Arabic and English, kind (runtime or weights), size, sha256, source URL, licence id, minimum runtime build. Sizes and hashes are pinned by us, never taken from Hugging Face at download time.
2. The user opens Settings, section **Models** (after Updates in `crates/ui/ui/settings.slint`, with a pure `crates/ui/src/models.rs` state module modelled on `updates.rs`). Each row: name, size, state (not installed, downloading with percent, ready, failed), a Download button and a Remove button, a switch to enable the related actions, the storage path, and a link to the licence text.
3. The daemon downloads, reports progress over IPC like S-091 (`Request` and `Response` variants), verifies, renames atomically. The window decides nothing and never takes focus. One notification at the end.
4. Nothing downloads without a click. No background fetch, no telemetry. The licence is shown before the first download.
5. Redirects: `fetch.rs` already allows https-only redirects (`--proto-redir =https --max-redirs 5 --max-filesize`) and `flow.rs` applies only `url_is_clean` to asset URLs; the feed prefix guards the manifest alone. So a Hugging Face to CDN redirect works today, and the pinned size and sha256 from the signed catalog are the guard.
6. Gaps in the reusable code, to build in the story: no resume (no `-C -`, a failure restarts from zero), and a fixed `PACKAGE_TIMEOUT` sized for update packages. Models of 1 GB or more need HTTP range resume, a progress-based timeout and a per-model `max_bytes`. Also check free disk space before starting and refuse with a clear message.

Owner direction (2026-10-29): the AI edition is offered in two places, the welcome window on first run and the Models section in settings. The offer is a short, designed card in the hive style: what it adds (talk to Kawaro, read text from images, turn speech into text), what it costs (size on disk, nothing leaves the machine), one licence line, and one Download button. One click starts the download in the daemon, so it continues in the background until complete even if the window is closed or the file manager is used meanwhile. Rules that keep this consistent with CLAUDE.md:

- Consent is that one click, with the licence line visible on the card. No pre-ticked box, no silent download.
- The offer shows once on the welcome window. After a dismiss it lives only in settings; no repeated prompts.
- Progress shows in the tray tooltip, one notification at the end, never a window that takes focus.
- Interrupted downloads resume from the `.part` file (HTTP range) and are re-verified by sha256 before the atomic rename. A failed verify deletes the file and reports once.
- The AI features appear in the hive only after the files are verified; before that, nothing changes.

Starter catalog proposal, smallest first: llama.cpp runtime build (pinned), LFM3-1.7B QAD-Q4_0 (about 1.05 GB), later VL-2B plus mmproj, later Omni-2B. Quantisation: the 1.7B and VL-2B GGUF repos have no plain Q4_0 file (V, reviewer read the file trees). They hold Q4_K_M, Q5_K_M, Q6_K, Q8_0, QAD-Q4_0, BF16 and F16; only the Omni repo has Q4_0. The upstream PR tested the Q4_0 and Q8_0 quant types, not these file names. Start the spike with QAD-Q4_0 (Liquid's own edge build, U whether it is what their benchmarks used) and Q8_0 as the reference, and add Q4_K_M only after our own check.

## 6. Licence analysis (needs a human or legal check)

LFM Open License v1.0 (V): commercial use is not licensed for entities at or above 10 million USD annual revenue; qualified non-profits are exempt for non-commercial use; redistribution of the work and derivatives is allowed with the licence text and notices; it terminates on breach.

- It is not an OSI licence and its revenue cap and termination clause are not GPLv3 terms. Section 1 of the licence says works that stay separable, or only link or bind by name to the interfaces, are not derivative works, which supports keeping the model a separate child process. Sections 2 and 4 cover distribution in any medium, and nothing addresses an app that triggers the download from the vendor host on the user's behalf. That is a grey area (U) and needs counsel. Also note section 3 (patent licence ends if the user sues over the work) and section 7 (no trademark rights, so the UI must not imply Liquid AI endorses Kawaro). Decision for the draft: never bundle, never mirror; the download goes to the vendor's own host and the user accepts the licence on the offer card.
- Kawaro users above the threshold would need their own commercial terms. The settings row must show the licence and the threshold in plain words.
- `docs/engines.md` requires a licence entry for each new component. Weights and the llama.cpp build (MIT, to be confirmed at the pinned release) each get one.

## 7. Risks

- Vendor claims are unreproduced; Arabic and OCR quality are unknown. Mitigation: a spike with our own Arabic fixtures before any story is approved.
- Upstream support is days old. Pin the runtime, run our own smoke test per quantisation.
- Cold start. Loading 1.1 GB takes time; the feature must never sit on the hive path or block the UI.
- Disk and RAM. Show sizes before download; refuse when free space is too low; set a memory ceiling so 8 GB machines do not swap.
- Licence drift. Pin the licence file hash in the catalog.
- Hallucinated choices. The model proposes, the registry disposes: unknown ids and out-of-range params are rejected by existing validation.

## 8. Proposed next steps (none started)

1. Spike in `spikes/lfm3/` (outside the workspace, throwaway), inside the 6 GB scope, `CARGO_BUILD_JOBS` 1: run LFM3-1.7B on a fixed set of 30 Arabic and English requests mapped to our real `actions/*.toml` ids, grammar-constrained; record accuracy, first-token time and peak RAM on the owner's CPU. Run VL-2B on five Arabic and five English document images against Tesseract. Result goes into this document.
2. ADR 0010: opt-in model downloads (reverses the "never downloads" line in `docs/engines.md` for opt-in models), catalog format, data directory, redirect handling, licence display.
3. Stories from S-093: model catalog and download in the daemon; Models section in settings; the llama.cpp engine adapter; `ai.suggest-action`; `image.to-text`.

## 8b. Spike results (2026-10-29)

Scripts, fixtures and raw results: `spikes/lfm3/` (`out/*.json`). Runtime: llama.cpp b11972, CPU, 4 threads under a 6 GB memory scope, on an i7-14650HX that was busy (load average 13 to 17), so speeds are pessimistic. Pass bars were written before the runs (`spikes/lfm3/README.md`). All figures below are measured here, not vendor numbers.

Natural language to action, LFM3-1.7B QAD-Q4_0, 30 requests (15 Arabic, 15 English, 3 with no right action):

| | first run (headline) | tuned on the same 30 | bar |
|---|---|---|---|
| action accuracy | 80% (ar 80, en 80) | 76.7% (ar 80, en 73) | 90% |
| rejected by registry rules | 18 of 30 | 4 of 30 | 5% |
| `none` cases answered `none` | 1 of 3 | 0 of 3 | 3 of 3 |
| cold first request | 18.7 s (2684 prompt tokens) | 7.7 s (1056) | 10 s |
| median request | 4.8 s | 4.1 s | 3 s |
| peak RSS | 2.08 GB | 2.08 GB | 2.5 GB |

The tuned column used a per action JSON schema and a compact catalog, tuned on the same items, so it is not an accuracy claim. Findings:

- LFM3 reasons by default and spends the whole token budget before any JSON (empty content, `reasoning_content` full). `enable_thinking: false` is mandatory for the selector.
- A free form `params` object invites invented keys (file names, copied catalog fields). A per action schema cuts registry rejections from 60% to 13%; the registry check stays necessary.
- Typical errors: pdf.to-images read as documents.to-pdf, rotate degrees and flip axis swapped, an action proposed for requests with no matching action. Arabic and English score alike.
- Verdict: **fails the accuracy and `none` bars**. The model may only propose, and the hive must show the proposal for confirmation; it can never run an action by itself. The Q8_0 reference was not measured (download pending), so how much is quantisation is open.

OCR, LFM3-VL-2B QAD-Q4_0 with the Q8_0 mmproj against Tesseract (`ara` for Arabic pages, `eng` for English), 10 synthetic renders, normalised character error rate:

| | Tesseract | VL-2B | bar |
|---|---|---|---|
| Arabic mean (5 pages) | 0.278 | 0.263 | VL at most 1.25 times Tesseract |
| English mean (5 pages) | 0.013 | 0.001 | VL not worse |
| slowest page | 0.3 s | 20.4 s | 60 s |
| peak RSS | small | 2.44 GB | 4 GB |

Per page, VL wins on clean diacritic text (0.11 against 0.55), tables (0.23 against 0.54) and noisy skewed pages (0.01 against 0.16), and loses badly on the Arabic two column page (0.79 against 0.09) and on small Arabic text (0.16 against 0.05). **Passes the bars**, but the pages are synthetic and cleaner than real scans, and ten pages cannot support a quality claim. Reading it: VL is a candidate for noisy or table-like pages, Tesseract stays the fast default, and a real-scan test (including real two column Arabic) comes before any story. Time per page is 10 to 20 s on 4 cores, so this is a background action only.

Runtime notes: the official b11972 Linux CPU build runs the model directly. It reports `version: 0.8.0-dev (build 11972)`, so a probe must read the build number. The cold start of a 1 GB model is about 1 to 2 s.

## 8c. Capability and resource survey (2026-10-29)

Same setup as 8b (llama.cpp b11972, CPU, i7-14650HX with 20 threads, the machine busy). Fixtures: `spikes/lfm3/fixtures/cases/` (48 text, 24 vision, 6 synthetic audio clips). Results: `spikes/lfm3/out/`. Thinking off, temperature 0, one run each, so differences of one case are noise.

### Resources (text model 1.7B QAD-Q4_0, file 1.0 GB)

| measure | value |
|---|---|
| prompt speed, 1 / 2 / 4 / 8 threads | 37 / 78 / 145 / 97 tokens/s |
| decode speed, 1 / 2 / 4 / 8 threads | 13 / 21 / 26 / 23 tokens/s |
| prompt speed at 256 / 1024 / 4096 tokens | 136 / 116 / 95 tokens/s |
| decode speed at depth 0 / 4096 / 16384 | 24 / 27 / 11 tokens/s |
| RSS at context 2K / 8K / 32K / 128K | 1.9 / 2.0 / 2.3 / 3.4 GB |
| load time | 1 to 2 s |

Reading: 4 threads is the sweet spot (8 is slower on this hybrid CPU, so cap at 4 or half the cores). Context is cheap in memory (the short convolution and window attention design), but decode halves by 16K tokens of context. A normal job (about 1K prompt tokens, 100 output tokens) takes 8 s of prompt plus 4 s of decode, so 10 to 15 s on 4 cores. The vision model runs at 2.3 to 2.6 GB peak.

### Text tasks (48 cases, 8 categories)

37 of 48 pass: tool call 6/6, rename 6/6, summarize 6/6, refuse or say unknown 6/6, classify 5/6, translate 4/6, extract 3/6, rewrite 1/6. Arabic 19/24, English 18/24, so no language gap. Failures are mostly the checker's strictness (extra JSON keys in 2 of 3 extract failures; a valid label with a different case) and one counting error; rewrite fails on length limits (it ignores "at most N words") and leaks its own reasoning tags once. Useful: structured extraction, tool calls, file names, short summaries, honest "I cannot know". Not useful: tight length control, translation as a product feature.

### Vision tasks (24 cases)

17 of 24 pass. English 11/12, **Arabic 6/12**. Strong: receipts, charts and UI questions in English, stamped and low contrast pages. Weak: Arabic names (misreads letters), Arabic handwriting (nonsense), rotated Arabic text, counting objects, an Arabic table cell, an Arabic chart label. Median 9.6 s per image. Conclusion: English document understanding is good for a 2B model; Arabic is OCR grade only on clean printed text (see 8b) and unreliable for names, handwriting and rotation.

### Speech to text (Omni-2B Q4_0 plus Q8_0 mmproj, 6 synthetic clips)

English (flite voice, 9 to 14 s): word error rate 0.00, 0.00 and 0.07, about 2 s per clip, so several times faster than real time on 4 cores; peak RSS 2.7 GB. Arabic (espeak-ng voice): unusable (character error 0.80, 0.87 and 3.6, with Latin script and a repetition loop), **but espeak Arabic is itself barely intelligible, so this says nothing about real Arabic speech**. Real speech is measured below.

#### Real speech against an independent engine (2026-10-29)

Clips come from open datasets, fetched by `spikes/lfm3/fetch_real_audio.py` (audio stays outside the repository; `fixtures/cases/audio-real-manifest.json` lists every clip): 12 Arabic read speech clips from google/fleurs ar_eg test (CC-BY-4.0, Egyptian speakers), 12 Syrian dialect podcast clips from Overtech/syrian-arabic-speech-dataset (the SYRA research licence; used locally for evaluation only, nothing redistributed), and 8 English clips from LibriSpeech via hf-internal-testing/librispeech_asr_dummy (CC-BY-4.0). The independent judge is Whisper through faster-whisper (int8, CPU, 4 threads, greedy decoding, language forced), not our own metric, scored with the same normalisation. Mean word error rate (character error in brackets):

| set | LFM3-Omni-2B Q4_0 | Whisper base (74M) | Whisper small (244M) |
|---|---|---|---|
| Arabic, read speech (12) | 0.60 (0.32) | 0.47 (0.14) | **0.24 (0.07)** |
| Arabic, Syrian dialect (12) | 0.74 (0.41) | 0.93 (0.56) | **0.39 (0.22)** |
| English, read speech (8) | **0.065 (0.04)** | 0.11 (0.05) | 0.10 (0.05) |
| seconds per clip (mean 10 s clips) | 2.8 to 3.9 | 0.7 to 1.0 | 2.0 to 2.2 |
| model files | 1.6 GB, 3.2 GB RSS | 0.15 GB | 0.48 GB |

Verdict: **Omni is not adopted for Arabic speech to text.** Whisper small is about twice as accurate on Arabic, smaller (a third of the size), and faster. Omni's only edge is English read speech (0.065 against 0.10), too small a gap on 8 clips to justify a 1.6 GB model. Omni also leaks Latin and Cyrillic characters into Arabic output and can loop on bad input, so a length cap and a script check would be needed. Both engines are weak on the dialect (0.39 at best), so dialect transcription needs a larger Whisper or a dialect model and its own measurement. Limits: 32 clips and 333 seconds in total, single speakers per clip, no noisy recordings, no long files; treat the ranking as solid and the exact numbers as indicative.

### What this means for Kawaro (best uses, ranked)

1. `image.to-text` for noisy or table pages, optional, background only (8b).
2. Structured extraction from text or OCR output into JSON (receipt, invoice, form fields), always with a per action schema and validation.
3. Smart file naming from content (6/6), a suggestion the user accepts.
4. Natural language to action as a confirmed proposal only (8b).
5. Speech to text stays on Whisper class engines (measured above); Omni is not worth its 1.6 GB.
6. Not worth building: translation, rewriting with limits, handwriting, counting.

Caveat for every row: one run, synthetic inputs, no independent replication.

## 9. Corrections found while researching

- `CLAUDE.md` mentions `core::probe`. No such module exists. The real equivalents are `crates/engines/src/locate.rs` and `kawaro_core::update::apply::probe_facts`.
- The `backlog/_TEMPLATE.md` is Arabic while real stories are English headed. New stories follow the English form of S-090.

## 10. Sources read

Liquid blog and collection: https://www.liquid.ai/blog/lfm3, https://huggingface.co/collections/LiquidAI/lfm3. Model cards: https://huggingface.co/LiquidAI/LFM3-1.7B, https://huggingface.co/LiquidAI/LFM3-VL-2B, https://huggingface.co/LiquidAI/LFM3-Audio-2B, https://huggingface.co/LiquidAI/LFM3-Omni-2B, https://huggingface.co/LiquidAI/d1-2B, https://huggingface.co/LiquidAI/d1-Omni-2B. Licence: https://huggingface.co/LiquidAI/LFM3-1.7B/blob/main/LICENSE. Runtime: https://github.com/ggml-org/llama.cpp/pull/31252. Docs: https://docs.liquid.ai/ (the llama.cpp page there still covers only LFM2 and LFM2.5). Third party: https://www.orcarouter.ai/blog/lfm3-family-release-tri-hybrid, https://www.mindstudio.ai/blog/liquid-ai-lfm3-family-release, https://huggingface.co/lmstudio-community/LFM3-1.7B-GGUF.

Not found: Reddit, Hacker News and llama.cpp issue threads; unsloth and bartowski GGUFs; independent CPU speed; any Arabic quality test; any independent OCR test; the blog text beyond the first part.
