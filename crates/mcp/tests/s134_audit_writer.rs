//! S-134 AC1 and AC2: the writer honours [mcp] audit_path and rotates.
//! Contract pinned here (tester): Config::from_table reads audit_path; every
//! tools/call appends one JSON line to resolved_audit_path() with parents
//! created; an audit file over 1 MiB rotates to .1 before the new line; with
//! audit = false nothing is created; without the key the default
//! <config dir>/mcp-audit.jsonl is used.

use kawaro_mcp::{Config, serve};
use serde_json::json;
use std::path::Path;
use std::sync::Mutex;

static ENV: Mutex<()> = Mutex::new(());

fn config(text: &str) -> Config {
    let table: toml::Table = toml::from_str(text).unwrap();
    Config::from_table(&table)
}

fn calls(count: usize) -> String {
    let mut input = String::new();
    for id in 0..count {
        let request = json!({
            "jsonrpc": "2.0",
            "id": id + 1,
            "method": "tools/call",
            "params": {"name": "kawaro_formats", "arguments": {}},
        });
        input.push_str(&request.to_string());
        input.push('\n');
    }
    input
}

fn serve_calls(config: &Config, count: usize) {
    let mut output = Vec::new();
    serve(std::io::Cursor::new(calls(count)), &mut output, config).expect("serve");
}

fn lines(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{} must exist: {e}", path.display()))
        .lines()
        .map(str::to_string)
        .collect()
}

#[test]
fn ac1_two_calls_write_two_lines_to_a_nested_custom_path() {
    let dir = tempfile::tempdir().unwrap();
    let audit = dir.path().join("nested").join("deep").join("audit.jsonl");
    let c = config(&format!(
        "[mcp]\nenabled = true\nread_only = false\naudit = true\naudit_path = \"{}\"\nallowed_roots = [\"{}\"]\n",
        audit.display(),
        dir.path().display()
    ));
    serve_calls(&c, 2);
    assert_eq!(lines(&audit).len(), 2, "one JSON line per tools/call");
    assert!(
        audit.parent().unwrap().is_dir(),
        "missing parents are created"
    );
}

#[test]
fn ac2_an_oversized_custom_file_rotates_to_one() {
    let dir = tempfile::tempdir().unwrap();
    let audit = dir.path().join("audit.jsonl");
    let old = vec![b'x'; 1_048_577];
    std::fs::write(&audit, &old).unwrap();
    let c = config(&format!(
        "[mcp]\nenabled = true\nread_only = false\naudit = true\naudit_path = \"{}\"\nallowed_roots = [\"{}\"]\n",
        audit.display(),
        dir.path().display()
    ));
    serve_calls(&c, 1);
    let rotated = dir.path().join("audit.jsonl.1");
    assert_eq!(
        std::fs::read(&rotated).unwrap(),
        old,
        "the old bytes move to .1"
    );
    assert_eq!(lines(&audit).len(), 1, "the new file starts with one line");
}

#[test]
fn ac2_audit_false_creates_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let audit = dir.path().join("audit.jsonl");
    let c = config(&format!(
        "[mcp]\nenabled = true\nread_only = false\naudit = false\naudit_path = \"{}\"\nallowed_roots = [\"{}\"]\n",
        audit.display(),
        dir.path().display()
    ));
    serve_calls(&c, 1);
    assert!(!audit.exists(), "audit = false writes no audit file");
}

#[test]
fn ac2_without_the_key_the_default_config_dir_path_is_used() {
    let dir = tempfile::tempdir().unwrap();
    let _lock = ENV.lock().unwrap();
    let home = std::env::var_os("HOME");
    let xdg = std::env::var_os("XDG_CONFIG_HOME");
    unsafe {
        std::env::set_var("HOME", dir.path());
        std::env::set_var("XDG_CONFIG_HOME", dir.path().join("xdg"));
    }
    let default = dir
        .path()
        .join("xdg")
        .join("kawaro")
        .join("mcp-audit.jsonl");
    let c = config(&format!(
        "[mcp]\nenabled = true\nread_only = false\naudit = true\nallowed_roots = [\"{}\"]\n",
        dir.path().display()
    ));
    serve_calls(&c, 1);
    let result = lines(&default);
    unsafe {
        match home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
        match xdg {
            Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
    }
    assert_eq!(result.len(), 1, "the default path gets the line");
}
