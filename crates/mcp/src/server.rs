//! The newline delimited JSON-RPC 2.0 loop.
//!
//! One line in, at most one line out. Nothing but JSON-RPC lines reaches the
//! output stream; diagnostics go to stderr. The server keeps no state beyond
//! the client name used in the audit line, so the stateless 2026-07-28
//! revision and the older handshake both work on the same loop.

use crate::audit;
use crate::config::Config;
use crate::protocol::{self, FALLBACK_VERSION, PROTOCOL_VERSIONS};
use crate::tools;
use serde_json::{Value, json};
use std::io::{BufRead, Write};
use std::time::Instant;

/// The longest request line the server keeps. A longer line is discarded
/// and answered with one parse error, so a host cannot grow the buffer
/// without bound.
const MAX_LINE: usize = 1024 * 1024;

/// A line read from the input, at most [`MAX_LINE`] bytes long.
enum Line {
    /// The input ended.
    End,
    /// A complete line, without its newline.
    Text(Vec<u8>),
    /// The line was over the cap and the rest of it was dropped.
    TooLong,
}

/// Serves the loop until the input closes.
pub(crate) fn serve<R: BufRead, W: Write>(
    mut input: R,
    output: &mut W,
    config: &Config,
) -> std::io::Result<()> {
    if !config.enabled {
        return Err(std::io::Error::other(
            "MCP server is disabled: set [mcp] enabled = true in config.toml",
        ));
    }
    let mut session = Session::default();
    loop {
        match read_line(&mut input, MAX_LINE)? {
            Line::End => break,
            Line::TooLong => {
                let response = protocol_error(
                    Value::Null,
                    -32700,
                    "parse error: the request line is over 1 MiB",
                );
                write_response(output, &response)?;
            }
            Line::Text(bytes) => {
                let text = match std::str::from_utf8(&bytes) {
                    Ok(text) => text.trim(),
                    Err(_) => {
                        let response = protocol_error(
                            Value::Null,
                            -32700,
                            "parse error: the request line is not UTF-8",
                        );
                        write_response(output, &response)?;
                        continue;
                    }
                };
                if text.is_empty() {
                    continue;
                }
                if let Some(response) = session.handle(text, config) {
                    write_response(output, &response)?;
                }
            }
        }
    }
    Ok(())
}

