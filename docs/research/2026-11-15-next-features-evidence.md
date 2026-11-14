# Next feature research: current source and technical evidence

Date: 2026-11-15. Source baseline: main 43ca8f5.
Scope: transformed image preview, named user presets, batch outcomes/retry, unavailable-action explanations, local OCR, and the later platform phase.
Method: source/graph/document reads and official-source search results. No build, test, benchmark, compositor, OCR run, package install or model download. No pricing, adoption or performance forecast.

## 1. Research conclusions

1. Improve the existing conversion path before adding another engine family. Shared transform semantics and result provenance are prerequisites for a truthful preview.
2. Preserve quick one-click defaults. Saving a preset is explicit and is different from automatically remembering the last form.
3. Expose the existing queue's outcomes rather than build a second queue or derive state from notification text.
4. Availability is a capability question, not simply whether an executable exists. A missing OCR language pack and an unsupported input need different explanations.
5. Start OCR with the deterministic Tesseract system program already favored by historical project measurements. Searchable PDF needs a separate feasibility decision; it is not just another output suffix.
6. Stable core contracts come first. Native focus, notifications, paths and child-process behavior must be qualified on other systems later; portable type design alone cannot certify them.

## 2. Verified source facts: preview

| Evidence | Current behavior | Planning implication |
| --- | --- | --- |
| crates/ui/src/stage_two.rs, render around line 590 | Decodes a source draft and masks it; params do not transform that picture | Do not rename it result preview before sharing the engine path |
| crates/ui/src/stage_two.rs, dimension_target around line 373 | Reads scale/max_side for the readout | Extend the shared geometry plan for crop/resize-box |
| crates/ui/src/preview_worker.rs, jpeg_draft/other_draft | Draft decode omits the orientation step applied by the engine | Orientation parity is a correctness requirement |
| crates/engines/src/image.rs, run_single lines 468-530, load around 625 | Parses/loads/transforms/encodes before writing | Extract reusable preparation before the write boundary |
| crates/engines/src/image/metadata.rs, extract/inject | JPEG metadata handling changes final bytes after initial encoding | Exact size must count final artifact bytes, metadata included |
| crates/ui/src/lib.rs, dispatch_stage_update and panel_generation | Session/panel stamps reject old deliveries | Preview needs these stamps plus request generation and source identity |
| crates/ui/src/preview_worker.rs, Generation | A state machine exists but is not the production scheduler | Choose one production scheduler, do not run two competing counters |
| crates/core/src/engine.rs, Estimate | An estimated memory value is not a resource limit | Admission limits and measured release criteria are separate |

The graph query for run_single found the production image Engine::run caller. A later extraction still needs direct sibling review of image/encode.rs, image/metadata.rs, source geometry, UI/daemon provider wiring and MCP preview behavior. Graph edges do not imply every behavioral sibling has been reviewed.

## 3. Verified source facts: presets

- OptionsSpec, Field and Preset in crates/core/src/options.rs define schema validation, field IDs, parameter keys and clears rules. resolve and resolve_params intentionally use different ambiguity precedence, as ADR 0012 records.
- StageTwoState stores explicit parameter keys; action-defined preset values use field IDs. Persistence must choose one representation and convert once.
- The current actions declare no action.options.preset entries, although chip support exists. Do not describe these as a populated library of ready-made presets.
- remember is declared but not consumed in production. It is not an existing saved-preset store.
- Config in crates/core/src/config.rs has manual PartialEq and whole-file fallback behavior. A separate presets file avoids resetting ordinary settings because one saved preset is invalid.
- The daemon reload/watch path is in crates/daemon/src/ui.rs, state.rs and watch.rs. A presets file needs its own revision and watch integration.
- The UI production crate depends on core, not engines. Keep that direction; the daemon is the composition root that already depends on both.

## 3a. Verified source facts: batch results and capability gaps

