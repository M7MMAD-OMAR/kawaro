# ADR 0011: A local MCP server for Kawaro

Status: accepted (owner approval 2026-11-04; implemented in S-121 and S-122)
Date: 2026-11-04

## Context

The owner asked for Kawaro to work as the file hands of any local AI app: the agent brings the intelligence, Kawaro performs deterministic file operations on the same machine, and nothing leaves the computer. The research is in `docs/research/mcp-agent-integration.md` and `docs/research/agent-integration-and-expansion.md`.

Facts that shape this record, all checked on 2026-11-04:

- The current MCP revision is **2026-07-28**. It is stateless: no `initialize` handshake, a new `server/discover` request, required `resultType` on every result, per-request `_meta`, and no `ping` or `logging/setLevel`. Hosts that still use the 2025-11-25 handshake are common and must keep working.
- The stdio transport is newline delimited JSON-RPC 2.0. The server MUST NOT write anything to stdout that is not an MCP message; stderr is free for logging.
- Hosts confirmed to start a local stdio server: Claude Desktop, Claude Code, Cursor and LM Studio. ChatGPT connectors and the Apps SDK are remote, so a local file server is not reachable there.
- `rmcp` 3.8.0, the Tier 1 Rust SDK, brings tokio and an async runtime. The rest of the workspace is synchronous and the base install stays light.
- The Tasks extension is Beta and no host in the official support matrix declares it. MCP Apps is Stable but it is an extension with a web bundle to maintain.
- The repo already has the pieces the server needs: `Registry::builtin`, `Registry::matching`, `queue::plan`, `queue::run_job`, the never overwrite output rules and one `Engine` trait. There is no network code in the tree.
- Before this block, actions carried fixed `[action.params]` and `Request::Run` carried only an action id, so no caller could change a value per run. ADR 0012 and S-117 add typed per-run options in core, which is what makes the `options` argument of the MCP tools meaningful.

Constraints that stay: nothing leaves the machine; the base install stays light; the original file is never modified; the output goes next to the original with a numbered name on collision; Arabic and English are both originals; one user process, no listener, no account.

## Options

| Option | Cost | What it gives | Verdict |
| --- | --- | --- | --- |
| `rmcp` 3.8.0 with tokio | Add an async runtime and a large SDK to a small synchronous server | Protocol handling and future revisions for free | Rejected in this block. Six tools do not justify tokio and the SDK weight, and the stateless revision makes a hand written loop small. It stays an option later. |
| Hand written stateless JSON-RPC loop over `serde_json` | One module we own, no new dependency, no runtime | Exactly the methods Kawaro needs, no new dependency and no runtime, testable with a `Cursor` | Chosen |
| Streamable HTTP transport | A loopback port, a shared secret, a same user check and a sentence in the privacy claim | Hosts that only accept URLs | Rejected in this block. No host that matters requires it. |
| The daemon as the executor | New IPC surface; `Request::Run` cannot carry params until S-120 | One queue, tray progress, cancel, one process for many hosts | Deferred. This block runs engines in process; a later story moves execution to the daemon. |
| One wrapper script or plugin per host | Per host maintenance | Nothing the snippet does not already give | Rejected |

## Decision

1. **A local stdio process started by the host.** The host runs `kawaro mcp` as a child process. There is no port, no authentication, no HTTP code and no remote connector. The process lives and dies with the host, and `stdout` carries only JSON-RPC lines while every log line goes to `stderr`.

2. **A hand written stateless JSON-RPC loop, no `rmcp` and no tokio.** One request line in, at most one response line out, flush after each response. Notifications (no `id`) get no response. Malformed lines get `-32700` with a null id; unknown methods get `-32601`. `rmcp` remains an option if the server grows.

3. **Off by default.** `config.toml` gains a `[mcp]` table with `enabled = false`. A host configuration alone cannot turn the server on: with `enabled = false` the server returns the error `MCP server is disabled: set [mcp] enabled = true in config.toml` and writes nothing to stdout. The settings window is not changed in this block; the user edits `config.toml`.

4. **Read-only first, writes behind a second switch.** `read_only = true` by default. `kawaro_formats` and `kawaro_plan` work in both modes. Every write tool returns `READ_ONLY` until the user sets `read_only = false`. Read-only mode is not a UI hint; it is checked before any engine is built.

