# Next Features Implementation Plan

> **For agentic workers:** Implement this plan task-by-task using available execution tools. Steps use checkbox (- [ ]) syntax for tracking.

**Goal:** Add truthful transformed preview, named presets, actionable batch outcomes, capability explanations and optional image OCR, then decide searchable-PDF architecture before any PDF OCR implementation.

**Architecture:** Keep validation and platform-free records in core, image work and subprocess adapters in engines, rendering in UI, and composition/state ownership in daemon. Reuse current queue/output safety and session generations. Introduce proposed interfaces explicitly; do not change the Engine trait to make every engine implement image preview.

**Tech Stack:** Existing Rust workspace, Slint Material components, TOML/serde, existing sha2, existing image codecs, system Tesseract. No new runtime package is adopted by this plan.

**Spec:** docs/specs/2026-11-15-next-features.md.

**Evidence:** docs/research/2026-11-15-next-features-evidence.md. Source baseline 43ca8f5. Proposed APIs below do not exist yet.

## Global constraints

- Owner GO of 2026-11-15 authorizes full source implementation. Existing prohibition on executable checks, screenshot capture, builds, installers, model inference, factory and CI remains. Test source may be prepared but not executed.
- Selected implementation agent: pi/minimax/MiniMax-M3.1-Flash. Parent owns design decisions and final review.
- Heavy checks remain deferred. All test names/cases below are future acceptance designs, never execution instructions for this pass.
- Keep existing defaults, original-file preservation, collision-safe finalization, local processing and S-136 session ownership.
- Do not add engines to UI dependencies. Core preview records contain no Slint or image crate types.
- Do not duplicate OptionsSpec parsing/validation or image transformation algorithms.
- Core feature behavior first. Native desktop/platform work, cross-builds and packaging follow stable feature contracts.
- Public 0.5.0 and candidate source 0.5.1 remain separate. No version bump, release tag, public support claim or benchmark claim from this plan.
- Main only; exact-path commits; preserve unrelated changes. Any source push under continued CI deferral carries its explicit unverified status.
- English repository documentation, Arabic/English future user strings, Western digits, no long dashes. Arabic runtime copy receives the installed T1-T16 review before acceptance.

## 0. Status hygiene and implementation packets

**Files:** docs/PLAN.md, CLAUDE.md, backlog/S-132-image-compress-to-target.md, backlog/S-133-pdf-compress-no-duplicate.md, backlog/S-134-mcp-audit-path.md, backlog/S-139-correct-customization-ui.md, backlog/S-140-reliable-alt-escape-cancel.md, backlog/S-141-publish-0.5.1.md.

**Consumes:** Existing code and dated evidence. **Produces:** Accurate source/acceptance/publication status, no product change.

- [x] Reconciled S-132/S-133/S-134 and source-publication wording in S-139/S-140/S-141: current source presence is separated from dated acceptance logs and pending release verification. No done label was inferred from symbol presence.
- [x] Added current-baseline pointers to the four older product/image/agent/PDF research documents, preserving original dated content.
- [x] Recorded executable-check deferral and the later-platform sequence in PLAN/CLAUDE and this specification. S-138 remains draft with unresolved semantics.
- [ ] Before each later implementation packet, read only this specification, the relevant task, the existing story and the exact named source spans. Refresh graph callers for changed symbols, including platform siblings.
- [ ] A packet hands off: allowed paths, current signatures, proposed signatures, invariants, concrete future cases, commands prohibited now, and a concise source diff. The implementer does not edit tester-owned assertions or broaden scope.
- [ ] Parent reviews the packet result. If executable checks remain deferred, call the result implemented but unverified, never accepted or released.

No test is necessary for a documentation-only status correction. Review exact source/evidence references and Git diff only.

## 1. S-142: transformed image preview

### Files and boundaries

Create:
- crates/core/src/image_preview.rs: platform-free preview request/result/provider types.
- crates/engines/src/image/prepare.rs: shared in-memory operation pipeline, extracted from existing image path without a second implementation.
- crates/engines/tests/image_preview.rs: future production-path parity cases.
- crates/ui/tests/result_preview.rs: future scheduling/stamp/UI cases.

