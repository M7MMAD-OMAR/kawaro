//! One JSON line per `tools/call` (S-121).
//!
//! The log is the only record of what a model asked this machine to do, so
//! it is written even when the call failed. It never holds file contents.

use crate::config::Config;
use serde_json::json;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Rotate to `.1` once the file is over this size.
const LIMIT_BYTES: u64 = 1024 * 1024;

/// Appends one line for one tool call. A failed write is silent: an audit
/// problem must not fail a conversion.
pub(crate) fn append(
    config: &Config,
    tool: &str,
    ok: bool,
    paths: &[String],
    outputs: &[String],
    client: &str,
    ms: u128,
) {
    if !config.audit {
        return;
    }
    let Some(file) = audit_file(config) else {
        return;
    };
    if let Some(dir) = file.parent()
        && std::fs::create_dir_all(dir).is_err()
    {
        return;
    }
    rotate(&file);
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let line = json!({
        "ts": ts,
        "tool": tool,
        "ok": ok,
        "paths": paths,
        "outputs": outputs,
        "client": client,
        "ms": ms,
    });
    if let Ok(mut handle) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file)
    {
        let _ = writeln!(handle, "{line}");
    }
}

/// The audit file: the test seam when set, otherwise `mcp-audit.jsonl` in
/// the Kawaro config folder.
fn audit_file(config: &Config) -> Option<PathBuf> {
    if let Some(path) = &config.audit_path {
        return Some(path.clone());
    }
    kawaro_core::paths::config_dir().map(|dir| dir.join("mcp-audit.jsonl"))
}

/// Renames an oversized file to `<file>.1`, dropping an older one.
fn rotate(file: &Path) {
    let over = std::fs::metadata(file)
        .map(|meta| meta.len() > LIMIT_BYTES)
        .unwrap_or(false);
    if !over {
        return;
    }
    let mut name = file.as_os_str().to_owned();
    name.push(".1");
    let _ = std::fs::rename(file, PathBuf::from(name));
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn an_oversized_file_is_rotated_and_the_new_line_goes_into_a_fresh_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("log").join("mcp-audit.jsonl");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let old = vec![b'x'; LIMIT_BYTES as usize + 1];
        std::fs::write(&path, &old).unwrap();
        let config = Config {
            audit: true,
            audit_path: Some(path.clone()),
            ..Config::default()
        };

        append(&config, "kawaro_formats", true, &[], &[], "test", 1);

        let mut rotated = path.as_os_str().to_owned();
        rotated.push(".1");
        let rotated = PathBuf::from(rotated);
        assert_eq!(std::fs::metadata(&rotated).unwrap().len(), old.len() as u64);
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect();
        assert_eq!(lines.len(), 1, "the fresh file holds only the new line");
        let line: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(line["tool"].as_str(), Some("kawaro_formats"));
    }
}
