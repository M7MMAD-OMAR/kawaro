# Kawaro next features: product specification

Status: source implementation authorized by explicit owner GO on 2026-11-15; runtime acceptance deferred.
Date: 2026-11-15.
Baseline: main 43ca8f5. Public release 0.5.0; 0.5.1 is a source candidate.
Authority: owner request for complete research and planning, with heavy checks deferred and other platforms planned after the features stabilize.

## 1. Goal and success

Make the existing converter easier to trust, repeat and recover: see a transformed result, reuse a chosen setup, understand every batch outcome, discover a missing tool, and extract text locally. Preserve the quick hive path and the untouched original.

Success means each feature has an explicit observable outcome and failure behavior. A code implementation, a local acceptance result and a published release are separate states. This planning pass establishes neither runtime acceptance nor release readiness.

## 2. Current baseline and corrections

- Typed action options, a customization panel, CLI params, local opt-in MCP, image crop/fit/fill/target-size compression, PDF page operations, PDF text extraction and qpdf actions already exist. Do not recreate them from the older November 4 research.
- The current picture is a source draft. It does not show transformed output. The source draft and the image engine differ on EXIF orientation; the dimension readout handles scale/max-side but not every crop/resize-box parameter.
- User-saved reusable presets do not exist. Action-defined preset machinery and the unused remember declaration are different concepts.
- Jobs already run through a queue, expose progress and completion, and can be cancelled. The proposed results feature extends this instead of adding another runner.
- Missing-action explanations and engine discovery already exist in parts of the product. The new feature must extend those paths rather than show duplicate installation prompts.
- Local OCR is a proposed capability, distinct from existing PDF text extraction. Prior Arabic experiments rejected LFM3-VL as the OCR engine; the recorded deterministic baseline is Tesseract. Those measurements are historical, not a new quality guarantee.
- S-132/S-133/S-134 have source implementations despite stale in-progress story headings. Their exact acceptance evidence must be reconciled before any heading is marked done.

## 3. Working policy for this programme

1. Owner GO now authorizes full source implementation, following completed research/planning. No build, test, benchmark, compositor, model inference, installer or CI runs while the existing owner deferral is in force.
2. Implementation uses the owner-selected Pi/MiniMax-M3.1-Flash agent with small explicit work packets. The parent owns plans, decisions and review. No factory is implied.
3. During development, source review and focused acceptance designs are mandatory. Executable checks are deferred under the current owner instruction. A planned test must not be reported as run.
4. When executable validation is resumed, use the touched feature's cases first. Do not rebuild all platforms after every small edit. Complete release gates belong to a separate release candidate, not every documentation or implementation step.
5. No implementation may declare itself production accepted solely from static review. A source push may remain explicitly unverified and use the existing owner-approved CI deferral. No release tag follows from this planning authorization.
6. Maintain one main checkout, preserve unrelated work, stage exact paths, and never force push. Keep Linux window controls hidden and preserve application navigation.

## 4. Priorities and dependency order

| Order | Deliverable | Why now | Dependency |
| --- | --- | --- | --- |
| 0 | Reconcile story statuses and freeze this specification | Avoid duplicate implementation and false completion | Existing source and evidence |
| 1 | S-142, transformed image preview | The visible result should agree with the chosen operation | Existing S-117/S-118/S-119/S-139; shared image path |
| 2 | S-143, named user presets | Reuse a setup after preview behavior is defined | Existing options schema; S-142 preview integration |
| 3 | S-144, batch outcomes and safe retry | Make partial failure actionable | Existing queue and daemon job state |
| 4 | S-145, unavailable-action explanations | Help users obtain the operation they expected | Existing registry, engine lookup and doctor |
| 5 | S-146, optional image-to-text OCR | Add a useful deterministic local capability | S-145 availability contract; existing process/output contract |
| 6 | S-147, searchable-PDF feasibility and design | Decide dependencies, text-layer behavior and licensing before coding | Image OCR contract plus PDF research |
| Later | Searchable-PDF implementation | Only after the feasibility decision is explicit | S-147 must return a supported route |
| Later | Platform qualification and native adapters | Test stable feature contracts against actual desktop differences | Core feature acceptance evidence |
| Separate | S-141 release publication | Ship the accepted source when release validation is resumed | Exact candidate gates, packaging and channel evidence |

These are dependency priorities, not calendar estimates. Batch results and availability have independent core contracts, but Rust builds and feature integrations remain serialized. No unmeasured deadline, speed or memory saving is promised.

