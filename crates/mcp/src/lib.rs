//! Kawaro MCP server (S-121, S-122).
//!
//! A stdio JSON-RPC 2.0 server that exposes the action registry and the
//! engines to an MCP host, so a local chat app can convert a file. It is off
//! by default: `[mcp] enabled = true` in config.toml is the only way to turn
//! it on. Nothing leaves the machine; every write tool saves one new file
//! next to the original, through kawaro-core, so the original is never
//! changed and no file is overwritten.

mod audit;
mod config;
mod preview;
mod protocol;
mod server;
mod tools;

pub use config::Config;

use std::io::{BufRead, Write};
use std::path::PathBuf;

/// Serves newline delimited JSON-RPC 2.0 over `input` and `output`.
///
/// When the config has `enabled = false` this returns an error and writes
/// nothing, so a host configuration alone cannot start the server.
pub fn serve<R: BufRead, W: Write>(
    input: R,
    output: &mut W,
    config: &Config,
) -> std::io::Result<()> {
    server::serve(input, output, config)
}

/// Runs the server on the process standard streams, reading the settings
/// from `config_path` or from the user's config.toml.
pub fn run_stdio(config_path: Option<PathBuf>) -> Result<(), String> {
    let (config, warning) = match config_path {
        Some(path) => Config::from_file(&path),
        None => match Config::default_path() {
            Some(path) => Config::from_file(&path),
            None => (Config::default(), None),
        },
    };
    if let Some(warning) = warning {
        eprintln!("kawaro mcp: {warning}");
    }
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    serve(stdin.lock(), &mut stdout.lock(), &config).map_err(|e| e.to_string())
}
