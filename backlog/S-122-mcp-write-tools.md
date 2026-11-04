# S-122 MCP write tools: convert, images to PDF, PDF and image operations
Status: done (2026-11-04)

## Story
As a user who trusts a local agent, I want it to run a conversion with my settings and get the result back as text, a small preview, a file link and a machine readable summary, so that the file lands next to my original with no upload and no overwrite. This is the second slice of ADR 0011: it executes the four write tools declared in S-121, in process, with the options from S-117 and the result shape the contract fixes.

## Facts found while planning
- `queue::run_job(job, action, files, engine, cancel, events)` runs one job on the calling thread and reports `JobEvent::TaskDone` and `JobEvent::TaskFailed`; the final names come from `queue::plan` and the queue placement, so a second run takes the next number and never replaces a file.
- The engine trait gives `id()` and `available()`; `kawaro_engines::all()` returns every compiled engine, available or not.
- `image.to-pdf` exists (combined output) and the image and PDF engines already implement every op the named tools need.
- The preview decoder is the `image` crate; base64 0.23.1 is already in `Cargo.lock`.
- The tester's `crates/mcp/tests/s121_mcp.rs` covers this story too; the tests for the write path are named below.

## Acceptance criteria
- AC1 (`kawaro_convert`): arguments `{action: string, paths: [string], options?: object, preview?: bool}`. The checks run in this order: path policy (`OUTSIDE_ROOTS`, `TOO_MANY_FILES`), `read_only` (`READ_ONLY`), action id exists in the registry (`UNKNOWN_ACTION`), the action is offered for these paths by `Registry::matching` (`NO_ACTION`, and the message names at least two action ids that are available), an engine with that id exists in `kawaro_engines::all()` and `available()` is true (`ENGINE_MISSING`, the message names the engine id), then options. A refused check builds no engine and writes no file.
- AC2 (named tools): `kawaro_images_to_pdf` takes `{paths, options?, preview?}` and maps to `image.to-pdf`. `kawaro_pdf` takes `{op, paths, options?, preview?}` where `merge`, `split`, `rotate`, `compress` and `to_images` map to `pdf.merge`, `pdf.split`, `pdf.rotate`, `pdf.compress` and `pdf.to-images`. `kawaro_image` takes `{op, paths, options?, preview?}` where `resize`, `rotate`, `flip`, `grayscale` and `compress` map to `image.resize-50`, `image.rotate-90`, `image.flip-h`, `image.grayscale` and `image.compress`. An `op` outside the list is `BAD_OPTIONS` and the message names the accepted values. After mapping, the same checks as AC1 apply.
- AC3 (options): when the action carries `options`, the argument object is resolved with `action.options.resolve_params(&action.params, options)`; the resolved table replaces `action.params` for the run and is echoed as `structuredContent.params` (TOML values converted to JSON). When the action carries no options, an absent or empty `options` is accepted and any non empty `options` is `BAD_OPTIONS`. An `OptionsError` from resolve is `BAD_OPTIONS` and the message carries the option error text. `options` is always the base `[action.params]` plus explicit values; field defaults are never applied by the server.
- AC4 (in-process execution): the action is cloned, its params are replaced, and the run calls `kawaro_core::queue::run_job(0, &action, &files, engine.as_ref(), &CancelToken::new(), &tx)` on the server thread; the sender is dropped and every event is drained. The server never contacts the daemon, opens no socket and starts no process for Kawaro itself. `TaskDone` outputs and `TaskFailed` errors are collected.
- AC5 (success result): `resultType` `complete`, `isError` false, and `content` in this order: one `text` block, the optional `image` block, then one `resource_link` per output. The text holds each absolute output path, its size in bytes and the words `next to the originals`. Each `resource_link` has `uri` starting `file://`, `name` equal to the file name and a `mimeType` for the file. `structuredContent` is `{action, engine, params, outputs: [{path, bytes}], elapsed_ms, overwritten: false}` with `path` absolute and `bytes` the on disk size.
- AC6 (preview): `preview` defaults to true. When true and at least one output has an image extension, the first such output is decoded with the `image` crate, scaled so its longest side is at most `preview_px`, re-encoded as PNG and sent as `{"type":"image","data":<standard base64>,"mimeType":"image/png"}`. When `preview` is false no image block is sent. A decode, scale or encode failure omits the image block and leaves `isError` false; the text and the links are unaffected.
- AC7 (failure): when any task fails, `isError` is true, the first error text is returned as the text block `ERROR [FAILED]: message`, `structuredContent.error` is `{code:"FAILED", message}`, and `structuredContent.outputs` holds only the outputs that succeeded. The call is still a JSON-RPC success with a result, never a JSON-RPC error.
- AC8 (never destructive): the server has no delete, overwrite or in place operation. A repeated call on the same input creates a numbered second file (the core placement rule) and reports `overwritten: false`; the input file is unchanged and both outputs exist. `out` and `in_place` arguments are not accepted, and an unknown argument is `BAD_OPTIONS`.
- AC9 (audit): a write call appends the same one line format as S-121 with `ok` true and `outputs` naming the final paths, or `ok` false and an empty `outputs` when the run failed. `ms` is the wall clock of the call.
- AC10 (gates and scope): `cargo fmt --check` and `cargo clippy -p kawaro-mcp --all-targets -D warnings` pass, and `cargo test -p kawaro-mcp` passes with the S-121 tests in the same file. No `rmcp`, no tokio, no HTTP, no new `Cargo.lock` package, MSRV 1.85. No change outside `crates/mcp`, the workspace manifest, `crates/cli/Cargo.toml` and `crates/cli/src/main.rs`; no change to `crates/core` other than what S-117 already made, and none to the daemon, the UI, `docs/architecture.md`, `docs/engines.md` or `CHANGELOG.md` in this block.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | `convert_image_to_webp_uses_the_options_and_previews_the_output`; refusal order and the remaining codes by unit test in the crate | unit |
| AC2 | Named tool mapping by unit test in the crate: each `op` reaches the expected action id | unit |
| AC3 | `convert_image_to_webp_uses_the_options_and_previews_the_output` asserts `params.quality == 60`; `BAD_OPTIONS` for an options object on an action without options | unit |
| AC4 | The same convert test: the output exists, no daemon is contacted, and the process has no socket | unit |
| AC5 | The convert test asserts the text, the image block, the resource link, `action`, `engine`, `outputs[0].path` and `bytes`, `overwritten` false and `elapsed_ms` | unit |
| AC6 | The convert test asserts the image block for a JPEG input, `mimeType` `image/png` and non empty `data`; `preview: false` omits it | unit |
| AC7 | Unit test in the crate: a task failure gives `isError` true, code `FAILED`, the error text in the content and the successful outputs only | unit |
| AC8 | `a_second_convert_does_not_overwrite_the_first_output` | unit |
| AC9 | `audit_writes_one_json_line_per_tool_call` (read call) plus a write call line by unit test in the crate | unit |
| AC10 | The cargo gates above; reviewer reads the manifest diff | gate |

