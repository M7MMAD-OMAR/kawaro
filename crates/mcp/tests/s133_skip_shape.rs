//! S-133 AC3: MCP turns a skipped compress into a successful result with an
//! empty outputs list and a skipped list.

use kawaro_mcp::{Config, serve};
use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;

fn gs_available() -> bool {
    Command::new("gs")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn config_for(dir: &Path) -> Config {
    Config {
        enabled: true,
        read_only: false,
        allowed_roots: vec![dir.to_path_buf()],
        max_files: 50,
        audit: false,
        preview_px: 256,
        audit_path: None,
    }
}

#[test]
fn ac3_a_skipped_compress_is_not_an_error_in_mcp() {
    if !gs_available() {
        eprintln!("skipped: Ghostscript is not installed");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/docs/one-page.pdf");
    let pdf = dir.path().join("one-page.pdf");
    std::fs::copy(fixture, &pdf).unwrap();
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "kawaro_convert",
            "arguments": {
                "action": "pdf.compress",
                "paths": [pdf.to_str().unwrap()],
                "options": { "preset": "screen", "dpi": 72 },
                "preview": false,
            },
        },
    });
    let mut input = request.to_string();
    input.push('\n');
    let mut output = Vec::new();
    serve(
        std::io::Cursor::new(input),
        &mut output,
        &config_for(dir.path()),
    )
    .expect("serve");
    let text = String::from_utf8(output).unwrap();
    let line: Value = serde_json::from_str(text.lines().next().expect("one response")).unwrap();
    let result = &line["result"];
    assert_eq!(result["isError"].as_bool(), Some(false), "{result}");
    let structured = &result["structuredContent"];
    assert_eq!(
        structured["outputs"].as_array().map(Vec::len),
        Some(0),
        "a skip writes no output"
    );
    let skipped = structured["skipped"].as_array().expect("a skipped list");
    assert_eq!(skipped.len(), 1, "{structured}");
    assert_eq!(skipped[0]["reason"].as_str(), Some("already-optimized"));
    assert!(
        skipped[0]["path"]
            .as_str()
            .unwrap_or("")
            .ends_with("one-page.pdf")
    );
    let text = result["content"][0]["text"].as_str().unwrap_or("");
    assert!(
        text.to_lowercase().contains("already optimized"),
        "the text says why: {text}"
    );
    assert!(!dir.path().join("one-page-compressed.pdf").exists());
}