/// Reads one line, keeping at most `cap` bytes of it. A longer line is read
/// through its newline and dropped in bounded chunks, so the buffer never
/// holds more than `cap` bytes of it.
fn read_line<R: BufRead>(input: &mut R, cap: usize) -> std::io::Result<Line> {
    let mut kept: Vec<u8> = Vec::new();
    loop {
        let available = match input.fill_buf() {
            Ok(buffer) => buffer,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        if available.is_empty() {
            return Ok(if kept.is_empty() {
                Line::End
            } else {
                Line::Text(kept)
            });
        }
        let split = available.iter().position(|&byte| byte == b'\n');
        let content_len = split.unwrap_or(available.len());
        let consume = content_len + usize::from(split.is_some());
        if kept.len() + content_len > cap {
            input.consume(consume);
            if split.is_none() {
                discard_line(input)?;
            }
            return Ok(Line::TooLong);
        }
        kept.extend_from_slice(&available[..content_len]);
        input.consume(consume);
        if split.is_some() {
            return Ok(Line::Text(kept));
        }
    }
}

/// Drops the rest of a line that is already over the cap.
fn discard_line<R: BufRead>(input: &mut R) -> std::io::Result<()> {
    loop {
        let available = match input.fill_buf() {
            Ok(buffer) => buffer,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        if available.is_empty() {
            return Ok(());
        }
        match available.iter().position(|&byte| byte == b'\n') {
            Some(index) => {
                input.consume(index + 1);
                return Ok(());
            }
            None => {
                let len = available.len();
                input.consume(len);
            }
        }
    }
}

/// Writes one JSON-RPC line and flushes it.
fn write_response<W: Write>(output: &mut W, response: &Value) -> std::io::Result<()> {
    serde_json::to_writer(&mut *output, response).map_err(std::io::Error::other)?;
    output.write_all(b"\n")?;
    output.flush()
}

/// What a connection remembers: the client name for the audit line.
#[derive(Default)]
struct Session {
    client: String,
}

impl Session {
    /// Answers one request line. `None` means a notification, which gets no answer.
    fn handle(&mut self, text: &str, config: &Config) -> Option<Value> {
        let request: Value = match serde_json::from_str(text) {
            Ok(value) => value,
            Err(e) => {
                return Some(protocol_error(
                    Value::Null,
                    -32700,
                    &format!("parse error: {e}"),
                ));
            }
        };
        let Some(object) = request.as_object() else {
            return Some(protocol_error(
                Value::Null,
                -32700,
                "parse error: the request is not a JSON object",
            ));
        };
        let Some(method) = object.get("method").and_then(Value::as_str) else {
            return Some(protocol_error(
                Value::Null,
                -32700,
                "parse error: the request has no method",
            ));
        };
        // A notification has no id and is never answered.
        let id = object.get("id").cloned()?;
        let params = object.get("params").cloned().unwrap_or(Value::Null);
        if let Some(name) = params
            .pointer("/_meta/io.modelcontextprotocol/clientInfo/name")
            .and_then(Value::as_str)
        {
            self.client = name.to_string();
        }
        let outcome = match method {
            "initialize" => Ok(self.initialize(&params)),
            "server/discover" => Ok(discover()),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(tools::list()),
            "tools/call" => self.call(&params, config),
            _ => Err((-32601, "method not found".to_string())),
        };
        Some(match outcome {
            Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
            Err((code, message)) => protocol_error(id, code, &message),
        })
    }

    /// The legacy handshake. The requested revision is echoed when this build
    /// knows it, otherwise the newest one every host in the matrix accepts.
    fn initialize(&mut self, params: &Value) -> Value {
        if let Some(name) = params.pointer("/clientInfo/name").and_then(Value::as_str) {
            self.client = name.to_string();
        }
        let requested = params.get("protocolVersion").and_then(Value::as_str);
        let version = requested
            .filter(|version| PROTOCOL_VERSIONS.contains(version))
            .unwrap_or(FALLBACK_VERSION);
        json!({
            "protocolVersion": version,
            "capabilities": protocol::capabilities(),
            "serverInfo": protocol::server_info(),
            "instructions": protocol::INSTRUCTIONS,
        })
    }

    /// `tools/call`: check the argument names, run the tool, then write the
    /// audit line (one line per call, S-121 AC11).
    fn call(&mut self, params: &Value, config: &Config) -> Result<Value, (i64, String)> {
        let Some(name) = params.get("name").and_then(Value::as_str) else {
            return Err((-32602, "tools/call needs a tool name".to_string()));
        };
        let args = match params.get("arguments") {
            Some(arguments) => arguments.clone(),
            None => json!({}),
        };
        let started = Instant::now();
        // The input schema is host side only, so the server checks the
        // argument names itself before the tool runs.
        let result = tools::reject_unknown_arguments(name, &args)
            .or_else(|| tools::call(name, &args, config));
        let Some(result) = result else {
            // A call for a name outside the six is still a tools/call: the
            // answer is a JSON-RPC error and the audit still gets its line.
            if config.audit {
                audit::append(
                    config,
                    name,
                    false,
                    &tools::requested_paths(&args),
                    &[],
                    &self.client,
                    started.elapsed().as_millis(),
                );
            }
            return Err((-32602, format!("unknown tool `{name}`")));
        };
        if config.audit {
            let ok = !result
                .get("isError")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            audit::append(
                config,
                name,
                ok,
                &tools::requested_paths(&args),
                &tools::output_paths(&result),
                &self.client,
                started.elapsed().as_millis(),
            );
        }
        Ok(result)
    }
}

/// `server/discover`: the stateless revision's version and identity answer.
fn discover() -> Value {
    json!({
        "resultType": "complete",
        "protocolVersions": protocol::DISCOVER_VERSIONS,
        "capabilities": protocol::capabilities(),
        "serverInfo": protocol::server_info(),
        "instructions": protocol::INSTRUCTIONS,
    })
}

/// A JSON-RPC error object.
fn protocol_error(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": code, "message": message},
    })
}
#[cfg(test)]
mod tests {
    use super::*;

    fn served_config() -> Config {
        Config {
            enabled: true,
            audit: false,
            ..Config::default()
        }
    }

    #[test]
    fn an_oversized_line_gets_one_parse_error_and_the_loop_continues() {
        let mut input = vec![b'x'; MAX_LINE + 10];
        input.push(b'\n');
        input.extend_from_slice(b"{\"jsonrpc\":\"2.0\",\"id\":9,\"method\":\"ping\"}\n");
        let mut output: Vec<u8> = Vec::new();
        serve(std::io::Cursor::new(input), &mut output, &served_config()).unwrap();
        let text = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2, "{text}");
        let first: Value = serde_json::from_str(lines[0]).unwrap();
        assert!(first["id"].is_null(), "{first}");
        assert_eq!(first["error"]["code"].as_i64(), Some(-32700));
        let second: Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(second["id"].as_i64(), Some(9));
        assert!(second["result"].is_object());
    }
}
