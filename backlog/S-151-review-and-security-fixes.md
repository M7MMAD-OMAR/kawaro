# S-151: Review and security fixes for S-142 to S-150

Status: DONE (source, fmt, clippy, workspace tests and Windows cross stages on the second machine, 2026-11-22). No visual, compositor or real desktop check.

## Why

Three fresh read-only reviews of `fc47ca5..e5fd582` (Gemini 4 Neon on the UI and daemon, MiniMax-M3.1-Flash on core, engines, CLI and MCP, Claude Haiku 5.5 as a source-first security review) found defects in the S-142 to S-150 source. Findings and states: `docs/development/2026-11-21-review-findings.md`, section "Second review (2026-11-22)".

## Acceptance criteria

1. A Tesseract or OCRmyPDF installed after the daemon started is seen after a reload by the hive, the queue and the planner alike; availability comes only from the engine instances the queue runs.
2. An OCRmyPDF below the version floor is shown unavailable with its reason in the hive, the drop hive and MCP, not offered and failing.
3. A per-page timeout warning anywhere in OCRmyPDF stderr fails the job.
4. A retry never holds the daemon state lock during file system calls, and a double trigger starts one child job.
5. The Results page follows the language the settings window shows.
6. A drop on the More cell leaves the hive in click-to-choose mode; paging never drops an action when the outer ring has one slot or the Windows hive sits at a monitor edge.
7. `image.to-pdf` offers no preview; a successful preset Apply replaces the pending message.
8. `kawaro results` escapes control characters in names and reasons; the socket never lives in a shared directory and is refused in a directory another user owns or can write; the cross-device output copy never writes through a symlink.

## Tests

`crates/engines/tests/s151_engine_refresh.rs`, `crates/core/tests/s151_socket_dir.rs`, `crates/mcp/tests/s151_mcp_ocrmypdf.rs`, `crates/ui/tests/s151_ui_fixes.rs`, and `s151` unit tests in the daemon, office, layer and results sources.