5. **`allowed_roots` is the only way to name files.** Every root and every requested path is made absolute and canonicalised, so a symbolic link is checked by its target: a link whose target is outside every root is refused, and a link to a file inside a root is accepted as that file. Roots must be existing directories. Paths must be existing files under one root, otherwise `OUTSIDE_ROOTS`. An empty list accepts no path at all (fail closed), so a fresh install must add roots before any tool can touch a file. `max_files` (default 50) bounds one call. There is no delete, no overwrite and no in place operation in any mode, ever.

6. **One JSON audit line per `tools/call`.** Appended to `<config dir>/mcp-audit.jsonl` (overridable for tests), directory created on demand, rotated to `.1` when the file passes 1 MiB. Fields: `ts`, `tool`, `ok`, `paths`, `outputs`, `client`, `ms`. `client` comes from `initialize` `params.clientInfo.name` or the per-request `_meta` client info, and is used in the audit line only. `audit = false` turns it off.

7. **Six tools, deterministic order.** `kawaro_formats`, `kawaro_plan`, `kawaro_convert`, `kawaro_images_to_pdf`, `kawaro_pdf`, `kawaro_image`. The list does not vary per connection. `tools/list` carries `ttlMs` 300000 and `cacheScope` `private`. Job status and cancel are not in this block because every tool call is a bounded blocking run; they are deferred.

8. **One result shape.** `tools/call` returns `{"resultType":"complete","content":[text, optional image, resource_link per output],"structuredContent":{...},"isError":false|true}`. The text block is always present and carries the absolute output path, the bytes and the words "next to the originals"; the image block is a PNG thumbnail of the first image output (`preview` defaults to true); the `resource_link` is a `file://` URI with a name and a MIME type; `structuredContent` carries the machine fields (`action`, `engine`, `params`, `outputs` with `path` and `bytes`, `elapsed_ms`, `overwritten` false). A failed tool returns `isError: true`, a text block `ERROR [CODE]: message` and `structuredContent.error` with the same `code` and `message`. Stable codes: `DISABLED`, `READ_ONLY`, `OUTSIDE_ROOTS`, `TOO_MANY_FILES`, `NO_ACTION`, `UNKNOWN_ACTION`, `BAD_OPTIONS`, `ENGINE_MISSING`, `FAILED`.

9. **In-process engine execution in this block.** The server clones the action, replaces `action.params` with the resolved options, builds a channel and calls `kawaro_core::queue::run_job` with a fresh `CancelToken`, then drains the events. No daemon, no IPC, no new socket. The daemon becomes the executor in a later story, once `Request::Run` can carry params (S-120); until then a host and the hive do not share a queue or progress and cancel.

10. **Protocol details that matter to hosts.** A legacy `initialize` answer echoes the requested version when it is one of 2026-07-28, 2025-11-25, 2025-06-18, 2025-03-26 or 2024-11-05, and answers 2025-11-25 otherwise. `server/discover` advertises 2026-07-28, 2025-11-25 and 2025-06-18. Both carry `capabilities: {"tools":{}}`, `serverInfo` with the name `kawaro` and the crate version, and the instruction line "Local file conversion and PDF tools. Nothing leaves the machine.". `ping` is answered with an empty result for hosts that still send it. Every result object carries `resultType: "complete"`; older hosts ignore the extra fields. Only tools are declared: no resources, no prompts, no logging, no sampling, no elicitation, no multi round trip requests.

11. **The chat preview is decoration, the file is the deliverable.** The file next to the originals is guaranteed by the core queue and never depends on what a host renders. A host may draw the image, show the link as a link, or show neither; the text block alone is a complete answer. A preview that cannot be decoded or scaled is omitted and does not fail the call.

12. **Install is one paste.** `kawaro mcp --snippet claude|cursor|lm-studio|generic` prints one `{"mcpServers":{"kawaro":{"command":"kawaro","args":["mcp"]}}}` object to stdout and exits 0, even when the server is disabled. An unknown host is an error that lists the four. The command writes no host configuration file; a writing install helper (SEP-1024 `--dry-run` then `--write`) is deferred.

13. **Explicitly not in this block.** No Tasks extension (Beta, no host support), no MCP Apps (extension, web bundle, churn), no Streamable HTTP transport, no ChatGPT remote connector, no registry listing, no Skills package, no per-tool toggles, no settings window surface, no job status or cancel tools, no cancellation of a running call, no resources or prompts, no sampling or elicitation.

