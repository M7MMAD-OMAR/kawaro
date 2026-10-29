# ADR 0010: Opt-in model downloads, and whether LFM3 is the first model

Status: proposed (part A ready for acceptance; part B decided from the spike, see below)
Date: 2026-10-29

## Context

`docs/engines.md` says Kawaro never downloads an engine. That held while every engine was a system program. Local language and vision models change the picture: they are useful, they are 1 GB or more, and no distribution packages them. Research is in `docs/research/lfm3-local-models.md`; measurements are in `spikes/lfm3/out/`.

Constraints that stay: nothing leaves the machine except the download the user asked for; the base install stays light; heavy work runs as a separate child process; the original file is never modified; no telemetry.

## Part A: the download mechanism (independent of any model)

### Decision

1. Downloading is allowed only for **opt-in components** (a runtime build or model weights), only after one explicit click on a card that shows what is added, the size on disk, the licence line and the words "nothing leaves your machine". No pre-ticked box, no background fetch. Base engines (FFmpeg, LibreOffice, Ghostscript) stay system programs and are never downloaded.
2. A **signed catalog** lists each component: id, Arabic and English name, kind, size, sha256, source URL, licence id and licence file hash, minimum runtime build. Same minisign key and verification order as updates (ADR 0006). Sizes and hashes are ours; nothing is trusted from the remote host at download time.
3. The daemon downloads, so it continues after the window closes. Progress shows in the tray tooltip, one notification at the end, never a window that takes focus.
4. Interrupted downloads resume from the `.part` file with HTTP range, verify sha256, then rename atomically. A failed verify deletes the file and reports once. Free disk space is checked first. The timeout is progress based, not a fixed total.
5. Redirects from the vendor host to its CDN are allowed (https only, bounded count); the pinned size and sha256 are the guard.
6. Files go to a new data directory (`$XDG_DATA_HOME/kawaro/models`, `%LOCALAPPDATA%\Kawaro\models`), not the cache. Settings has a Models section with Download, Remove and the path. The offer appears once in the welcome window, then only in settings.
7. The runtime is invoked with local paths only. llama.cpp's `-hf` flag is never used because it fetches by itself and bypasses the pinned hash.
8. Features that need a component are hidden until its files are installed and verified.
9. `docs/engines.md` is amended: "Kawaro never downloads a base engine; opt-in components are the only exception (ADR 0010)".

### Consequences

- The updater code is reused but needs range resume, a progress based timeout and a per component size ceiling.
- Weights are neither bundled nor mirrored by us. Whether an app may trigger the vendor download on the user's behalf under the LFM Open License is open and needs counsel before any release (research, section 6).

## Part B: LFM3 as the first model

Measured in `spikes/lfm3/` against bars written before the runs (research doc, section 8b).

- **Action selector (text 1.7B): not adopted for now.** 80% accuracy first run against a 90% bar, and `none` handled in 1 of 3 cases. It may later return as a proposal that the user confirms in the hive, never as an automatic choice, once a larger or better quantised model (Q8_0 was not measured) is checked on the same fixtures with thinking disabled, a per action schema and registry validation.
- **OCR (VL-2B plus mmproj): not adopted.** It passed on 10 clean synthetic renders but lost on 32 real Arabic pages (CER 0.49 against Tesseract 0.18 on book pages, 0.64 against 0.35 on noisy scans, 2.4 against 0.01 on line images, with loops and 30 to 130 s per page). Research section 8c. Tesseract stays the OCR engine; stronger open OCR engines are measured separately.
- **Speech to text (Omni-2B): not adopted.** Measured on 32 real clips against Whisper (research, section 8c): Whisper small is about twice as accurate on Arabic (0.24 against 0.60 on read speech, 0.39 against 0.74 on Syrian dialect), smaller and faster; Omni wins only on English read speech (0.065 against 0.10). The speech engine, if built, is a Whisper class engine behind the same opt-in mechanism.
- Therefore no LFM3 model enters the starter catalog. The first catalog entry is a Whisper class speech engine. The download mechanism is validated by `spikes/model-download/`.
- Runtime rules: version probe by build number; `enable_thinking: false` for any text use; local paths only.
- The licence question (research, section 6) must be answered before any public release of the download.