- core/queue.rs JobEvent includes Started, Progress, TaskDone, TaskSkipped, TaskFailed and Finished. TaskDone has final output and optional detail but no task identity. TaskFailed carries only a representative input, error, elapsed and cancellation flag.
- core/queue_impl.rs plan (lines 52-78) groups PerFile inputs independently and Combined as one ordered selection. A merge and images-to-PDF are combined operations. Replanning a subset changes their meaning.
- daemon/state.rs JobInfo stores aggregate counts and first output/error/skip, not a per-task ledger. daemon/jobs.rs pump removes the record on Finished. A retained session results view is genuinely new functionality.
- Current production completion notifications are per completed task, with cancellation/no-start suppression, as daemon/jobs.rs task_notice records. The planned ledger must not silently introduce a second notification stream or change this behavior as a side effect.
- Tasks cancelled before starting may emit no terminal task event and are not counted in existing aggregate task counters. The new manifest must let the local ledger mark those rows cancelled at a cancelled Finished event without rewriting old counter semantics.
- core/ipc.rs Event::from drops elapsed/detail/cancelled on individual events. A new internal task-manifest field must be deliberately ignored by this existing wire mirror when no new remote result API is in scope. Never add an unnegotiated event variant that old clients cannot decode.
- core/unsupported.rs HINTS/tool_name cover FFmpeg, LibreOffice and Ghostscript but not Poppler/qpdf. Missing tools in the latter routes can therefore be classified as not supported rather than missing. This is a concrete source gap, not a hypothetical request for another installer.
- core/action_impl.rs shown/matching hide unavailable-engine actions. daemon/plan.rs already provides zero-action help; CLI doctor and MCP availability also exist. Extend the common composition/reason paths and keep existing help.
- engines/locate.rs Program includes Pdftotext/Qpdf; docs/engines.md lookup table lags that set. OCR has no current action, adapter or language-pack probe.
- engines/process.rs ProcessTree, core Task/TaskOutcome and the output temp-finalization helpers are reusable for a child OCR engine. Existing encrypted-PDF refusal is not permission to strip signatures or force OCR.
- SkipReason is serialized through IPC and currently has only AlreadyOptimized. Adding an OCR skip enum variant would make old wire clients vulnerable to decode failure; the first image OCR packet uses a specific empty-recognition failure instead. A new typed skip needs a separate versioned compatibility design.

## 4. Source-status reconciliation

S-132/S-133/S-134 headings still say in progress. Their corresponding source is present: target-byte compression in image.rs; TaskOutcome::Skipped and the PDF already-optimized result; MCP audit_path configuration. This establishes source presence, not that every acceptance case passed or a release was published.

Historical research files dated November 4 contain earlier baseline statements such as no per-run options or no crop. They are historical inputs and must not override current code. Preserve their original context and add a current-baseline pointer rather than silently rewrite their historical conclusions.

## 5. Official technical sources

Only primary documentation supports decisions below. Retrieved through search on 2026-11-15; examples describe upstream behavior, not commands run in this repository. Existing dependency versions remain pinned until a separate change is justified.

### Image decoding and display proxy

