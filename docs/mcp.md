[English](mcp.md) | [العربية](mcp.ar.md)

# Use Kawaro from a local AI app (MCP)

Kawaro can run as a local MCP server, so a local AI host (Claude Desktop,
Claude Code, Cursor, LM Studio and other hosts that start a stdio server)
can convert files with Kawaro. The file never leaves the machine: there is
no network listener, no upload and no account. The host starts
`kawaro mcp`, and every result is a new file next to the original.

The server is off by default and read only by default.

## 1. Turn it on

Edit `~/.config/kawaro/config.toml` (Linux) or
`%APPDATA%\Kawaro\config.toml` (Windows):

```toml
[mcp]
enabled = true
read_only = false
allowed_roots = ["~/Documents", "~/Pictures", "~/Downloads"]
max_files = 50
audit = true
preview_px = 512
```

- `enabled`: the server refuses to start while this is false.
- `read_only`: true allows `kawaro_formats` and `kawaro_plan` only. Set it
  to false when you want conversions.
- `allowed_roots`: only files inside these folders are read or written. An
  empty list refuses every path. `~` expands to your home folder.
- `max_files`: the most files one call may take.
- `audit`: one JSON line per tool call in `mcp-audit.jsonl` in the Kawaro
  config folder (`~/.config/kawaro` on Linux), rotated at 1 MiB. The file
  stays there even when `--config` points somewhere else.
- `preview_px`: the longest side of the PNG preview in a tool result.

## 2. Add the host

Print the snippet for the host you use:

```sh
kawaro mcp --snippet claude
kawaro mcp --snippet cursor
kawaro mcp --snippet lm-studio
kawaro mcp --snippet generic
```

The snippet is:

```json
{"mcpServers":{"kawaro":{"command":"kawaro","args":["mcp"]}}}
```

- Claude Desktop: put it in `claude_desktop_config.json` (Settings,
  Developer, Edit Config) and restart the app.
- Claude Code: `claude mcp add kawaro -- kawaro mcp`.
- Cursor: add it to `mcp.json` (Settings, MCP, Add new MCP server).
- LM Studio: add it to the app's `mcp.json` and enable it.

The host starts the server itself. You do not need the tray icon or the
daemon for MCP.

## 3. Ask for a conversion

For example:

> Combine these three images into one PDF and put it next to them.

The model calls `kawaro_images_to_pdf`. The chat gets a short text answer
with the absolute path, a small PNG preview where the host draws images, a
`file://` link and structured fields. The PDF is next to the first image.

## Tools

| Tool | What it does |
| --- | --- |
| `kawaro_formats` | The engines and actions available on this machine |
| `kawaro_plan` | What would happen, with no write: matching actions, output names, option schema |
| `kawaro_convert` | Run one action id with optional options |
| `kawaro_images_to_pdf` | Combine images into one PDF |
| `kawaro_pdf` | merge, split, rotate, compress, to_images |
| `kawaro_image` | resize, rotate, flip, grayscale, compress |

Options are the same typed options the action file declares, for example
`{"quality": 60}` or `{"dpi": 300}`.

## Safety

- The original file is never modified. A second run creates a numbered new
  file and never overwrites.
- There is no delete, overwrite or in place tool.
- Only `allowed_roots` are reachable; symlinks are resolved and a target
  outside a root is refused.
- The server has no network code and no HTTP listener.
- To stop it, set `enabled = false` or remove the host entry.

## Limits

- PDF and ZIP results get text and a link but no image preview.
- ICO and AVIF results get no preview.
- The server runs the conversion in its own process, so the tray progress
  and the end of job notification of the daemon are not used.
- Remote hosts (for example a web ChatGPT connector) are out of scope,
  because they would need the file to leave the machine.
- The 2026-07-28 MCP revision and the older initialize handshake are both
  answered; Tasks and MCP Apps are not implemented.

## Check it from a terminal

```sh
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"shell"}}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  | kawaro mcp
```

If the config is still disabled the command prints
`kawaro: MCP server is disabled: set [mcp] enabled = true in config.toml`.
