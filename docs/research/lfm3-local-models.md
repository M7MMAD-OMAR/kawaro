# Research: Liquid AI LFM3 as a local model engine for Kawaro

Status: reviewed once by an independent agent, corrections applied (2026-10-29). Not a decision. A decision needs ADR 0010 (see section 8).

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
4. **Speech to text (Omni-2B).** `audio.to-text`, `video.to-subtitles`. Arabic ASR unconfirmed; compare against Whisper class engines before choosing. Audio-2B GGUF is not ready.
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

## 9. Corrections found while researching

- `CLAUDE.md` mentions `core::probe`. No such module exists. The real equivalents are `crates/engines/src/locate.rs` and `kawaro_core::update::apply::probe_facts`.
- The `backlog/_TEMPLATE.md` is Arabic while real stories are English headed. New stories follow the English form of S-090.

## 10. Sources read

Liquid blog and collection: https://www.liquid.ai/blog/lfm3, https://huggingface.co/collections/LiquidAI/lfm3. Model cards: https://huggingface.co/LiquidAI/LFM3-1.7B, https://huggingface.co/LiquidAI/LFM3-VL-2B, https://huggingface.co/LiquidAI/LFM3-Audio-2B, https://huggingface.co/LiquidAI/LFM3-Omni-2B, https://huggingface.co/LiquidAI/d1-2B, https://huggingface.co/LiquidAI/d1-Omni-2B. Licence: https://huggingface.co/LiquidAI/LFM3-1.7B/blob/main/LICENSE. Runtime: https://github.com/ggml-org/llama.cpp/pull/31252. Docs: https://docs.liquid.ai/ (the llama.cpp page there still covers only LFM2 and LFM2.5). Third party: https://www.orcarouter.ai/blog/lfm3-family-release-tri-hybrid, https://www.mindstudio.ai/blog/liquid-ai-lfm3-family-release, https://huggingface.co/lmstudio-community/LFM3-1.7B-GGUF.

Not found: Reddit, Hacker News and llama.cpp issue threads; unsloth and bartowski GGUFs; independent CPU speed; any Arabic quality test; any independent OCR test; the blog text beyond the first part.