[image thumbnail](https://docs.rs/image/latest/image/imageops/fn.thumbnail.html) documents a fast downsampling algorithm and warns about aliasing and color-space assumptions. A small display proxy therefore cannot promise exact compression artifacts or encoded size. Use actual encoded bytes in the explicit mode when judging codec output.

[image decoder limits](https://docs.rs/image/latest/src/image/io/limits.rs.html) distinguishes strict dimension limits from best-effort allocation limits. The allocation budget is not a universal hard memory ceiling. Product admission limits and no-unbounded-queue policy reduce risk but still need later measurement.

### File persistence

[Rust filesystem guidance](https://doc.rust-lang.org/stable/std/fs/) recommends atomic operations rather than existence-check races. [OpenOptions::create_new](https://doc.rust-lang.org/std/fs/struct.OpenOptions.html) gives an exclusive atomic create primitive. Use the existing output-commit helpers for conversion artifacts. Preset replacement is a separate store operation with a single writer, a same-directory temp file and a revision conflict check; it must not copy the conversion no-overwrite policy blindly.

The search also surfaced rename_noreplace in beta documentation, explicitly marked experimental. It is not an adopted API or a reason to upgrade the repository toolchain.

### Notification capability differences

[Desktop Notifications D-Bus protocol](https://specifications.freedesktop.org/notification/latest/ar01s09.html) defines capability discovery and optional server features. The results surface must remain reachable from the tray even where notification actions/persistence are unavailable. Do not use notification dismissal or server persistence as authoritative job state.

### OCR language data and searchable PDF

[Tesseract installation](https://tesseract-ocr.github.io/tessdoc/Installation) names language packages including ara and eng. [Tesseract best data](https://tesseract-ocr.github.io/tessdoc/Data-Files-in-tessdata_best.html) describes a speed/accuracy tradeoff, not a guarantee for the owner's documents. The first OCR feature detects installed language data; it does not fetch larger weights automatically.

[OCRmyPDF cookbook](https://ocrmypdf.readthedocs.io/en/latest/cookbook.html) requires the requested language data and explains that a sidecar includes OCR text, omitting pre-existing text and pages not processed. Do not label that sidecar complete PDF text extraction. Existing Poppler extraction is the separate all-text route, subject to document limitations.

[OCRmyPDF installation](https://ocrmypdf.readthedocs.io/en/stable/installation.html) treats system dependencies explicitly. A Python package alone is not a complete deployment plan. PDF OCR adoption must inventory the executable, transitive dependencies, license obligations and runtime behavior before implementation.

### OCR licensing, output and signed-document constraints

The upstream [Tesseract license](https://github.com/tesseract-ocr/tesseract/blob/main/LICENSE) is Apache-2.0. Its [main repository](https://github.com/tesseract-ocr/tesseract) separately identifies Leptonica and warns that dependencies can have other licenses. Record tool and language-data provenance in the later adapter packet; this is not a complete distribution-license audit.

[Tesseract command-line documentation](https://tesseract-ocr.github.io/tessdoc/Command-Line-Usage.html) documents explicit language arguments. Its [FAQ](https://tesseract-ocr.github.io/tessdoc/FAQ) describes UTF-8 plain-text output. Treat returned document characters as content, not as product UI to normalize.

The official [OCRmyPDF signed-document behavior](https://ocrmypdf.readthedocs.io/en/stable/cookbook.html) refuses signed-PDF modification by default and explains that overriding it invalidates signatures. The planned first PDF scope keeps refusal; it does not offer an automatic override. Current [installation documentation](https://ocrmypdf.readthedocs.io/en/stable/installation.html) also reports changing renderer/dependency choices, including optional Ghostscript in version17. Do not design a new adapter around a historical fixed dependency list or an unreviewed latest-version pin.

## 6. Existing local OCR evidence

The project's docs/research/lfm3-local-models.md section 8c and ADR 0010 record real Arabic-page experiments, the rejection of LFM3-VL for OCR and the absence of a universal benefit from the tried preprocessing variants. These are historical experiments with stated sample limits, not independently rerun today. Reuse the decision; do not reuse exact timings as a new performance promise or redistribute restricted evaluation material.

## 7. Parent review corrections to research suggestions

- A worker-shared provider must use Arc with a Send + Sync trait boundary, not Rc annotated Send + Sync.
- Exact output length is possible for any actually encoded final artifact, not only JPEG. JPEG metadata is one reason an intermediate byte count can be wrong.
- Keeping work off the event loop does not by itself preserve a 150 ms responsiveness budget: CPU contention and scheduling still matter. No new timing acceptance is claimed.
- A generation stamp rejects publication; it does not preempt a codec call. Cooperative cancellation and hard process termination are different capabilities.
- A new user preset should not crowd the already reviewed fixed action footer. Keep secondary management inside the panel's existing component/layout rules.

## 8. Evidence still required later

A versioned image parity corpus; codec allocation/latency measurement on the target machine; schema-change and external-edit preset fixtures; mixed/grouped batch failure cases; engine/language capability cases; real Arabic/English OCR ground truth; PDF text-layer search/copy ordering and signed/encrypted input policy; actual desktop qualification after feature stabilization.

All are future acceptance work, not executed checks. Detailed cases and execution deferral are in docs/superpowers/plans/2026-11-15-next-features.md. The product decisions and proposed limits are in docs/specs/2026-11-15-next-features.md.
