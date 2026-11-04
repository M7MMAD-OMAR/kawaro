# Kawaro as a local MCP server: feasibility and design

Research note for the owner question of November 2026. No code was changed by this note. Every external fact links to the primary page listed in Sources; a claim I could not open is marked **unverified**. Kawaro facts come from docs/architecture.md, crates/core/src/action.rs and actions/*.toml.

## 1. Verdict in one screen

| Question | Answer | Reason |
| --- | --- | --- |
| Can a local AI app call Kawaro? | Yes for hosts that start a local stdio server: Claude Desktop, Claude Code, Cursor and LM Studio are confirmed | Section 4 |
| Is the owner scenario (ask an AI, preview in chat, file next to the originals) reachable? | Partly | The protocol can return an image plus a file link in one result, so the file part works everywhere; whether the chat draws the image is host specific and unproven in most hosts (section 5) |
| Long jobs in a chat | Blocking works for small files; the Tasks extension is beta and no host in the official matrix declares it | Sections 2 and 3 |
| Turn it off per tool | Yes, Kawaro owns its own tool list and can filter it from config.toml | Section 8 |
| Effort | Small: one crate, one CLI subcommand, 8 tools, docs, tests. **Guess**: 8 to 15 working days for an MVP, 20 to 30 with approval UX and audit log | Section 11 |

Recommendation: **GO for a stdio only MVP, off by default**, conditionally: read only first, no per run options, a plan tool before a write tool, and one host (Claude Desktop or Claude Code) as the acceptance target. Do not build MCP Apps or Tasks now.

## 2. State of MCP on 2026-11-04

Current specification revision: **2026-07-28**, previous revision **2025-11-25**. The changes that matter to a small local server, all from the official Key Changes page:

| Change | What it means for Kawaro |
| --- | --- |
| Stateless core: the initialize handshake is gone, each request carries version and capabilities in the _meta field, new server/discover RPC | A server is easier to write by hand; no per connection state |
| HTTP sessions and the Mcp-Session-Id header removed; list results no longer vary per connection | Irrelevant for stdio |
| ping, logging/setLevel and notifications/roots/list_changed removed; log level is per request in _meta | Simplifies the server; do not implement them |
| Every result carries a required resultType (complete or input_required); server initiated requests are replaced by multi round-trip requests (MRTR) | A confirmation prompt becomes an input_required retry, not an elicitation call, and only on 2026-07-28 hosts |
| Tasks moved out of the core into the beta extension io.modelcontextprotocol/tasks, polled with tasks/get and tasks/update | Long jobs are not a core primitive any more |
| subscriptions/listen replaces the HTTP GET stream and resources/subscribe | Irrelevant for stdio |

Extensions, from the official page: **MCP Apps is Stable** (io.modelcontextprotocol/ui, interactive HTML in a sandboxed iframe), **Tasks is Beta, intended for Core**, **Skills over MCP is Beta**. The community maintained **Extension Support Matrix lists no client with Apps support and no client with Tasks support**, which is stale: the ext-apps repository lists ChatGPT, Claude, VS Code, Goose and Postman as supported clients, and VS Code published Apps support in January 2026. Correction added by the Lead after checking https://raw.githubusercontent.com/modelcontextprotocol/ext-apps/main/README.md and https://code.visualstudio.com/blogs/2026/01/26/mcp-apps-support on 2026-11-04. Read that as: plain tools returning text, image and resource blocks stay the safe base; Apps is real on the top hosts but still an extension with churn, so it is a later phase, not an MVP promise.

Registry: the official MCP Registry is live at registry.modelcontextprotocol.io, with production and staging environments.

Churn is the headline risk: two major revisions in the nine months before this note, and the second one removed the handshake, sessions and ping. A Kawaro server must be written against the current revision and tested against a 2025-11-25 host.

## 3. Transports

| | stdio | Streamable HTTP |
| --- | --- | --- |
| Who starts the process | The host, as a child process | Kawaro, as a long lived listener |
| Network | None, inherited standard streams only | A loopback port to open, protect and document |
| Fits Kawaro's privacy claim | Yes, nothing to explain | Needs a sentence in the docs and a loopback check |
| Needed for | Every local chat app in section 4 | Hosts that only accept URLs (remote connectors, gateways) and future browser UIs |

stdio rules from the spec: newline delimited JSON-RPC, the server **MUST NOT** write anything to stdout that is not an MCP message, and stderr is free for logging. The same page notes the framing works unchanged over any reliable bidirectional byte stream including Unix domain sockets, which is convenient: one code path can serve a host over stdio and a test harness or the daemon over the existing socket.

Loopback HTTP is worth adding only when a host that matters refuses stdio, and then behind an explicit setting, bound to 127.0.0.1, with a shared secret and a same user check.

Host side limits: the Tasks documentation states the problem plainly, blocking ties up a connection and many clients and transport intermediaries impose timeouts that make this impractical beyond a few seconds. Kawaro cannot see or change a host timeout, so every write tool needs a bounded time and a clear still running, call kawaro_job_status, answer.

## 4. Host compatibility matrix

Only rows I opened and read are marked verified; everything else is honestly unverified for this note.

| Host | Local stdio server | Configuration | Verified |
| --- | --- | --- | --- |
| Claude Desktop | Yes, started from the config file, with your explicit permission for each action | claude_desktop_config.json in ~/.config/Claude/ on Linux, the Claude folder under APPDATA on Windows, Application Support/Claude on macOS; a mcpServers object whose entries hold command and args | yes |
| Claude Code | Yes | claude mcp add, with --transport http for remote; local servers supported | yes |
| Cursor | Yes, servers in any language that can print to stdout or serve an HTTP endpoint | mcp.json | yes |
| LM Studio | Yes, both local and remote MCP servers, following Cursor's mcp.json notation | mcp.json inside the app | yes |
| VS Code and GitHub Copilot | Widely reported, **not verified here**: the docs page renders by script and returned no text | unverified | no |
| Zed, Windsurf, JetBrains AI, Gemini CLI, Codex CLI, LibreChat, Raycast, n8n | Reported in the wild, not checked | unverified | no |
| Open WebUI | Not checked; the documentation URLs I tried returned 404 | unverified | no |
| Ollama | Model runtime, not an MCP host | not applicable | no |
| ChatGPT | Connectors and the Apps SDK are remote; nothing found for a local stdio server | unverified | no |

Practical reading: the hosts a Linux or Windows user is most likely to try first are confirmed, so an MVP can be accepted on those and the rest added to the matrix as users report them.

## 5. Can the result show a preview in the chat and reach the file?

Tool results in the content field may contain text, image, audio, resource links and embedded resources, and any of them may carry annotations. There is also structuredContent, JSON validated by the tool's optional outputSchema, and the spec asks servers that return it to also include the serialized JSON as text for older clients.

| Block | What it does in the owner scenario | Host rendering |
| --- | --- | --- |
| image | A PNG preview of page 1 or of the converted image, base64 in the result | Some hosts draw images; no per host guarantee I could verify |
| text | The absolute output path, sizes, and a sentence the user can copy | Always drawn |
| resource_link | A file URI with a name and MIME type, for example file:///home/u/scan.pdf | A link at best; there is no standard reveal in file manager action |
| structuredContent | Sizes, dimensions, page count, job id, no overwrite flag | Model facing in most hosts, sometimes hidden |

Consequences to accept before building:

1. The file on disk is the reliable deliverable; the chat preview is decoration. Kawaro already writes beside the original and never overwrites, so the visible outcome does not depend on any host.
2. A PDF preview has to be rasterised by Kawaro and sent as a small PNG (**guess**: 512 px wide, under 200 KB base64), which costs time on the call path.
3. MCP Apps, the only standard way to get a rich interactive panel (a page carousel, an open folder button), is Stable in the spec and the ext-apps repository lists ChatGPT, Claude and VS Code as supported clients (checked 2026-11-04), even though the community matrix is stale. It is still an extension with churn and a web bundle to maintain, so keep it out of the MVP and treat it as the phase after the plain tools are trusted.

## 6. What the repo already gives us

- Registry::matching already answers which actions fit these files, and kawaro run ACTION_ID FILES already has the shape an MCP tool needs (action id plus paths). No new core model is required.
- Output naming, hard link placement and the no overwrite rule live in crates/core/src/output.rs; the MCP server must call the same code, not a copy.
- Heavy work already runs as child processes behind the Engine trait, and the daemon already queues, reports progress and cancels. The MCP process should be a thin client of the daemon, exactly like kawaro run.
- **Gap**: actions carry fixed params in [action.params] and there is no per run override anywhere. An MVP therefore exposes Kawaro actions as they are; per run options (quality, max side) wait for the declarative option schema proposed in the parallel image and preview research (docs/research/image-editing-and-preview.md, in progress).
- **Gap**: nothing in the config model covers permissions. A [mcp] section is new surface and needs an ADR.

## 7. Security model

The official Local Server Security page is blunt: a stdio server is an ordinary operating system process that inherits the user's files and network, the stdio transport is not a sandbox, and client and server share one trust domain. The same page states that clients offering one click installation must show the exact command (SEP-1024). Kawaro's advantage is that its only capability is files and its only behaviour is read an input, write a new file.

| Threat | Countermeasure in the server |
| --- | --- |
| Prompt injection in file names, EXIF or PDF text | Kawaro never interprets content; return names escaped in a text block, never as instructions; never read a path from file content |
| Tool poisoning (descriptions used as instructions) | The catalog is built from the shipped actions/*.toml, not from user files; annotations are static, and the spec says hosts must treat them as untrusted anyway |
| Confused deputy | No network, no tokens, no proxying to another service; nothing to steal except files the user already owns |
| Writes outside the user's intent | allowed_roots list, canonicalise every path, resolve symlinks and refuse when the target escapes a root, refuse system directories and dot files in the home directory |
| Destructive operations | Kawaro has none today (original untouched, never overwrite). Keep it that way: no delete, no overwrite, no in place tool, ever |
| Resource abuse from a confused model | Limits on file count, bytes read, wall clock and preview size; cancel maps to the existing CancelToken |
| Daemon socket exposure | The MCP process talks to kawarod over the user only socket or named pipe that already exists; never expose that socket to the host |
| No audit trail | One JSON line per call in the log directory: tool, paths, result, bytes, caller clientInfo name, duration |
| Silent activation | Off by default; the server refuses to start when mcp.enabled is false, so a host config alone cannot turn it on |

The general Security Best Practices page covers the wider attack classes (confused deputy, token passthrough and related OAuth flows). Almost none of it applies to a stateless stdio file server with no credentials, which is an argument for keeping the server credentials free.

## 8. Recommended design

Command: kawaro mcp, a stdio server started by the host, logging to stderr only. It starts kawarod on demand the way kawaro open does. Optional flags: --roots DIR (repeatable, overrides config for one run), --read-only, --max-files N.

config.toml additions (new ADR required):

    [mcp]
    enabled = false
    read_only = true
    allowed_roots = ["~/Documents", "~/Pictures", "~/Downloads"]
    tools = { convert = true, images_to_pdf = true, pdf = true, image = true, inspect = true, jobs = true }
    max_files = 50
    preview_px = 512

Settings window: one switch on the Integration page plus a tool list with per tool toggles, no new window, no focus stealing, exactly like the existing Notifications and integration page.

Tool set, kept small, because models pick better from 6 to 10 tools than from 20:

| Tool | Kind | Notes |
| --- | --- | --- |
| kawaro_formats | read | Formats and engines available on this machine (core::probe) |
| kawaro_plan | read, dry run | Given paths, return the action ids that fit, the output name, the estimated size and time. No writes |
| kawaro_convert | write | Run one action id on paths, return outputs. The general case |
| kawaro_images_to_pdf | write | The headline case, a named tool so the model does not have to guess an action id |
| kawaro_pdf | write | op is merge, split, rotate, compress or to_images |
| kawaro_image | write | op is convert, resize, rotate, flip, grayscale or compress |
| kawaro_job_status | read | Poll a job started with mode job |
| kawaro_job_cancel | write | Cancels a job |

tools/list example, one tool, abridged:

    {"jsonrpc":"2.0","id":1,"method":"tools/list","result":{
     "tools":[{
      "name":"kawaro_images_to_pdf",
      "title":"Combine images into one PDF",
      "description":"Make one PDF from local image files, saved next to the first image. Nothing leaves the machine.",
      "inputSchema":{"type":"object","properties":{
        "paths":{"type":"array","items":{"type":"string"},"minItems":1},
        "mode":{"type":"string","enum":["wait","job"],"default":"wait"},
        "preview":{"type":"boolean","default":true}},
       "required":["paths"]},
      "outputSchema":{"type":"object","properties":{
        "outputs":{"type":"array","items":{"type":"object"}},
        "duration_ms":{"type":"integer"}}},
      "annotations":{"readOnlyHint":false,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}
     }],
     "ttlMs":300000,"cacheScope":"private"}}

tools/call result example, success:

    {"jsonrpc":"2.0","id":2,"result":{
     "resultType":"complete",
     "content":[
      {"type":"image","data":"BASE64_PNG_OF_PAGE_1","mimeType":"image/png"},
      {"type":"text","text":"Created /home/u/Pictures/scan.pdf (2.1 MB, 12 pages) next to the originals."},
      {"type":"resource_link","uri":"file:///home/u/Pictures/scan.pdf","name":"scan.pdf","mimeType":"application/pdf"}],
     "structuredContent":{"outputs":[{"path":"/home/u/Pictures/scan.pdf","bytes":2202009,"pages":12,"action":"image.to-pdf"}],"duration_ms":4200,"overwritten":false},
     "isError":false}}

Long work: default mode wait, with progress notifications (a progress token in _meta, current and optional total) for jobs Kawaro estimates under a few seconds; above that, return mode job with a job id and let the model poll the status tool. This avoids the beta Tasks extension and every host timeout problem while staying inside plain tools.

Errors: user level failures return isError true with a short text and a stable code (ENGINE_MISSING, NO_ACTION, OUTSIDE_ROOTS, TOO_MANY_FILES). Transport errors are reserved for a broken server, so hosts never lose the message in a red toast.

Idempotency: a retried call never overwrites, it produces a second file with a number suffix. That is safe but surprising, so kawaro_plan exists and the tool description says a retry makes a second file.

Versioning: serverInfo version follows the Kawaro version, the server accepts both protocol revisions the SDK supports, and kawaro_formats reports the action catalog so a stale host config is visible in the chat.

Install helper: kawaro mcp install --host claude-desktop|cursor|lm-studio --dry-run prints the exact snippet and only writes after --write, following SEP-1024. kawaro mcp snippet --host X prints it for copying.

## 9. Agent Skills angle

The MCP documentation now has a Build with Agent Skills page and a Beta Skills over MCP extension; the client matrix shows only ChatGPT partial. A Kawaro skill folder (a short SKILL.md: when the user says make these a PDF, call kawaro_images_to_pdf with these arguments; never invent paths) is cheap, and it also works for hosts without MCP if it tells the agent to run the kawaro CLI. Ship it after the MCP server, not instead of it.

## 10. SDK choice

| Option | Licence | Status | Fit |
| --- | --- | --- | --- |
| rmcp 3.8.0, the official Rust SDK in modelcontextprotocol/rust-sdk | Apache-2.0, with a licence file noting a transition from MIT, so contributions are mixed | Tier 1, implements 2026-07-28 and stays compatible with 2025-11-25, edition 2024, rust-version 1.88, tokio | Best fit, one dependency tree, GPL-3.0-or-later compatible |
| Hand written server over serde_json | Kawaro's own code | The stateless 2026-07-28 revision makes this small | Escape hatch if the MSRV or tokio is a problem |
| Python or TypeScript SDK | MIT | Tier 1 | Rejected: adds a Node or Python runtime to a light base install |

**Conflict with the current repo**: CLAUDE.md states rust 1.85, rmcp 3.8.0 needs 1.88. Either raise the workspace MSRV (a separate decision, it touches the distro toolchains in CI) or hand write the stdio loop. Settle this before writing the story.

rmcp is async and tokio based. kawaro mcp is its own process, so it can own a small tokio runtime without changing the daemon's thread model; the daemon keeps doing the work.

## 11. Effort (guesses, with assumptions)

| Piece | Guess | Assumption |
| --- | --- | --- |
| crates/mcp plus the kawaro mcp subcommand, stdio, tool registry | 2 to 3 days | rmcp works at the repo MSRV, daemon IPC reused as is |
| 8 tools over the existing registry and queue | 2 to 3 days | No per run options, no preview at first |
| Roots, read only mode, limits, audit log | 2 to 4 days | One reviewer pass over the path rules |
| Settings switch and per tool toggles | 1 to 2 days | Reuses the existing Integration page |
| Preview rendering (PDF page to PNG, image thumbnail) | 1 to 2 days | pdf.to-images already exists through Ghostscript |
| Install helper and snippets for 3 hosts | 1 day | Text only |
| Docs (English plus Arabic), CHANGELOG, ADR | 1 day | Standard policy |
| Tests (unit for path rules, one end to end with a scripted stdio client) | 2 to 3 days | Fixtures already exist |
| **MVP total** | **8 to 15 days** | Assumes no new engine work |

The honest risk in the estimate is host testing: each host has to be installed, configured and driven by hand at least once, and that time is not in the table.

## 12. Risks

1. **Protocol churn.** Two revisions in nine months; the last one removed the handshake, sessions and ping. Budget one maintenance story per revision, and pin the SDK.
2. **Host variance.** Timeouts, no progress UI in some hosts, images drawn or not drawn, a resource link rendered as nothing. Every feature must degrade to the text answer.
3. **Security review.** A process that writes user files on behalf of a model needs a written threat model and a reviewer who did not write it. That is the ADR.
4. **Support surface.** Different config files, formats and install paths per host, each changing without notice. Keep the install helper data driven.
5. **Product confusion.** Two entry points to the same engine (hive and chat) with different option models while actions still carry fixed params. Accept that the MCP side exposes exactly what the hive shows.
6. **Discovery.** Listing in the official registry makes Kawaro publicly discoverable from hosts; that is a marketing win and a standing accuracy obligation.

## 13. Conditions for GO

Go if all of these hold: (a) one host is chosen as the acceptance target and installed on a test machine; (b) read only mode ships first and write mode is a second toggle; (c) the plan tool exists before the write tool; (d) the ADR settles roots, limits, audit log and the MSRV question; (e) nobody promises a live chat preview, only that the file appears next to the originals, with a preview where the host draws one.

Do not go if the MSRV cannot move and the hand written server is judged too much surface, or if rich in chat panels are wanted now: that needs MCP Apps, and no host in the official matrix supports it today.

## Sources

All accessed 2026-11-04.

- MCP specification, latest revision 2026-07-28: https://modelcontextprotocol.io/specification/latest
- Key Changes for 2026-07-28: https://modelcontextprotocol.io/specification/2026-07-28/changelog
- Base protocol overview: https://modelcontextprotocol.io/specification/2026-07-28/basic/index.md
- Transports overview: https://modelcontextprotocol.io/specification/2026-07-28/basic/transports.md
- stdio transport: https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio.md
- Tools (content blocks, outputSchema, structuredContent, isError, annotations, tool names): https://modelcontextprotocol.io/specification/2026-07-28/server/tools.md
- Progress utility: https://modelcontextprotocol.io/specification/2026-07-28/basic/utilities/progress.md
- Extensions overview (Apps Stable, Tasks Beta, Skills Beta): https://modelcontextprotocol.io/extensions/overview.md
- MCP Apps overview: https://modelcontextprotocol.io/extensions/apps/overview.md
- MCP Tasks overview (beta, polling, timeout argument): https://modelcontextprotocol.io/extensions/tasks/overview.md
- Extension support matrix, community maintained: https://modelcontextprotocol.io/extensions/client-matrix.md
- Official SDKs and tiers: https://modelcontextprotocol.io/docs/2026-07-28/sdk.md
- Local Server Security (stdio is not a sandbox, SEP-1024): https://modelcontextprotocol.io/docs/2026-07-28/tutorials/security/local-server-security.md
- Security Best Practices: https://modelcontextprotocol.io/docs/2026-07-28/tutorials/security/security_best_practices.md
- Connect local servers (Claude Desktop config paths and format): https://modelcontextprotocol.io/docs/2026-07-28/develop/connect-local-servers.md
- Build with Agent Skills: https://modelcontextprotocol.io/docs/2026-07-28/develop/build-with-agent-skills.md
- MCP Registry, production and staging: https://registry.modelcontextprotocol.io/
- Rust SDK repository: https://github.com/modelcontextprotocol/rust-sdk
- Rust SDK README (revision support, lifecycle modes): https://raw.githubusercontent.com/modelcontextprotocol/rust-sdk/main/README.md
- Rust SDK workspace manifest (rmcp 3.8.0, Apache-2.0, edition 2024, rust-version 1.88): https://raw.githubusercontent.com/modelcontextprotocol/rust-sdk/main/Cargo.toml
- Rust SDK licence file (MIT to Apache-2.0 transition): https://raw.githubusercontent.com/modelcontextprotocol/rust-sdk/main/LICENSE
- Claude Code MCP reference (claude mcp add, stdio and HTTP): https://code.claude.com/docs/en/mcp.md
- Cursor MCP docs (mcp.json, servers that print to stdout): https://cursor.com/docs/context/mcp
- LM Studio MCP docs (mcp.json, Cursor notation, local and remote): https://lmstudio.ai/docs/app/mcp
- VS Code MCP docs (page exists, content not readable by fetch, marked unverified): https://code.visualstudio.com/docs/copilot/chat/mcp-servers