## Out of scope
Read tools, the protocol loop and the config (S-121); per-run options in the daemon and the CLI `--set` (S-120); job status, job cancel and long running jobs; MCP Apps and a rich result card; a PDF page preview on the server; byte and wall clock limits; cancellation of a running call; Windows and macOS host checks; docs, `docs/architecture.md`, `docs/engines.md` and `CHANGELOG.md` changes, which the Lead keeps outside this block.

## Review log
- 2026-11-04, planner: story written from the fixed implementation contract. The tester's `crates/mcp/tests/s121_mcp.rs` covers S-121 and S-122 in one file; this story names only the write path tests, and the reviewer should map every write path assertion here. The contract fixes the check order (paths, read_only, action, matching, engine, options) and the tool to action mapping; both are written out above so a test can assert the first refusal, not just any refusal.

- 2026-11-04, implementer: the four write tools run `queue::run_job` in process with the resolved options; the success result is text, optional PNG preview, one resource link per output and structured content; a failure is a tool result with `isError`; the second run takes a numbered name.
- 2026-11-04, lead: end-to-end evidence from the stdio run: `image.to-webp` with `quality: 60` returned the text, an image block of 9584 base64 characters and a resource link, wrote 214 bytes next to the input, and the retry wrote `photo (1).webp` with the input unchanged; read-only mode answered `ERROR [READ_ONLY]` and wrote nothing.

- 2026-11-04, reviewer: REWORK on the check order and the missing write-path tests.
- 2026-11-04, mcp-impl and tester: the order is now paths, read_only, action, matching, engine, options; the write-path tests cover the named tool mapping, preview true and false, options on an action without options, the FAILED shape with an empty output list, the write audit line and unknown arguments.
- 2026-11-04, lead: full gates green (fmt, clippy on core, mcp and cli, core tests, the 34 mcp tests and the cli suite), and the reviewer probes all pass. Docs and the ADR MSRV wording corrected in the same round.

- 2026-11-04, mcp-impl: kawaro_pdf now maps ten ops (merge, split, rotate, compress, to_images, extract_pages, delete_pages, reorder_pages, metadata, to_text) and the op enum is generated from the same table; mcp tests green.
