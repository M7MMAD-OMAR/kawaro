# Fresh review findings on the S-142 to S-149 source (2026-11-21)

Two independent read-only reviews of commit 82771bc: Gemini 4 Neon (UI and daemon) and MiniMax-M3.1-Flash (core, engines, CLI, MCP). Each line names the fix owner and state.

## Fixed by the parent

- MCP read-only mode reported every action `disabled-by-policy`; read-only refuses the write tools, not the actions, so discovery now reports the machine like `kawaro doctor` (crates/mcp/src/tools.rs).
- MCP kept the first Tesseract lookup and language probe for the whole server life; discovery now looks the program up again and forgets its language probe per call.
- The daemon preset adapter turned every NeedsReview preset into Unavailable, so Review and save could never run (crates/daemon/src/presets.rs snapshot).
- Tried: cancelling a superseded running preview. Reverted, because an early cancel publishes a cancelled outcome before the newer request is queued; the scheduler tests pin one running plus one waiting request. Kept as a known limit.
- A language switch now asks the daemon for a fresh Results model, so job and unit rows follow the language (welcome.rs, settings.rs).
- Only expanded jobs check their unit outputs on disk (results_view.rs).
- Retry runs its disk checks on a worker thread and reports back on the event loop (daemon ui.rs).
- Customize Run drops keys equal to the action's fixed params before validation, so image.to-pdf runs (daemon ui.rs).
- The OCR language probe no longer waits on the run permit, and the OCR adapter writes straight to the queue's temp output (engines ocr.rs).

## Fix packets

Presets panel (crates/ui/src/stage_two.rs, lib.rs, hive.slint):
- After a delete or external reload the selection is validated against the old list and can point at a deleted preset.
- Conflict and success messages are overwritten at once by the automatic List reload.
- Editing an option never marks the preset controls dirty, so Save current stays wrongly enabled or disabled.
- Applying a named preset leaves user-edited text fields and the stage error stale; the preset name field keeps typed text after selecting a preset.
- Non-image engines on image input (image.to-text) show an empty "Transformed draft" well.
- Named preset rows and the preview note are not mirrored in RTL.

Paging (crates/ui/src/layout.rs, lib.rs, layer.rs):
- A last page with ring 1 plus few outer cells collapses to one ring and pulls More into ring 1, resizing the hive under the pointer.
- When the accepted ring-2 slots are 1 or fewer, the page size falls back to 11 and actions are dropped.
- Dropping files on More in ChooseDuringDrag pages and then dismisses the hive.
- Windows paging ignores the monitor edge and re-reads the cursor on page change.
- An X11 drop show keeps a stale placement from an earlier edge show.
- Page change is not guarded against a terminal or closing hive.

Results and daemon (crates/ui/src/welcome.rs, settings.rs, results_view.rs, crates/daemon/src/ui.rs, plan.rs):
- A refused retry is only logged; the Results page re-enables Retry without showing the reason. (The checks now run off the UI thread.)

## Accepted as documented limits

- The skipped-file and REQ cells keep the S-108 18-cell budget (S-108 AC3 tests pin it). With S-149 paging this budget could grow; that is a separate S-108 change, not done here.

- Preset store compare-and-replace is not a transaction against external editors (documented in ADR 0016).
- A plain `kawaro cancel JOB` without an instance targets the current daemon by design; `INSTANCE:JOB` is the checked form.
- A cancel that lands between the queue dropping a finished job's token and the Finished event reports success and cancels nothing.

## Second review (2026-11-22), fixed in S-151

Reviews of `fc47ca5..e5fd582`: Gemini 4 Neon (UI, daemon), MiniMax-M3.1-Flash (core, engines, CLI, MCP), Claude Haiku 5.5 (source-first security review). Fixes by MiniMax-M3.1-Flash and Gemini 4 Neon packets, parent review and fixes. All states below are source fixed and checked by fmt, clippy, the workspace tests and the Windows cross stages on the second machine; no compositor or real desktop check.

Fixed:
- OCR and OCRmyPDF engines had no `refresh`; their program lookup was a process-wide `OnceLock`, so a tool installed after start was never seen by the queue or the planner. Lookup is now a refreshable process-wide lock; availability comes only from the queue's engine instances.
- OCRmyPDF version probe was never warmed, and MCP had no OCRmyPDF branch: an old OCRmyPDF was offered and failed. Warmed at start and reload; MCP reports the doctor reason.
- The OCRmyPDF per-page timeout warning was read from a 4 KiB stderr tail and could be lost; it is now matched while stderr streams.
- `to_text` ran `tesseract --list-langs` per job; it uses the cached probe first.
- Image options were validated after the full decode; now before.
- The drop hive used plain matching and offered actions the plan marks unavailable; it uses the same cached reasons.
- Retry stat'd every input under the state lock; the file checks now run outside it. A retry is claimed before the worker starts, so a double click starts one child.
- The Results page reverted to the saved language after a language change before Save; it now follows the window's language.
- Drop on More left the dnd machine in `Over` after the OS drag ended, stranding the layer hive; it now enters click-to-choose mode (input region, keyboard, idle timeout).
- Paging assumed `size + 1` outer slots and reserved ring-1 slots on a single page; Windows drop paging used an unconstrained first page. Pages now follow the accepted slots on every page.
- The layer hive stored the unshifted anchor while planning with the clearance-shifted one.
- `image.to-pdf` and combined actions offered a preview the engine refuses; "Applying preset" stayed after a successful Apply.
- The engine preview modules are now built on Windows; the legacy source draft stays Unix only.
- Security, low: `kawaro results` printed file names and reasons raw (terminal escape and row forging); control characters are now escaped.
- Security, low (pre-existing): without `XDG_RUNTIME_DIR` the socket lived in shared `/tmp`; it now lives in a per-user 0700 directory, and bind and connect refuse a socket directory that another user owns or that group or others can write.
- Security, needs validation (pre-existing): the cross-device output copy followed a symlink at the temp name; the fallback now removes the target and creates it with `create_new`.

Accepted limits:
- OCRmyPDF admits 250 pages at 180 s per page, more than the 600 s job budget; the job timeout stops such a job.
- The layer path switches keyboard interactivity after a drop on More; a compositor may ignore a late change (not checked on a real compositor).
- Without the daemon's preview provider, Windows shows no source draft picture (`preview_schedule.rs` source path stays Unix only).