Modify:
- crates/core/src/lib.rs: export the new platform-free module.
- crates/engines/src/image.rs: run_single, load, Op parsing/application and metadata preparation boundary.
- crates/engines/src/image/encode.rs and metadata.rs: reuse existing final encoder/injection helpers, minimal visibility change within engines.
- crates/core/src/image_geom.rs: shared result dimensions for existing modes, no duplicated proxy geometry.
- crates/ui/src/stage_two.rs: current render/readout and Preview state around lines 271/373/590/607.
- crates/ui/src/preview_worker.rs: one production bounded scheduler and oriented draft decode.
- crates/ui/src/lib.rs: Inner::open_stage, end_stage, teardown, dispatch_stage_update stamps.
- crates/daemon/src/ui.rs: wire_options composition around line 225 and StageTwoState construction around 250.
- crates/ui/ui/hive.slint and relevant localization: preview mode, unavailable/stale status and explicit encode control.
- docs/adr/0016-image-preview-and-presets.md: proposed architecture decisions before extraction.

### Proposed interfaces

All types below live in core::image_preview. Existing ActionDef and CancelToken are imported; PathBuf and Arc remain ordinary standard-library types.

~~~rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImagePreviewMode { TransformedDraft, EncodedResult }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreviewStamp { pub session: u64, pub panel: u64, pub request: u64 }
#[derive(Clone, Debug)]
pub struct ImagePreviewRequest {
    pub stamp: PreviewStamp,
    pub input: std::path::PathBuf,
    pub resolved_action: crate::ActionDef,
    pub mode: ImagePreviewMode,
}
#[derive(Clone, Debug)]
pub struct DisplayPixels { pub width: u32, pub height: u32, pub rgba8: Vec<u8> }
#[derive(Clone, Debug)]
pub struct ImagePreviewArtifact {
    pub stamp: PreviewStamp,
    pub pixels: DisplayPixels,
    pub output_width: u32,
    pub output_height: u32,
    pub output_format: String,
    pub final_encoded_bytes: Option<u64>,
    pub mode: ImagePreviewMode,
}
#[derive(Debug)]
pub enum ImagePreviewError {
    Cancelled, SourceChanged, PolicyLimit, Unsupported(String), Failed(String),
}
pub trait ImagePreviewProvider: Send + Sync {
    fn preview(&self, request: &ImagePreviewRequest, cancel: &crate::CancelToken)
        -> Result<ImagePreviewArtifact, ImagePreviewError>;
}
pub type SharedPreviewProvider = std::sync::Arc<dyn ImagePreviewProvider>;
~~~

The engines crate implements ImagePreviewProvider for ImageEngine. Daemon injects SharedPreviewProvider into the customization construction path. UI requests values through this provider from its worker, never from the event loop. Do not use Rc for a worker-shared provider. The resolved action is constructed through existing core validation before scheduling; no raw shell command or output path arrives from UI.

### Implementation sequence

- [x] Parent reviewed and accepted ADR 0016 for source implementation under the explicit owner GO; executable evidence remains deferred.
- [ ] Record run_single behavior and the output-write boundary before extraction. Keep the parent method's signature and all unrelated operation behavior stable.
- [ ] Introduce an internal PreparedImage result with transformed pixels, output format, oriented source dimensions, transformed dimensions, metadata segments and target-size-search information. Its constructor takes the existing parsed operation, decoded image and resolved params. Keep encode/metadata logic in engines.
- [ ] Route both export and preview through that preparation. Export alone writes the queue-provided temp output. Preview stops before any output write and emits no JobEvent.
- [ ] Use the existing output extension rule: action.output.ext, or source extension for an empty override. Reject unsupported output targets rather than inventing an output path merely to select an encoder.
- [ ] Implement TransformedDraft: header admission, oriented bounded decode, shared geometry on oriented original dimensions, proxy transform, bounded RGBA publication. final_encoded_bytes is always None. If an operation cannot be represented truthfully on a proxy, return Unsupported for automatic mode and offer explicit encoded mode.
- [ ] Implement EncodedResult: enforce input policy before decode; shared full transform; existing encoder and final metadata injection; count final bytes; decode those bytes into the bounded display buffer; release full decoded/encoded intermediates before publication. No persistent artifact cache.
- [ ] Keep compress-to's bounded search algorithm identical to export. Never run it on every keystroke. Any target-unreachable error appears honestly and writes nothing.
- [ ] Add cooperative cancel checks before/after decode, operation, each target-search encode, metadata injection and publication. Do not claim those checks preempt a codec already executing.
- [ ] Replace independent scheduling with one running request plus one newest pending request, using the current Generation concept as the sole production counter. Extend that existing state machine with the required cancellation/stamp semantics rather than retaining a competing counter. Increment request stamp after every relevant input/option/mode change. Validate checked width*height*4 against the returned RGBA length and display policy before creating a Slint buffer.
- [ ] Closing/replacing the panel cancels its token, clears cached full result and invalidates the stamp. A late worker result must match session, panel, request and source snapshot before any UI field changes.
- [ ] Capture input identity before/after reading; reject detected replacement. Exact bytes describe the read snapshot, not a later reread by Run. Input may change again after preview; revalidation belongs to Run.
- [ ] Extend readout using shared geometry. Display proxy dimensions must never replace the intended output dimensions.
- [ ] Reuse existing fixed footer and Material controls. Keep Run semantics, natural mouse reachability, keyboard navigation, Back, Escape, Reset and RTL. Secondary preview controls must not push required actions outside the small panel.