S-138 stays a separate existing draft. Planning cannot resolve its missing distant-drop baseline or invent the owner's release-before-drop semantics. Do not put new feature implementation behind an unapproved reinterpretation of that behavior.

## 5. S-142: transformed image preview

### User outcome

Changing supported image options shows the intended result without creating a converted file. The UI distinguishes a responsive transformed draft from a deliberately requested full encoded preview.

### Scope

- Still images handled by the existing image engine. Exclude images-to-PDF, PDF rendering, video, animation editing and arbitrary external-engine execution.
- Reuse the engine's operation parsing, orientation, geometry, alpha flattening, encoding and metadata rules. No UI-specific second implementation of crop or resize math.
- Correct geometry readouts for crop and resize-box as well as scale/max-side.
- Keep the current customization stage, Back, Run and S-136 session ownership. No extra window, no automatic conversion and no focus steal.

### Two honest modes

1. Transformed draft: a bounded source decode with orientation and supported operations applied to a display proxy. Geometry is calculated from the oriented original dimensions. Show no exact encoded size or compression-artifact claim. Qualify unsupported proxy operations instead of displaying a misleading substitute.
2. Encoded result: an explicit user request computes full-resolution bytes through the same final encode and metadata path as export, then decodes those bytes to a bounded display image. The displayed image is still scaled; the byte count is exact for that encoded artifact only. The mode is off by default and never starts on every keystroke.

An exact byte count is not restricted to JPEG: it is the final length of the actually encoded artifact for any supported encoder, including applicable metadata. It is not a promise that a later Run will have identical bytes if input, params, codec configuration or metadata changed. Run revalidates and follows the queue's safe output path.

### Initial scheduling policy, design targets only

- One preview worker, one running request and one replaceable pending request. No growing request queue.
- Debounce automatic draft requests by 180 ms. This is a UI policy, not a measured performance claim.
- Display buffer longest side at most 512 pixels; cache at most two buffers and a total of 8 MiB of display data. Do not retain full encoded artifacts in a global cache.
- Full encoded preview admission cap: 16,000,000 oriented source pixels and 64 MiB input file size for the first version. Oversize input keeps normal export available and explains why exact preview is unavailable. These are proposed product limits to review against future measurements.
- Admission checks and decoder limits are not a hard process RSS bound. Some codecs allocate internally; no strict memory or cancellation-latency guarantee is allowed without measurement or stronger process isolation.
- Cancellation is cooperative between stages. An in-flight codec may finish before cancellation is observed. Replacing/closing the panel invalidates all old publications immediately, even if computation finishes later.

### Observable acceptance, not run now

- An oriented phone photo has matching preview/export orientation; crop anchors, aspect, fit/fill/exact, grayscale, rotation and transparent-to-JPEG behavior agree with the engine contract.
- Compression quality can be judged only in encoded mode. Target-byte search never runs automatically while editing.
- A stale panel or source cannot publish pixels, labels or dimensions into a newer panel. No preview emits JobEvent or commits a file.
- Input errors, unsupported codecs and policy limits leave Run governed by the real action validation, with accurate explanations.

## 6. S-143: named user presets

### User outcome and scope

Save the current valid image-operation options under a name, apply them later to the same action, rename or delete them. Start with single-action image presets, not multi-step workflows or a new action language.

- Store separately in config-dir/presets.toml, schema version 1. Do not put preset data into Config or silently implement remember-last-values.
- Stable preset ID is distinct from editable display name and action ID. Initial saving supports built-in image actions only, excluding images-to-PDF; arbitrary custom actions and free-text string fields are not saved. Save allowlisted numeric/bool/enum/px-pair fields only, never paths, selections, input names, engine commands, secrets or permission grants.
- Plain one-click actions keep their existing defaults. Opening a form does not auto-apply the last user preset. Explicit application is required.
- Built-in action presets and named user presets are shown as different groups. Do not replace existing Reset semantics or add an overloaded fourth footer action; use a secondary in-panel preset control with existing Material components.
- Persist a deterministic fingerprint of option semantics and values, excluding translated labels. On fingerprint mismatch, revalidate every saved field against the current schema.
- An unknown field, changed type/range, missing action or unresolved clears conflict disables the entire saved preset with a reason. Never silently drop a value, clamp it, remap it to another field or run a partial preset.
- A compatible fingerprint change may be applied after complete revalidation and shown as compatible. Do not rewrite the store until the user saves.
- Unknown future store version opens read-only, preserving the file. Malformed data never resets ordinary config and never overwrites the broken store automatically.
- Use a single daemon writer. Save by same-directory temporary file, flush, then platform-safe replacement; detect external edits with a content revision. Reject an outdated save rather than overwrite concurrent changes.
- CLI/MCP preset management and import/export are later extensions. Per-run params still pass through the existing validator.

