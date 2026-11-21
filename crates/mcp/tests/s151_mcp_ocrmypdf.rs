//! S-151: the MCP server reports `pdf.make-searchable` with the same reason
//! as `kawaro doctor`.
//!
//! The test points `PATH` at its own directory, so the answer does not depend
//! on what the machine has installed. The fake program is a file; the only
//! child the server starts is the bounded version probe of the fake.

#![cfg(unix)]

use kawaro_mcp::{Config, serve};
use serde_json::{Value, json};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The `PATH` of this process belongs to this test alone.
static ENV: Mutex<()> = Mutex::new(());

fn config_for(dir: &Path) -> Config {
    Config {
        enabled: true,
        read_only: false,
        allowed_roots: vec![dir.to_path_buf()],
        max_files: 50,
        audit: false,
        preview_px: 512,
        audit_path: None,
    }
}

/// A fake program in `bin`, executable, that prints `version`.
fn program(bin: &Path, name: &str, version: &str) -> PathBuf {
    let path = bin.join(name);
    let script = format!("#!/bin/sh\necho '{name} {version}'\n");
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// The `kawaro_formats` answer of one server run.
fn formats(bin: &Path) -> Value {
    // SAFETY: this test file holds the only tests that read `PATH`, and
    // the guard in each test keeps them one at a time.
    unsafe { std::env::set_var("PATH", bin) };
    let dir = tempfile::tempdir().unwrap();
    let config = config_for(dir.path());
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {"name": "kawaro_formats", "arguments": {}},
    });
    let input = std::io::Cursor::new(format!("{request}\n"));
    let mut out: Vec<u8> = Vec::new();
    serve(input, &mut out, &config).expect("serve");
    let text = String::from_utf8(out).expect("utf8 output");
    let line = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .expect("the server answered");
    serde_json::from_str(line).expect("the answer is JSON")
}

/// The entry of one action in the unavailable list.
fn unavailable(formats: &Value, action_id: &str) -> Value {
    let list = formats
        .pointer("/result/structuredContent/unavailable")
        .and_then(Value::as_array)
        .expect("the unavailable list");
    list.iter()
        .find(|entry| entry["action_id"] == action_id)
        .unwrap_or_else(|| panic!("{action_id} is not in the unavailable list: {list:?}"))
        .clone()
}

#[test]
fn s151_make_searchable_is_unavailable_when_ocrmypdf_is_missing() {
    let _guard = ENV.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("bin");
    std::fs::create_dir(&bin).unwrap();
    // Nothing is installed: the path holds no program at all.
    let answer = formats(&bin);
    let entry = unavailable(&answer, "pdf.make-searchable");
    assert_eq!(entry["runnable"], json!(false));
    let reason = json!({"kind": "missing-executable", "program": "ocrmypdf"});
    assert_eq!(entry["reason"], reason, "{entry}");
    assert_eq!(entry["help_id"], json!("ocrmypdf"), "{entry}");
}

#[test]
fn s151_make_searchable_is_unavailable_below_the_version_floor() {
    let _guard = ENV.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("bin");
    std::fs::create_dir(&bin).unwrap();
    // The program is there, but below the floor `kawaro doctor` refuses.
    program(&bin, "ocrmypdf", "13.0.0");
    let answer = formats(&bin);
    let entry = unavailable(&answer, "pdf.make-searchable");
    assert_eq!(entry["runnable"], json!(false));
    assert_eq!(entry["reason"], json!({"kind": "probe-unknown"}), "{entry}");
}