14. **The `[mcp]` table.** Read with defaults for missing keys and unknown keys ignored: `enabled` (false), `read_only` (true), `allowed_roots` (empty), `max_files` (50), `audit` (true), `preview_px` (512). `audit_path` exists only as a test seam and is not read from TOML.

## Consequences

- Any local host that can start a stdio server can use Kawaro with one pasted object, with no account, no port and no upload. The privacy claim needs no new explanation: the process is a child of the chat app and has no network code.
- We own the protocol loop. That is a small amount of code today and a maintenance obligation at every MCP revision; the alternative was adding tokio and the SDK.
- The six tool names and their descriptions are static, so a host sees a stable list. The actions behind `kawaro_formats` and `kawaro_plan` come from `Registry::builtin()` plus the user actions directory when it exists, the same set the hive shows, so a new action appears with no second catalog.
- A fresh setup needs three edits before a write: `enabled = true`, at least one root in `allowed_roots`, and `read_only = false`. That is deliberate (trust is the adoption gate) but it must be said in the docs and in the error messages.
- Because execution is in process, a large conversion holds the host connection, the host timeout cannot be seen or changed by Kawaro, there is no tray progress and no cancel. The daemon executor is the fix, not more threads in the server.
- A second host starts a second server process and a second engine run; nothing serialises them in this block. The core output naming still prevents overwrites.
- Audit lines contain user paths. The file is a log for the user, not a policy or a sandbox; a user who can run the server can edit it.
- Without Tasks and Apps the chat shows plain text, maybe an image and a link. No approve button, no thumbnail carousel, no open folder action.
- The legacy version echo is a compatibility answer, not a promise that every older revision is implemented. Older hosts ignore `resultType`, `ttlMs` and `cacheScope`.

## Risks

1. **Protocol churn.** Two revisions in nine months and the last one removed the handshake, sessions and ping. Pin the accepted versions, test the legacy handshake, and budget one maintenance story per revision.
2. **Host variance.** Timeouts, missing image rendering, links rendered as nothing, different config file names. Every feature must degrade to the text block.
3. **Security review.** A process that writes files on behalf of a model needs a written threat model: prompt injection through file names or content, tool poisoning, confused deputy, path escape. Roots, canonicalisation, the read-only switch and the audit line reduce the damage; they are not a sandbox, and stdio is not one either.
4. **Time of check to time of use race.** A path can be replaced with a symbolic link between the canonical check and the engine opening it. Without descriptor relative opens the window cannot be closed; it is accepted and documented.
5. **stdout discipline regressions.** One stray `println!` from our code or a dependency breaks every host. The tests parse every output line as JSON.
6. **Blocking calls and long jobs.** A 300 page scan can outlive a host timeout and leave the host with no answer. The file may still be written; the user sees a timeout. Deferred job tools are the answer.
7. **In-process duplication.** The CLI, the daemon and the server each call the queue differently. The rule is one core call per run (`queue::run_job`) and no engine logic in the server.
8. **Runtime and version trap.** Adding `rmcp` later brings tokio into a binary that is otherwise synchronous, so it stays a separate decision. The workspace also declares rust-version 1.85 while existing code already uses let chains (a Rust 1.88 feature); that mismatch predates this block and is tracked as a separate follow-up.
9. **Not checked on Windows or macOS in this block.** The crate is platform neutral, but no host was driven there.

## Deferred

- The daemon as executor, with `Request::Run` carrying params (S-120), tray progress, cancel and one queue for many hosts.
- `kawaro_job_status` and `kawaro_job_cancel`, long running jobs, the Tasks extension.
- Per-tool toggles, a settings window switch, a first-run prompt for roots.
- MCP Apps rich cards; a preview of a PDF page rather than a converted image.
- Loopback HTTP transport and any remote connector (ChatGPT, hosted gateways).
- An install helper that writes host configuration files after showing the exact command.
- Resources, prompts, logging, sampling, elicitation, and multi round trip requests.
- Byte and wall clock limits, cancellation of a running call, per-file options, crop and PDF option breadth.
- A Windows and macOS host check, and a listing in the MCP registry.
- A default root list (`~/Documents`, `~/Pictures`, `~/Downloads`) is not applied: roots are explicit and an empty list refuses every path. The owner can change that with a later ADR.