Acceptance: save/apply/delete survive daemon restart; duplicate names are rejected per action using trimmed Unicode-lowercase comparison; Arabic names remain text; applying a preset triggers exactly one updated preview and never submits a job; broken/stale data preserves the original store and the ordinary settings.

## 7. S-144: batch outcomes and safe retry

### User outcome

A quiet, user-opened results view shows what happened to a selection, links existing outputs, explains skips/failures and can retry eligible failures. Existing notifications remain concise and do not open a window automatically.

- Model task identity explicitly. Job IDs alone and a representative input path are not sufficient for grouped actions.
- Keep success, deliberate skip, failure, cancellation and interruption distinct. A PDF that was not made smaller is skipped, not failed and not retried by default.
- Record the immutable action/params snapshot, ordered task inputs and successful final output paths from production queue events. Do not reconstruct success from an output filename guess.
- First version retains at most 100 completed jobs or 1,000 task records, whichever limit is reached first. Session-only memory, no on-disk history and no restart/resume claim. Evict whole completed jobs, never active jobs or arbitrary individual rows from a retained detailed job. A single job larger than the task-record cap retains an explicit summary-only record with aggregate counts and no Retry; do not present an incomplete subset as a complete batch.
- Retry is a new explicit submission with a parent-job reference. For per-file actions, submit failed, non-cancelled task units only. For combined actions, retry the whole original ordered group only when its single task failed and the user sees that scope. Never turn a failed merge into a subset merge.
- Revalidate action, engine and params, rejecting changed engine/op/output-mode semantics; compare current input identity with captured identity. Changed, missing or non-regular inputs block that retry until the user creates a new explicit job. Path/size/mtime identity is best effort and is not a content-hash or no-TOCTOU guarantee.
- Preserve originals, collision-safe output naming, no-overwrite commits, cancellation and exactly-once session submission. Repeated Retry clicks cannot create duplicate child jobs while the first submission is pending.
- No automatic retry, persistent crash recovery or replay through MCP in this version. Paths remain local; no full paths in a new unsolicited notification.

Acceptance covers a mixed batch with success/failure/skip, a whole-group failure, cancel, duplicate clicks, changed input, removed action, output deleted after success and session-history eviction.

## 8. S-145: unavailable-action explanations

### User outcome

A user can understand why an expected action is missing and open the right next step without guessing or silently installing software.

- The active hive keeps only runnable actions. Offer a discoverable explanation surface rather than flooding the hive with disabled cells.
- Explain separately: missing executable, missing language data, unsupported input, invalid options, disabled policy and a non-applicable selection.
- Reuse existing registry filtering, zero-action messaging, doctor and engine discovery. Do not add a second catalog or interpret display labels as capability IDs.
- First version offers a diagnostic and curated setup/help instructions. It never invokes sudo, a package manager, shell fragments, a browser without user action, or a network download as a side effect of probing.
- System engines remain system-provided. Do not offer a qpdf download or silently turn proposed ADR 0010 into an accepted installer architecture.
- Runtime probes are bounded, off the UI loop and cached with explicit refresh after installation. A hung executable produces an unknown/unhealthy state, not an installed-and-working claim.
- Capability availability must agree across hive, CLI and MCP for the same registry and policy. Preserve current no-actions reasons and mixed-selection behavior.

Acceptance: a missing qpdf/Poppler tool has a truthful explanation; PATH changes refresh once; disabled MCP is not described as missing software; input mismatch never requests an irrelevant engine install; no discovery action spawns a job or installer.

## 9. S-146: optional local image OCR

### User outcome

Choose an installed language set and extract printed text from a still image into a new UTF-8 text file beside the source. Arabic and English are initial language selections; this is not a handwriting or translation feature.