### Future focused acceptance, execution deferred

| Case | Concrete expected result | Surface |
| --- | --- | --- |
| 400x200 input, resize 50 percent | Intended output 200x100 in both preview/readout/export | Engine + core |
| 400x200 input, square crop centered | Intended output 200x200; same crop bounds as export | Geometry + engine |
| EXIF orientation 6 photo | Oriented display and export agree; copied JPEG orientation is normalized | Production engine |
| Transparent PNG to JPEG | Preview shows the same white flattening as encoded output | Encoded result |
| JPEG keep_metadata true/false | Reported count equals respective final artifact length after metadata step | Engine |
| Two rapid option changes A/B | Only B publishes; at most one pending request survives | Scheduler |
| Close A, open panel B, late A finishes | No B picture/readout/status mutation | Production stamped dispatch |
| 16,000,001 source pixels or input above 64 MiB | Explicit encoded preview is refused by policy, no committed file | Admission |
| compress-to target below reachable bound | Explicit error, no fake byte size, no automatic retry | Engine + UI |
| Preview followed by source edit then Run | Run treats current input afresh; old preview is not acceptance evidence | Daemon |

Existing crop/resize/metadata/export cases remain controls. Later visual acceptance uses actual small and large panels, both languages/themes, not only helper tests. The 150 ms hive budget remains unmeasured until a separately resumed timing check.

## 2. S-143: named user presets

### Files and interfaces

Create crates/core/src/presets.rs and crates/core/tests/user_presets.rs. Modify core/lib.rs, options.rs, daemon/state.rs, ui.rs and watch.rs, UI stage_two.rs/lib.rs/hive.slint. Do not add a presets field to Config or activate remember-last-values.

The following new persistence types import existing TOML values and use field IDs, not parameter keys:

