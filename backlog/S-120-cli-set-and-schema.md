# S-120 Per-run values on the wire: Request::Run params, kawaro run --set and the schema export
Status: done (2026-11-04)

## Story
As a script author or a coding agent that already watches the daemon, I want to pass one option value to one action run and read the JSON Schema of an action, so that a customized run is a command line argument and not a hand edited action file. This is the additive IPC and CLI half of ADR 0012. It changes no UI.

## What this story will prove
- The action options contract reaches the daemon without a second merge implementation: the daemon resolves the params with the same core function the rest of Kawaro uses, and an unknown key is refused with the option error text.
- The wire change is additive: an old client that sends no params runs the base `[action.params]` exactly as today, and a request from a new client is still valid JSON for an older field set.
- The CLI speaks the same schema as MCP: `kawaro run X --set K=V` and `kawaro actions --id X --schema` read the core export, and no CLI code knows the option list of any action.
- A user can check the schema before running: the output is the exact `params_schema()` JSON, closed with `additionalProperties: false`, so a script can validate its own values.
- The no-overwrite and output naming rules are unchanged; a customized run lands next to the original like a plain one.

## Dependencies
- S-117 for `ActionDef.options`, `resolve_params`, `params_schema` and `OptionsError`. This is the hard dependency: without the schema this story has nothing to send and nothing to print.
- S-121 and S-122 for the shared contract, not for code: the MCP `options` argument and `kawaro run --set` must resolve through the same core function and produce the same params, so both stories pin the same test values (quality 60 and max_side 1280) and neither grows a second merge. This story and the MCP stories can be implemented in either order.
- The daemon request loop, `ipc.rs` `Request::Run` and the existing `kawaro run` client path. No new dependency is planned.
- Nothing else. Not in this block; S-119 does not wait for it.

## Decisions to record (provisional, fixed on approval)
1. `Request::Run` gains `#[serde(default, skip_serializing_if = "Option::is_none")] params: Option<toml::Table>`. The JSON key is `params`, the same shape MCP uses. A missing field means the base params.
2. The daemon resolves the explicit params with `resolve_params(&action.params, &params)` before planning, replaces the action params for that job only, and never mutates the registry entry. An `OptionsError` returns a typed refusal that the CLI prints; the IPC variant name and shape are fixed when the story is approved.
3. The CLI form is `kawaro run ACTION --set KEY=VALUE`, repeatable. `KEY` is a field id or a param key (resolve accepts both); `VALUE` is parsed as a TOML scalar (integer, float, boolean, quoted string) and anything else is an error naming the key. `--set` with no `--wait` behaves like the current fire and forget request, with params.
4. A new CLI talking to an older running daemon must not silently drop the values: the CLI either verifies the daemon understands `params` or falls back to the in process run it already has. The exact check is decided on approval; silent fallback to defaults is forbidden.
5. `kawaro actions --id X --schema` prints `params_schema()` and exits 0; an action without options prints `{"type":"object","properties":{},"additionalProperties":false}`; an unknown id is an error that names it. `--schema` with `--json` or a file list is an error, not a silent combination.
6. No UI change, no settings key, no save of per-run values to disk. Remembering values is S-119 and a config map, not this story.

## Acceptance criteria (draft, to be fixed when the story is approved)
- AC1 (wire): `Request::Run` carries `params: Option<toml::Table>` with serde default and skip when none; a request JSON without the key parses as before; a request with a params object parses into the table; `Request::Run` round trips through serde with and without params.
- AC2 (daemon merge): a run with params resolves and executes with exactly those values (a quality of 60 in the output, or the engine receives the resolved table); the registry `ActionDef` is unchanged after the job; two consecutive runs with different values do not leak a value into the next.
- AC3 (refusals): an unknown key, a wrong type and an out of range value are refused before an engine starts, with a non zero exit and the `OptionsError` text on stderr; no output file is created.
- AC4 (CLI `--set`): `--set quality=60` and `--set max_side=1280` repeat and reach the run; `--set` without a value, with an unknown key, with a bad value or with a key for an action without options is an error naming the key and exits non zero. Values are parsed as TOML scalars, so a string field needs quotes and a bool uses true or false.
- AC5 (`--schema`): the exact JSON of `params_schema()` is printed for `image.resize-50` and for an action without options; the output parses as JSON and has `additionalProperties: false`; an unknown action id fails with a message that names it.
- AC6 (old client and daemon safety): an old client request without params works unchanged against the new daemon; a new client with `--set` against an old daemon either gets a clear version refusal or runs in process, never a silent default run; the chosen check has a test.
- AC7 (no overwrite): a customized run uses the same output naming and never replaces a file; a second identical run takes the next number.
- AC8 (gates): `cargo fmt --check`, `cargo clippy` on the touched crates with `-D warnings`, and the tests of `kawaro-core`, `kawaro-cli` and `kawaro-daemon` pass; no UI change; no new dependency; MSRV 1.85.

## Test cases (draft)
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | Serde round trip of `Request::Run` with and without params, old JSON literal pinned | unit |
| AC2 | Fake engine records the action params; two runs with different values; registry unchanged | unit |
| AC3 | Bad key, bad type and out of range through the daemon path, no output file | unit |
| AC4 | CLI argument table, stderr text and exit codes | unit |
| AC5 | Schema text for a form action and an empty action; JSON parse and the closed flag | unit |
| AC6 | Old JSON request path; new CLI against a fake old daemon answer | unit |
| AC7 | Two runs, two files, no replacement | integration |
| AC8 | The cargo gates | gate |

## Out of scope
The stage 2 UI and remembering values (S-119). MCP (S-121, S-122). `kawaro convert --set` beyond its existing `--quality`. Per-file options. A daemon settings page. Any change to the action option schema itself.

## Review log
- 2026-11-04, planner: draft from ADR 0012 decision 10 and `docs/research/agent-integration-and-expansion.md` section 5. The wire field is additive; the compatibility check for a new CLI against an older daemon is the open point to settle on approval. Not approved, not in this block.

- 2026-11-04, tester: crates/core/tests/s120_run_params.rs (7 tests) and crates/cli/tests/s120_cli_params.rs (7 tests) written first; the core red run was 11 expected compile errors, then 7/7 and 7/7 green.
- 2026-11-04, core-impl: Request::Run params, Request::Version and Response::Version, ActionDef::with_params, the daemon resolve-before-submit path, kawaro run --set and kawaro actions --id X --schema; clippy and the core, cli and daemon suites green.
- 2026-11-04, lead: docs/architecture.md IPC and CLI blocks updated, CHANGELOG bilingual line added, next free story S-125. One extra dependency line in crates/cli/Cargo.toml (toml, already in the workspace and the lock) was accepted because it adds no package.