- Tesseract child process, system executable and installed language data. Initial inputs are still JPEG, PNG and BMP; other codecs, TIFF containers and animated inputs are explicitly unavailable, not silently reduced to a frame. Normalize EXIF orientation and alpha through the shared deterministic image preparation into a private PNG before OCR. This is format/orientation handling, not an automatic contrast/deskew preprocessing pipeline. No LLM, Torch dependency, background model fetch or mandatory base-install expansion.
- Verify executable and requested traineddata languages before a job; missing ara/eng becomes S-145 capability guidance. Never guess that a language is installed from the executable alone.
- Use explicit language choice; do not claim automatic language detection. Do not add preprocessing by default: historical project experiments did not find a universal improvement.
- Single plain-text output per task keeps the existing engine/output contract. A later PDF OCR action is separate.
- Reuse managed child-process cancellation, private work-directory and queue temp-file commit mechanisms. Initial OCR policy: one OCR child at a time per daemon instance, 5-second language/version probe budget, 240-second OCR child budget per image, 16,000,000 source pixels, 64 MiB input and 8 MiB recognized-text output. These are proposed admission/time policies, not measured throughput or hard process-RSS promises. Pass arguments directly, never shell-join source paths or user language strings.
- Do not normalize Arabic spelling, digits, punctuation, diacritics or reading order in recognized content. Owner copy conventions apply to UI text, not silent mutation of a user's document.
- Empty recognition fails with the specific diagnostic No printed text recognized and produces no committed file. Do not invent confidence percentages or normalize content to make it look successful. This preserves the current serialized skip-reason contract; a new OCR skip variant is outside the first scope.
- Later quality validation must use a versioned real Arabic/English fixture set and ground truth, including mixed direction, columns, rotation and noisy scans, with CER and reading-order observations. No quality or throughput claim is made now.

Acceptance: missing language data prevents execution; explicit language reaches the process unchanged; cancellation leaves no committed output; Unicode and order are preserved as returned; an ordinary path with spaces/metacharacters remains one argument; source identity and bytes are unchanged.

## 10. S-147: searchable-PDF feasibility, no implementation yet

Produce a decision document comparing OCRmyPDF as a system child with any route using existing dependencies. Select on actual output correctness, dependency/licensing scope and the one-task/one-output contract. Do not implement a PDF text layer by combining guessed OCR bounding boxes.

Required decisions: mixed text/image pages, skip/redo behavior, page order/size/rotation, Arabic search and copy order, encrypted/signed/corrupt PDF refusal, digital-signature invalidation warnings, cancellation/temp cleanup, per-page timeouts and incomplete jobs, engine/language capability discovery, optional PDF/A conversion and visible text preservation.

Prefer a single new searchable PDF output. Sidecar text is a separate action or later multi-output contract, not an implicit second file. OCRmyPDF's sidecar contains OCR text only and is not an all-text extraction guarantee. Do not silently force OCR or flatten signed documents. A signed/encrypted input is refused in the first proposed scope unless a future explicit policy is approved.

Exit: a proposed architecture ADR, dependency/license inventory, executable argument design, future fixture matrix and a clear adopt/defer/reject recommendation. No version or package is pinned before compatibility review. Searchable-PDF implementation receives its own story only after this decision.

## 11. Other platforms and reliability follow-ups

Now: keep contracts in core, engine algorithms out of UI, paths as PathBuf, capability-based adapters and localized public messages. Avoid Linux commands in shared logic and do not assume rename, file identity, notification actions or process termination are portable.

Later, after feature source behavior and focused acceptance are stable:

1. Freeze the feature API and written invariants, list actual platform-sensitive boundaries.
2. Plan real GNOME/Nautilus and KDE/Dolphin qualification, including S-082 focus, Alt/pointer work and S-138 evidence without changing its unresolved semantics.
3. Plan Windows feature availability, path/Unicode behavior, process cancellation, focus, notifications and packaged engine detection. Existing Windows source remains supported; no new runtime acceptance is claimed.
4. Plan real X11 evidence separately. Keep macOS unsupported until a dedicated feasibility and native-integration plan exists.
5. Only then define the release/platform matrix, resource-limited builds and packaging gates. No cross-build or VM starts during this planning pass.

This ordering reduces churn. It cannot guarantee the absence of defects; later native evidence is required before platform claims.

## 12. Deliberately outside this programme

No cloud uploads, accounts, monetization promises, model-based action selection, speech transcription, background removal, upscaling, new codec families, multi-step workflow editor, persistent document history, automatic retry, universal desktop Alt claim or toolkit upgrade. Existing research remains available for future proposals.

## 13. Acceptance and traceability

The implementation plan maps each invariant above to exact files, proposed APIs, failure behavior and a deferred focused case. Planning completeness is reviewed against all five feature proposals plus PDF feasibility, current release/S-138 boundaries and the later-platform phase. Unknowns stay explicit research gates rather than being disguised as implementation details.