~~~rust
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct UserPreset {
    pub id: String,
    pub name: String,
    pub action_id: String,
    pub schema_fingerprint: String,
    pub values: toml::Table,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PresetDocument {
    pub version: u32,
    pub next_id: u64,
    pub presets: Vec<UserPreset>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreRevision(pub String);
#[derive(Clone, Debug)]
pub enum PresetStatus { Ready, NeedsReview(String), Unavailable(String) }
~~~

Proposed API responsibilities:
- canonical_values(action: &ActionDef, resolved: &toml::Table) -> Result<toml::Table, PresetError>: allowlist declared fields, map each param key back once, preserve type, exclude cleared/absent fields.
- validate_preset(preset: &UserPreset, action: Option<&ActionDef>) -> PresetStatus: resolve with existing OptionsSpec::resolve using field IDs; enforce current semantic/schema compatibility; never run anything.
- fingerprint(action: &ActionDef) -> String: deterministic SHA256 of canonical engine/op/output/base params and option semantics. Existing sha2 dependency is available in core. Sort unordered collections explicitly; translated labels are excluded.
- load(path: &Path) -> Result<(PresetDocument, StoreRevision), PresetError>: missing file is an empty version-1 document; corrupt or future-version files return read-only error state and retain bytes.
- save(path: &Path, expected: &StoreRevision, document: &PresetDocument) -> Result<StoreRevision, PresetError>: single writer, current-content compare, temp write/flush/replace, conflict without overwrite.

Define PresetError in the same module with InvalidName, InvalidValues, MissingAction, UnsupportedVersion, Conflict, Corrupt and Io variants; variants carry diagnostic text or the original IO error where appropriate. It is not an EngineError and cannot submit a job.

### Implementation sequence

- [ ] Extend proposed ADR 0016 with independent store semantics and the explicit field-ID representation. Preserve ADR 0012 resolver precedence.
- [ ] Define version-1 schema and stable preset IDs from the document-wide next_id counter. Format the next value as preset-{counter}, increment with checked_add and reject exhaustion. IDs are never reused after deletion. Do not add a randomness dependency. Treat omitted next_id as malformed version-1 data, not silent migration.
- [ ] Define name validation: trimmed nonempty, at most 64 Unicode scalar values, no control characters; reject duplicate trimmed/lowercased names within an action. Names are display text, never a path segment.
- [ ] Limit initial Save to built-in image actions other than images-to-PDF and declared numeric/bool/enum/px-pair fields. Arbitrary custom actions and free-text string controls are unsupported for persistence in this version. Save only the effective non-conflicting field values from the resolved valid form. Respect clears and validate through the current action before persistence. No path/input/output templates, commands or secret fields.
- [ ] Fingerprint changes never auto-apply. Revalidate values for diagnostics; set NeedsReview until the user explicitly reviews and re-saves a compatible preset. Missing fields/action, type/range errors or engine/op/output mapping changes remain Unavailable until repaired. No silent clamping or dropping.
- [ ] Use the daemon as the sole app writer and integrate presets.toml into watch stamps. A read/load failure preserves the last good in-memory list but disables saves against the damaged file until explicit repair. Do not overwrite unknown-version files.
- [ ] Implement the revision check with a single daemon write transaction. Document that arbitrary external editors can race a compare-and-replace without shared locking; do not advertise a universal OS transaction. If external-edit prevention is required beyond this contract, add the lock design before expanding claims.
- [ ] UI shows action-defined presets separately from user presets. Add explicit Save current, Apply, Rename and Delete via secondary in-panel controls; confirm Delete locally. Applying validates first, fills the form and schedules one preview; Run remains separate.
- [ ] Reset returns action defaults, not last saved preset. Plain clicks retain old behavior. File corruption, missing action and name conflicts are local, readable errors.

### Future focused acceptance, execution deferred

Save resize-50 at 25 percent under an Arabic name, restart the daemon, apply and observe 25 percent with zero jobs. Save scale/max_side with clears and prove the stored table cannot activate both. Delete a preset then save another and prove IDs are distinct. Change a saved field to a smaller range, remove an action, corrupt TOML, advance store version, rename concurrently and externally edit before Save: no wrong action, partial value application or destructive overwrite. Ordinary settings stay unchanged throughout. Apply one preset while an older preview finishes: only the current values publish.

## 3. S-144: batch outcomes and safe retry

This task consumes the existing queue and per-task events, not notification strings. The exact current event gaps are mapped in the source evidence section 3a. The proposed additive event contract below preserves the current IPC mirror. Do not add a persistent database.

**Files:** core/queue.rs, queue_impl.rs, engine.rs where existing task identity is surfaced; new core/job_result.rs and daemon/results.rs; daemon/state.rs, jobs.rs, progress.rs, notify.rs and tray modules; core/ipc.rs mirror conversion; new crates/ui/src/results.rs model and crates/ui/ui/results.slint component using existing Material controls; new focused queue/result/retry cases. Platform-specific tray siblings must be mapped before wiring an entry.

**Contract:** A task unit has an immutable ID, ordered input set, action/resolved-params snapshot and terminal status. A job holds task units and aggregate counts. A new retry job has parent_job and original task-unit references. The Engine::Task shape remains unchanged.

Proposed core::job_result records:
~~~rust
pub type TaskUnitId = u64;
#[derive(Clone, Debug)]
pub struct InputIdentity {
    pub path: std::path::PathBuf,
    pub len: u64,
    pub modified: Option<std::time::SystemTime>,
    pub regular_file: bool,
}
#[derive(Clone, Debug)]
pub struct TaskUnitManifest { pub id: TaskUnitId, pub inputs: Vec<InputIdentity> }
#[derive(Clone, Debug)]
pub struct JobManifest {
    pub action: crate::ActionDef,
    pub units: Vec<TaskUnitManifest>,
}
#[derive(Clone, Debug)]
pub struct RetrySubmission {
    pub parent_job: crate::queue::JobId,
    pub action: crate::ActionDef,
    pub inputs: Vec<std::path::PathBuf>,
    pub original_units: Vec<TaskUnitId>,
}
~~~

Define ResultLedger in daemon/results.rs with started(job: JobId, manifest: JobManifest), apply(event: &JobEvent), completed(job: JobId), clear_completed(), and retry_plan(job: JobId, registry: &Registry, engine_ok: &dyn Fn(&str) -> bool) -> Result<RetrySubmission, RetryBlocked>. These imports refer to existing core types and the proposed records above. Define RetryBlocked with MissingJob, NoEligibleFailures, GroupIncomplete, InputChanged, InputUnavailable, InvalidAction, ActionChanged, InvalidOptions, EngineUnavailable and AlreadySubmitting cases. The final IPC API is unchanged.

Add manifest: JobManifest to internal JobEvent::Started and unit: TaskUnitId to internal TaskDone/TaskSkipped/TaskFailed. The planner's enumerated unit index is attached by a queue-private wrapper around existing (Task, final_path), not a new required Engine::Task field. Capture the action and ordered input metadata once in that production plan. Update every internal event literal/match and retain public IPC Event variants exactly; Event::from explicitly ignores new internal fields with .. in its matches. No Request::RetryFailed or new wire variant is introduced.

- [ ] First map Started/TaskDone/TaskSkipped/TaskFailed/Finished producers and every consumer in daemon, CLI and MCP. A TaskDone output alone does not identify the original unit; introduce explicit unit identity where needed.
- [ ] Record the task manifest when the production planner expands a selection, before the runner mutates output paths. Preserve selection order and combined-action boundaries. Do not create a second planning algorithm in the UI.
- [ ] Reduce events into a typed in-memory ledger. Terminal states: succeeded, skipped with reason, failed with reason, cancelled, interrupted. On Finished(cancelled=true), unstarted manifest units become cancelled without inventing terminal queue events or changing old aggregate counts. A noncancelled completion with missing unit outcomes is interrupted/inconsistent, is not retried and produces a local diagnostic. Do not infer a crash from a timer. A daemon crash still loses session history.
- [ ] Add an explicit tray entry opening a user-requested results surface. Notifications keep current policy, no focus-stealing popup. The view exposes counts, per-unit status, existing outputs, clear-history and retry controls.
- [ ] Retain at most 100 completed jobs or 1,000 task units by evicting whole completed jobs. Active jobs are exempt until completion. A single larger job becomes an explicitly summary-only record with aggregate counts, optional first-output link and Retry disabled; do not retain an arbitrary partial task list. No disk history, restart replay, source-file content, or secret params in unsolicited notifications.
- [ ] retry_plan(job_id) returns either an immutable new submission manifest or a structured reason. It excludes successful/skipped/cancelled units. Per-file failures keep their own inputs; a combined failed task keeps the entire original ordered group.
- [ ] Compare the current action engine/op/output-mode against the stored action snapshot; changes yield ActionChanged. Revalidate stored resolved params through current options and current engine availability, then use existing Queue::submit with a new job ID. Missing modified-time identity, changed/missing/nonregular input or incomplete manifest blocks Retry and offers a fresh explicit run. Metadata identity remains best effort, not a content-hash/no-TOCTOU guarantee. OCR language preflight is still enforced by its adapter before any OCR processing.
- [ ] Make Retry idempotent while its submission is pending. After a child finishes, another retry is a new explicit user choice. Never reuse original output names as a command to overwrite them.
- [ ] Existing output links remain paths, not shell commands. A deleted output is reported unavailable; opening a folder must not be mistaken for opening an arbitrary URL or executing a filename.

**Future cases:** One oversized batch explicitly becomes summary-only with no partial Retry; missing input identity blocks Retry. Three independent inputs yield one success, one failed, one skipped: only the failed unit retries. A three-input merge failure retries all three in original order. A cancelled unit does not retry by default. Double-click Retry produces one child job. Old events after a completed job do not alter a newer child. Changed input/removal and invalid action fail preflight. Clearing history never cancels active jobs or removes output files. Exceeding retention evicts whole completed records. These are not executable results today.

## 4. S-145: unavailable-action explanations

**Files:** core registry/capability filtering and current no-action reasons; engines/locate.rs and adapters' available checks; daemon state/doctor composition; CLI doctor/actions output; existing UI no-actions/options surfaces; MCP discovery where it consumes the registry; docs/engines.md and later bilingual setup guide. Do not introduce an executable installer.

**Proposed data:** UnavailableReason distinguishes MissingExecutable { program }, MissingLanguageData { language }, UnsupportedInput, InvalidOptions, DisabledByPolicy and ProbeUnknown. CapabilityReport contains stable action ID, current runnable state, typed reason and an optional curated help ID. Public labels are localized by each surface.

- [ ] Inventory current filters and zero-action messaging before changing them. Reuse current kinds/reasons when semantically identical.
- [ ] Keep actionable hive cells limited to runnable operations. Add a secondary expected-actions explanation entry that does not obscure the primary action or require new OS integration.
- [ ] Centralize report construction in the existing registry/engine composition path. Do not let CLI, MCP and UI invent different executable/language checks.
- [ ] Probe executables without side effects, with bounded child lifetime where a version/language probe is necessary. Cache by tool identity and configuration; explicit refresh invalidates the cache.
- [ ] Resolve help by a curated action/tool ID. Display supported system-install guidance, never execute sudo/package managers or paste untrusted config into a shell command.
- [ ] Existing managed-download paths stay restricted to their existing allowlist and verification. Proposed ADR 0010 is not an accepted universal downloader. No new qpdf/Poppler automatic fetch.
- [ ] Keep policy-disabled actions distinct from missing tools, and input incompatibility distinct from a broken installation. A hanging or malformed probe is unknown/unhealthy, never success.

**Future cases:** Missing qpdf; available executable with requested OCR language absent; policy-disabled MCP; unsupported mixed selection; PATH executable replaced; probe timeout; tool installed followed by Refresh; no runnable actions with existing explanation. Assert no conversion/installer/network request occurs as a probe side effect. CLI/MCP/UI agree on stable IDs and reasons for the same state.

## 5. S-146: image OCR

**Files:** new engines/tesseract.rs and engines/tests/tesseract.rs; engines/lib.rs and locate.rs plus platform lookup siblings; existing process.rs helpers; actions/images.toml; core option/registry declarations only if existing types are insufficient; capability report from S-145; docs/engines.md and later Arabic/English operation guide.

**Architecture:** One Tesseract child per task, system executable and local language data. The existing engine trait and one-task/one-output temp commit remain unchanged. No model catalog or Python runtime is required for image-to-text.

- [ ] Write a proposed OCR architecture/licensing note before adapter registration. Record the Tesseract executable/license and locally installed traineddata separately. Distribution and language-data downloads are out of scope.
- [ ] Extend existing Program lookup with the Tesseract identity, including later-platform source exhaustiveness without running cross-builds. Do not spawn PATH probes during every UI redraw.
- [ ] Add a language preflight through the same bounded process helper. Parse only valid installed language identifiers from the explicit supported set; for the initial UI allow ara, eng and ara+eng after checking each component.
- [ ] Define the new image.to-text action using the existing typed options schema: language enum above, printed-text scope, UTF-8 txt output. First input scope is still JPEG, PNG and BMP; exclude other recognized image extensions in registry matching and recheck the actual decoded format in the adapter. Refuse TIFF/multiframe/unsupported input, never silently OCR one frame. Use the shared image preparation from S-142 to apply EXIF orientation and white alpha flattening into one private PNG before invoking Tesseract. Do not add automatic contrast/deskew preprocessing. No handwriting/translation setting or arbitrary CLI string field.
- [ ] Construct the process argv as [input_path, output_base, '-l', selected_language, 'txt'] with each argument separate. output_base is inside the task's private work directory; output naming is never derived from document text. Validate the executable's stdout/exit and expected output file rather than trusting a zero exit alone.
- [ ] Use one adapter-owned shared OCR permit per daemon instance, checking cancellation before waiting and before spawn. Apply the proposed 5-second probe/240-second OCR-child budgets and 16,000,000-pixel/64-MiB input admission. Refuse text outputs over 8 MiB before reading them into memory. These are initial policy values, not performance/RSS measurements. Reuse process-tree cancellation and cleanup; transfer valid successful txt into task.output only after all checks. The queue alone commits final output. On cancel/failure, leave no final file.
- [ ] Whitespace-only recognition returns EngineError::Failed with the specific No printed text recognized diagnostic and no task.output. Keep current serialized SkipReason unchanged: old IPC clients cannot decode an unnegotiated new enum variant. Map the diagnostic to a localized no-text explanation on user surfaces; do not normalize the underlying recognized document content.
- [ ] Preserve recognized text bytes as UTF-8 output. Do not normalize Arabic digits, spelling or diacritics. If returned text is not valid UTF-8, report an encoding error rather than silently replace bytes.
- [ ] Apply the S-145 missing-language explanation and ordinary batch statuses. No OCR starts from capability probing or preview requests.

**Future cases:** Missing executable; cancelled permit waiter; output above 8 MiB; ara missing but eng installed; ara+eng selection forwarded; metacharacters/spaces in source path; zero-text image; nonzero process exit; malformed/missing output; cancel and timeout; rotated/column/mixed-direction fixtures; original unchanged. Real quality evaluation needs separately approved real fixtures/ground truth and character-error/reading-order reports. Historical synthetic accuracy and old timings are not release claims.

## 6. S-147: searchable-PDF feasibility packet

**Files:** The current desktop study is docs/research/searchable-pdf-feasibility.md. Extend that evidence with the supported-version/dependency decision and a proposed PDF OCR ADR only after its number is confirmed free. No product adapter/action/dependency change in this task.

- [x] Desktop research compared system OCRmyPDF, Tesseract raster/PDF reassembly and a new Rust text-layer writer; conditional preference is system OCRmyPDF, with sources and limits in the feasibility document.
- [x] Complete the version-specific executable/library/language-data dependency/license inventory and optional-install boundaries before final adoption. No installer or runtime qualification is authorized by the desktop preference.
- [x] Define exactly one searchable-PDF output per task. Keep plain text extraction a separate operation. Explain why an OCR-only sidecar omits already-present text.
- [x] Define first-scope refusal for signed/encrypted documents and behavior for corrupt PDFs, mixed text/image pages, page ranges, rotation, existing text and unavailable languages. Preserve original page order/size and untouched input.
- [x] Specify child argv, child-tree cancellation, per-page/whole-job timeout policy, temp limits and no partial final commit. No force-OCR/redo by default, no silent PDF/A conversion or signature removal.
- [x] Define later Arabic acceptance: searchable words, logical copy/paste order, mixed Arabic/Latin/numbers, diacritics, multiple columns, rotated scans and existing-text pages. A picture that looks correct does not prove the hidden text layer works.
- [x] Record limitations of fixtures, accessibility tagging and archival claims. Do not advertise PDF/A, PDF/UA, searchable Arabic quality or digital-signature preservation without corresponding validation.
- [x] Return Adopt/Defer/Reject with reasons and the smallest viable dependency set. Only Adopt plus a reviewed scope creates the future PDF OCR implementation story. Do not reserve that implementation as done work now.

## 7. Later platform planning and release work

This is a later planning phase, not current implementation or validation.

- [ ] After each feature's behavior is stable, list its platform-specific boundaries: file replacement/identity, config paths, process-tree kill, executable/data lookup, notifications/tray activation, focus and output opening.
- [ ] Re-plan S-082 with real GNOME/Nautilus and KDE/Dolphin evidence. Native Alt/pointer mechanisms must have measured feasibility; the old Alt-only portal result remains disproven.
- [ ] Preserve S-138 as draft until actual distant-drop baseline and the owner's release-before-drop semantics are available. Do not infer those from the new preview/results features.
- [ ] Write the Windows feature matrix against the existing Windows source: local engine/data paths, Unicode/long paths, cancellation, results entry and new UI. Cross-build/runtime cases are later work, not assumed covered by Linux source review.
- [ ] Plan real X11 drag sources separately. Keep macOS unsupported until its own feasibility/integration scope is reopened, despite the historical build ADR.
- [ ] Resume release validation only as a separate authorized stage, with serial resource-limited commands and exact source evidence. S-141 publication does not follow automatically from committing these plans.

## 8. Deferred verification policy

Current stage: read source, review contracts, write cases and inspect exact diffs only. Do not execute any command below as part of planning.

When the owner resumes checks, each feature gets the smallest meaningful touched-module cases first, then its actual UI/native acceptance where applicable. Examples are image_preview for S-142, user_presets for S-143, job_result/retry for S-144, capability reasons for S-145 and a mock-process adapter plus approved real OCR fixtures for S-146. Retain existing safety regression controls; do not replace them with tests mirroring helper implementation.

A release candidate separately runs the required full set once on frozen source, under the repository's resource limits. New changes/failures justify another affected check; habit does not justify repeated full rebuilds. Native Linux desktop checks and Windows/X11/macOS qualification wait for the later-platform plan.

## 9. Parent review and completion record

- [ ] Every proposal has a current source baseline, user outcome, explicit exclusions, file boundary and proposed interface.
- [ ] Every destructive/racy/cancelled/stale-input path has a defined outcome and future observable case.
- [ ] Proposed APIs are labeled new; historical APIs are not silently renamed or claimed implemented.
- [ ] Review all existing event consumers and platform siblings before their task packet is approved.
- [ ] Confirm no tests/builds/CI/installers were run in this planning pass; record any real exception honestly.
- [ ] Confirm spec, stories, PLAN and active next-free number agree. Preserve current release/platform evidence limits.
- [ ] Produce a source-only review result; no production readiness or absence-of-defects guarantee.

## 10. Planning review record (2026-11-15)

Parent reviewed both Pi source reports against the baseline and official technical sources, then authored the scope, interfaces, story packets and sequencing. Material corrections incorporated: Arc rather than Rc at the worker boundary; final-artifact bytes rather than JPEG-only exactness; existing per-task notifications retained; internal task identity added without a new public IPC event; missing Poppler/qpdf reasons; no unnegotiated OCR SkipReason variant; explicit sparse-store compatibility handling; summary-only retention for oversized jobs; and format/language/resource boundaries for OCR.

Coverage: the five requested proposals have user outcomes, exclusions, files, proposed interfaces, failure cases and deferred focused acceptance. Searchable PDF has a separately scoped feasibility decision. Native platform work follows stabilized contracts. Existing S-138 and S-141 are preserved as separate work. Six new planned stories reserve S-142 through S-147; S-148 is next free.

Review method was source/document reading and exact diff inspection only. No build, test, lint suite, benchmark, compositor, installer, OCR/model execution or CI was run in this planning pass. The numerical limits are proposed policies, not measured results. Proposed ADR 0016 remains proposed. Planning completion is not product implementation or release acceptance.

## 11. Execution GO (2026-11-15)

The owner explicitly requested full implementation, selected MiniMax as executor and retained parent planning/review. Source implementation is now authorized. Build/test/visual/runtime/CI evidence remains deferred by the prior owner constraint. Work proceeds in bounded source packets with parent review of exact diffs; no packet is recorded as runtime-proven or released. S-142 shared engine preparation precedes preview UI. The independent missing-Poppler/qpdf explanation repair may proceed on disjoint paths while that foundation is built. No new branch/worktree is created.

## 12. Source execution record (2026-11-21)

Executors: `pi/minimax/MiniMax-M3.1-Flash` and `antigravity/gemini-4-neon`; parent review by the Opus manager. Status per section, source only, nothing built or tested:

- Section 1, S-142: source implemented and reviewed; superseded running previews finish and are discarded.
- Section 2, S-143: store, daemon worker and panel wiring implemented and reviewed.
- Section 3, S-144: ledger, CLI/IPC commands, Results settings page and tray entry implemented and reviewed.
- Section 4, S-145: core report and `kawaro doctor` implemented; hive and MCP surfaces in a follow-up packet.
- Section 5, S-146: adapter, language preflight, probe cache and APNG refusal implemented and reviewed.
- Section 6, S-147: Adopt, proposed ADR 0017, draft S-150.
- S-148 and S-149: implemented and reviewed; S-149 page size from accepted slots in a follow-up packet.

The unchecked boxes above stay unchecked until the deferred check stage runs their focused cases. Details: docs/development/2026-11-15-execution-status.md.
