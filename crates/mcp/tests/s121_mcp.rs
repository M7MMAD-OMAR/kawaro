//! S-121 and S-122: the local MCP server (`crates/mcp`).
//!
//! Newline delimited JSON-RPC 2.0 on stdin and stdout, no network, no
//! daemon: the tests drive `serve` with a Cursor the way a host does, over
//! real temporary files, and check the tools, the error codes, the preview
//! block and the audit line.

use kawaro_mcp::{Config, serve};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const INSTRUCTIONS: &str = "Local file conversion and PDF tools. Nothing leaves the machine.";
const VERSIONS: [&str; 5] = [
    "2026-07-28",
    "2025-11-25",
    "2025-06-18",
    "2025-03-26",
    "2024-11-05",
];

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

fn write_photo(path: &Path) {
    let image = image::RgbImage::from_pixel(4, 4, image::Rgb([200, 120, 40]));
    image
        .save(path)
        .unwrap_or_else(|e| panic!("save {}: {e}", path.display()));
}

struct Fixture {
    dir: tempfile::TempDir,
    photo: PathBuf,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let photo = dir.path().join("photo.jpg");
    write_photo(&photo);
    Fixture { dir, photo }
}

fn entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// Serializes the requests as newline delimited JSON, runs `serve` and parses
/// every output line as a JSON value.
fn serve_text(input: &str, cfg: &Config) -> Vec<Value> {
    let mut out: Vec<u8> = Vec::new();
    serve(std::io::Cursor::new(input), &mut out, cfg).expect("serve");
    let text = String::from_utf8(out).expect("utf8 output");
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("not JSON: {e}\n{l}")))
        .collect()
}

fn run(lines: &[Value], cfg: &Config) -> Vec<Value> {
    let mut input = String::new();
    for line in lines {
        input.push_str(&line.to_string());
        input.push('\n');
    }
    serve_text(&input, cfg)
}

fn result_for(out: &[Value], id: i64) -> &Value {
    out.iter()
        .find(|v| v.get("id").and_then(Value::as_i64) == Some(id))
        .unwrap_or_else(|| panic!("no response with id {id}: {out:?}"))
}

fn tool_result(out: &[Value], id: i64) -> &Value {
    &result_for(out, id)["result"]
}

fn call(id: i64, tool: &str, args: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": { "name": tool, "arguments": args },
    })
}

fn initialize(id: i64, version: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "initialize",
        "params": {
            "protocolVersion": version,
            "capabilities": {},
            "clientInfo": { "name": "test-client", "version": "1.0" },
        },
    })
}

fn error_code(result: &Value) -> &str {
    result["structuredContent"]["error"]["code"]
        .as_str()
        .unwrap_or("")
}

#[test]
fn default_config_matches_the_contract() {
    let c = Config::default();
    assert!(!c.enabled);
    assert!(c.read_only);
    assert!(c.allowed_roots.is_empty());
    assert_eq!(c.max_files, 50);
    assert!(c.audit);
    assert_eq!(c.preview_px, 512);
    assert!(c.audit_path.is_none());
}

#[test]
fn resolved_roots_canonicalize_and_drop_missing_paths() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("not-a-dir.txt");
    std::fs::write(&file, "x").unwrap();
    let cfg = Config {
        allowed_roots: vec![dir.path().to_path_buf(), dir.path().join("missing"), file],
        ..Config::default()
    };
    assert_eq!(
        cfg.resolved_roots(),
        vec![dir.path().canonicalize().unwrap()]
    );
}

#[test]
fn disabled_server_returns_an_error_and_writes_nothing() {
    let fx = fixture();
    let mut cfg = config_for(fx.dir.path());
    cfg.enabled = false;
    let mut out: Vec<u8> = Vec::new();
    let err = serve(std::io::Cursor::new(""), &mut out, &cfg).unwrap_err();
    assert!(err.to_string().contains("disabled"), "{err}");
    assert!(out.is_empty(), "the disabled server wrote to stdout");
}

#[test]
fn initialize_answers_with_server_info_and_a_supported_version() {
    let fx = fixture();
    let cfg = config_for(fx.dir.path());
    let mut lines: Vec<Value> = VERSIONS
        .iter()
        .enumerate()
        .map(|(i, v)| initialize(i as i64 + 1, v))
        .collect();
    lines.push(initialize(99, "1999-01-01"));
    let out = run(&lines, &cfg);

    for (i, version) in VERSIONS.iter().enumerate() {
        let r = tool_result(&out, i as i64 + 1);
        assert_eq!(r["protocolVersion"].as_str(), Some(*version), "{version}");
    }
    // An unknown requested version falls back to the current one.
    assert_eq!(
        tool_result(&out, 99)["protocolVersion"].as_str(),
        Some("2025-11-25")
    );

    let r = tool_result(&out, 1);
    assert_eq!(r["serverInfo"]["name"].as_str(), Some("kawaro"));
    assert_eq!(
        r["serverInfo"]["version"].as_str(),
        Some(env!("CARGO_PKG_VERSION"))
    );
    assert!(r["capabilities"]["tools"].is_object());
    assert_eq!(r["instructions"].as_str(), Some(INSTRUCTIONS));
}

#[test]
fn discover_advertises_protocol_versions_and_only_tools() {
    let fx = fixture();
    let cfg = config_for(fx.dir.path());
    let out = run(
        &[json!({ "jsonrpc": "2.0", "id": 1, "method": "server/discover" })],
        &cfg,
    );
    let r = tool_result(&out, 1);
    assert_eq!(r["resultType"].as_str(), Some("complete"));
    let versions: Vec<&str> = r["protocolVersions"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(versions, ["2026-07-28", "2025-11-25", "2025-06-18"]);
    let caps = r["capabilities"].as_object().unwrap();
    assert_eq!(caps.keys().collect::<Vec<_>>(), ["tools"]);
    assert_eq!(r["serverInfo"]["name"].as_str(), Some("kawaro"));
    assert_eq!(r["instructions"].as_str(), Some(INSTRUCTIONS));
}

#[test]
fn tools_list_is_deterministic_and_declares_the_six_tools() {
    let fx = fixture();
    let cfg = config_for(fx.dir.path());
    let out = run(
        &[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" }),
            json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }),
        ],
        &cfg,
    );
    let first = tool_result(&out, 1);
    let second = tool_result(&out, 2);
    assert_eq!(first, second, "tools/list order is stable");
    assert_eq!(first["resultType"].as_str(), Some("complete"));
    assert_eq!(first["ttlMs"].as_u64(), Some(300_000));
    assert_eq!(first["cacheScope"].as_str(), Some("private"));
    let names: Vec<&str> = first["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "kawaro_formats",
            "kawaro_plan",
            "kawaro_convert",
            "kawaro_images_to_pdf",
            "kawaro_pdf",
            "kawaro_image",
        ]
    );
}

#[test]
fn notifications_get_no_response_and_ping_is_empty() {
    let fx = fixture();
    let cfg = config_for(fx.dir.path());
    let out = run(
        &[
            json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
            json!({ "jsonrpc": "2.0", "method": "notifications/cancelled", "params": { "requestId": 3 } }),
            json!({ "jsonrpc": "2.0", "id": 7, "method": "ping" }),
        ],
        &cfg,
    );
    assert_eq!(out.len(), 1, "notifications get no response: {out:?}");
    assert_eq!(out[0]["id"].as_i64(), Some(7));
    assert!(out[0]["result"].is_object());
    assert!(out[0].get("error").is_none());
}

#[test]
fn unknown_method_is_method_not_found() {
    let fx = fixture();
    let cfg = config_for(fx.dir.path());
    let out = run(
        &[json!({ "jsonrpc": "2.0", "id": 3, "method": "no/such/method" })],
        &cfg,
    );
    assert_eq!(out.len(), 1);
    assert_eq!(out[0]["id"].as_i64(), Some(3));
    assert_eq!(out[0]["error"]["code"].as_i64(), Some(-32601));
    assert_eq!(
        out[0]["error"]["message"].as_str(),
        Some("method not found")
    );
}

#[test]
fn malformed_line_is_a_parse_error_with_a_null_id() {
    let fx = fixture();
    let cfg = config_for(fx.dir.path());
    let out = serve_text("{not json\n", &cfg);
    assert_eq!(out.len(), 1, "one response line for the malformed line");
    assert!(out[0].get("id").is_some(), "the id key must be present");
    assert!(out[0]["id"].is_null(), "the id is null");
    assert_eq!(out[0]["error"]["code"].as_i64(), Some(-32700));
}

#[test]
fn formats_lists_version_engines_and_actions() {
    let fx = fixture();
    let cfg = config_for(fx.dir.path());
    let out = run(&[call(1, "kawaro_formats", json!({}))], &cfg);
    let r = tool_result(&out, 1);
    assert_eq!(r["isError"].as_bool(), Some(false), "{r}");
    assert_eq!(r["resultType"].as_str(), Some("complete"));
    let sc = &r["structuredContent"];
    assert_eq!(sc["version"].as_str(), Some(env!("CARGO_PKG_VERSION")));
    let versions: Vec<&str> = sc["protocolVersions"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(versions.contains(&"2026-07-28"), "{versions:?}");
    let engines = sc["engines"].as_array().unwrap();
    assert!(
        engines
            .iter()
            .any(|e| e["id"] == "image" && e["available"] == true),
        "{engines:?}"
    );
    let actions = sc["actions"].as_array().unwrap();
    let webp = actions
        .iter()
        .find(|a| a["id"] == "image.to-webp")
        .expect("image.to-webp is listed");
    assert_eq!(webp["engine"].as_str(), Some("image"));
    assert_eq!(webp["ring"].as_u64(), Some(1));
    assert_eq!(webp["has_options"].as_bool(), Some(true));
}

#[test]
fn plan_lists_matching_actions_and_writes_nothing() {
    let fx = fixture();
    let cfg = config_for(fx.dir.path());
    let before = entries(fx.dir.path());
    let out = run(
        &[call(
            1,
            "kawaro_plan",
            json!({ "paths": [fx.photo.to_str().unwrap()] }),
        )],
        &cfg,
    );
    let r = tool_result(&out, 1);
    assert_eq!(r["isError"].as_bool(), Some(false), "{r}");
    let sc = &r["structuredContent"];
    let actions = sc["actions"].as_array().unwrap();
    let webp = actions
        .iter()
        .find(|a| a["id"] == "image.to-webp")
        .unwrap_or_else(|| panic!("image.to-webp missing: {actions:?}"));
    assert_eq!(webp["ring"].as_u64(), Some(1));
    assert_eq!(webp["engine"].as_str(), Some("image"));
    assert_eq!(webp["label"].as_str(), Some("Convert to WebP"));
    assert_eq!(
        webp["options"]["properties"]["quality"]["type"].as_str(),
        Some("integer")
    );
    let outputs = webp["outputs"].as_array().unwrap();
    assert_eq!(outputs.len(), 1);
    let final_path = Path::new(outputs[0].as_str().unwrap());
    assert!(final_path.is_absolute(), "{final_path:?}");
    assert_eq!(final_path.file_name().unwrap(), "photo.webp");
    assert_eq!(entries(fx.dir.path()), before, "plan must not write");
    assert!(!fx.dir.path().join("photo.webp").exists());
}

#[test]
fn convert_image_to_webp_uses_the_options_and_previews_the_output() {
    let fx = fixture();
    let cfg = config_for(fx.dir.path());
    let out = run(
        &[call(
            1,
            "kawaro_convert",
            json!({
                "action": "image.to-webp",
                "paths": [fx.photo.to_str().unwrap()],
                "options": { "quality": 60 },
            }),
        )],
        &cfg,
    );
    let r = tool_result(&out, 1);
    assert_eq!(r["isError"].as_bool(), Some(false), "{r}");
    assert_eq!(r["resultType"].as_str(), Some("complete"));
    let sc = &r["structuredContent"];
    assert_eq!(sc["action"].as_str(), Some("image.to-webp"));
    assert_eq!(sc["engine"].as_str(), Some("image"));
    assert_eq!(sc["params"]["quality"].as_i64(), Some(60));
    assert_eq!(sc["overwritten"].as_bool(), Some(false));
    assert!(sc["elapsed_ms"].is_number(), "{sc}");

    let outputs = sc["outputs"].as_array().unwrap();
    assert_eq!(outputs.len(), 1);
    let path = PathBuf::from(outputs[0]["path"].as_str().unwrap());
    assert_eq!(Path::new(&path).file_name().unwrap(), "photo.webp");
    assert!(outputs[0]["bytes"].as_u64().unwrap() > 0);
    assert!(outputs[0]["path"].as_str().unwrap().ends_with("photo.webp"));
    assert!(Path::new(&path).is_file(), "the webp is next to the photo");

    let content = r["content"].as_array().unwrap();
    assert_eq!(content[0]["type"].as_str(), Some("text"));
    let text = content[0]["text"].as_str().unwrap();
    assert!(text.contains("next to the originals"), "{text}");
    assert!(text.contains(path.to_str().unwrap()), "{text}");
    let preview = content
        .iter()
        .find(|c| c["type"] == "image")
        .expect("an image preview block");
    assert_eq!(preview["mimeType"].as_str(), Some("image/png"));
    assert!(!preview["data"].as_str().unwrap().is_empty());
    let link = content
        .iter()
        .find(|c| c["type"] == "resource_link")
        .expect("a resource link per output");
    assert!(link["uri"].as_str().unwrap().starts_with("file://"));
    assert_eq!(link["name"].as_str(), Some("photo.webp"));
    assert!(link["mimeType"].as_str().unwrap().starts_with("image/"));
}

#[test]
fn a_second_convert_does_not_overwrite_the_first_output() {
    let fx = fixture();
    let cfg = config_for(fx.dir.path());
    let arguments = json!({
        "action": "image.to-webp",
        "paths": [fx.photo.to_str().unwrap()],
        "options": { "quality": 60 },
    });
    let out = run(
        &[
            call(1, "kawaro_convert", arguments.clone()),
            call(2, "kawaro_convert", arguments),
        ],
        &cfg,
    );
    let first = tool_result(&out, 1);
    let second = tool_result(&out, 2);
    assert_eq!(first["isError"].as_bool(), Some(false), "{first}");
    assert_eq!(second["isError"].as_bool(), Some(false), "{second}");
    assert_eq!(
        second["structuredContent"]["overwritten"].as_bool(),
        Some(false)
    );
    let first_path = first["structuredContent"]["outputs"][0]["path"]
        .as_str()
        .unwrap()
        .to_string();
    let second_path = second["structuredContent"]["outputs"][0]["path"]
        .as_str()
        .unwrap()
        .to_string();
    assert_ne!(first_path, second_path, "the second run takes a free name");
    assert!(Path::new(&first_path).is_file());
    assert!(Path::new(&second_path).is_file());
}

#[test]
fn paths_outside_allowed_roots_are_refused() {
    let allowed = fixture();
    let outside = fixture();
    let cfg = config_for(allowed.dir.path());
    let out = run(
        &[call(
            1,
            "kawaro_plan",
            json!({ "paths": [outside.photo.to_str().unwrap()] }),
        )],
        &cfg,
    );
    let r = tool_result(&out, 1);
    assert_eq!(r["isError"].as_bool(), Some(true), "{r}");
    let text = r["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("OUTSIDE_ROOTS"), "{text}");
    assert_eq!(error_code(r), "OUTSIDE_ROOTS");
}

#[test]
fn read_only_allows_reads_and_refuses_writes() {
    let fx = fixture();
    let mut cfg = config_for(fx.dir.path());
    cfg.read_only = true;
    let photo = fx.photo.to_str().unwrap();
    let out = run(
        &[
            call(1, "kawaro_formats", json!({})),
            call(2, "kawaro_plan", json!({ "paths": [photo] })),
            call(
                3,
                "kawaro_convert",
                json!({ "action": "image.to-webp", "paths": [photo] }),
            ),
        ],
        &cfg,
    );
    assert_eq!(tool_result(&out, 1)["isError"].as_bool(), Some(false));
    assert_eq!(tool_result(&out, 2)["isError"].as_bool(), Some(false));
    let r = tool_result(&out, 3);
    assert_eq!(r["isError"].as_bool(), Some(true), "{r}");
    assert!(
        r["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("READ_ONLY")
    );
    assert_eq!(error_code(r), "READ_ONLY");
    assert!(!fx.dir.path().join("photo.webp").exists());
}

#[test]
fn selections_over_max_files_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let paths: Vec<String> = (0..3)
        .map(|i| {
            let path = dir.path().join(format!("photo{i}.jpg"));
            write_photo(&path);
            path.to_str().unwrap().to_string()
        })
        .collect();
    let mut cfg = config_for(dir.path());
    cfg.max_files = 2;
    let out = run(&[call(1, "kawaro_plan", json!({ "paths": paths }))], &cfg);
    let r = tool_result(&out, 1);
    assert_eq!(r["isError"].as_bool(), Some(true), "{r}");
    let text = r["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("TOO_MANY_FILES"), "{text}");
    assert_eq!(error_code(r), "TOO_MANY_FILES");
}

#[test]
fn audit_writes_one_json_line_per_tool_call() {
    let fx = fixture();
    let audit_path = fx.dir.path().join("logs").join("mcp-audit.jsonl");
    let mut cfg = config_for(fx.dir.path());
    cfg.audit = true;
    cfg.audit_path = Some(audit_path.clone());
    let out = run(
        &[
            initialize(1, "2025-11-25"),
            call(2, "kawaro_formats", json!({})),
        ],
        &cfg,
    );
    assert_eq!(out.len(), 2);
    assert!(audit_path.is_file(), "the audit file is created");
    let text = std::fs::read_to_string(&audit_path).unwrap();
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 1, "one line per tools/call, not per request");
    let line: Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(line["tool"].as_str(), Some("kawaro_formats"));
    assert_eq!(line["client"].as_str(), Some("test-client"));
    assert_eq!(line["ok"].as_bool(), Some(true));
    assert!(line["ts"].is_u64(), "{line}");
    assert!(line["ms"].is_number(), "{line}");
    assert!(line["paths"].is_array(), "{line}");
    assert!(line["outputs"].is_array(), "{line}");
}

// Tests the reviewer found missing from the S-121 and S-122 acceptance
// criteria: config parsing from a file, the refusal order, every write tool
// gate, the named tool mapping, the preview switch, the failure shape, the
// write audit line and the protocol errors outside the four methods.

const ONE_PAGE_PDF: &[u8] = include_bytes!("../../../tests/fixtures/docs/one-page.pdf");

fn write_fixture_pdf(path: &Path) {
    std::fs::write(path, ONE_PAGE_PDF).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
}

fn failure_text(result: &Value) -> &str {
    result["content"][0]["text"].as_str().unwrap_or("")
}

fn output_path(result: &Value) -> String {
    result["structuredContent"]["outputs"][0]["path"]
        .as_str()
        .unwrap_or("")
        .to_string()
}

#[test]
fn config_from_file_reads_the_mcp_table_and_a_missing_file_gives_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let text = format!(
        "[mcp]\nenabled = true\nread_only = false\nallowed_roots = [\"{}\"]\nmax_files = 3\naudit = false\npreview_px = 64\n",
        dir.path().display()
    );
    std::fs::write(&path, text).unwrap();
    let (cfg, warning) = Config::from_file(&path);
    assert!(warning.is_none(), "{warning:?}");
    assert!(cfg.enabled);
    assert!(!cfg.read_only);
    assert_eq!(cfg.allowed_roots, vec![dir.path().to_path_buf()]);
    assert_eq!(cfg.max_files, 3);
    assert!(!cfg.audit);
    assert_eq!(cfg.preview_px, 64);
    assert!(cfg.audit_path.is_none());

    let (missing, warning) = Config::from_file(&dir.path().join("absent.toml"));
    assert!(warning.is_none(), "{warning:?}");
    assert!(!missing.enabled);
    assert!(missing.read_only);
    assert_eq!(missing.max_files, 50);
}

#[test]
fn refusal_order_checks_the_paths_before_read_only() {
    let inside = fixture();
    let outside = fixture();
    let mut cfg = config_for(inside.dir.path());
    cfg.read_only = true;
    let missing = inside.dir.path().join("absent.jpg");
    let out = run(
        &[
            call(
                1,
                "kawaro_convert",
                json!({
                    "action": "image.to-png",
                    "paths": [outside.photo.to_str().unwrap()],
                }),
            ),
            call(
                2,
                "kawaro_convert",
                json!({
                    "action": "image.to-png",
                    "paths": [missing.to_str().unwrap()],
                }),
            ),
            call(
                3,
                "kawaro_convert",
                json!({
                    "action": "image.to-png",
                    "paths": [inside.photo.to_str().unwrap()],
                }),
            ),
        ],
        &cfg,
    );
    assert_eq!(
        error_code(tool_result(&out, 1)),
        "OUTSIDE_ROOTS",
        "{:?}",
        tool_result(&out, 1)
    );
    assert_eq!(
        error_code(tool_result(&out, 2)),
        "OUTSIDE_ROOTS",
        "{:?}",
        tool_result(&out, 2)
    );
    assert_eq!(
        error_code(tool_result(&out, 3)),
        "READ_ONLY",
        "{:?}",
        tool_result(&out, 3)
    );
}

#[test]
fn unknown_action_is_reported_before_the_options() {
    let fx = fixture();
    let cfg = config_for(fx.dir.path());
    let out = run(
        &[call(
            1,
            "kawaro_convert",
            json!({
                "action": "no.such.action",
                "paths": [fx.photo.to_str().unwrap()],
                "options": {"x": 1},
            }),
        )],
        &cfg,
    );
    assert_eq!(error_code(tool_result(&out, 1)), "UNKNOWN_ACTION");
}

#[test]
fn an_unknown_pdf_op_is_bad_options_even_in_read_only() {
    let fx = fixture();
    let pdf = fx.dir.path().join("doc.pdf");
    write_fixture_pdf(&pdf);
    let arguments = json!({
        "op": "bogus",
        "paths": [pdf.to_str().unwrap()],
        "preview": false,
    });
    let out = run(
        &[call(1, "kawaro_pdf", arguments.clone())],
        &config_for(fx.dir.path()),
    );
    let r = tool_result(&out, 1);
    assert_eq!(error_code(r), "BAD_OPTIONS", "{r}");
    assert!(failure_text(r).contains("merge"), "{}", failure_text(r));

    let mut read_only = config_for(fx.dir.path());
    read_only.read_only = true;
    let out = run(&[call(1, "kawaro_pdf", arguments)], &read_only);
    assert_eq!(error_code(tool_result(&out, 1)), "BAD_OPTIONS");
}

#[test]
fn unknown_top_level_arguments_are_bad_options() {
    let fx = fixture();
    let cfg = config_for(fx.dir.path());
    let photo = fx.photo.to_str().unwrap();
    let out = run(
        &[
            call(
                1,
                "kawaro_convert",
                json!({"action": "image.to-webp", "paths": [photo], "out": "x.webp"}),
            ),
            call(
                2,
                "kawaro_convert",
                json!({"action": "image.to-webp", "paths": [photo], "in_place": true}),
            ),
        ],
        &cfg,
    );
    assert_eq!(error_code(tool_result(&out, 1)), "BAD_OPTIONS");
    assert_eq!(error_code(tool_result(&out, 2)), "BAD_OPTIONS");
}

#[test]
fn missing_and_directory_paths_are_outside_roots() {
    let fx = fixture();
    let cfg = config_for(fx.dir.path());
    let missing = fx.dir.path().join("absent.jpg");
    let out = run(
        &[
            call(
                1,
                "kawaro_plan",
                json!({"paths": [missing.to_str().unwrap()]}),
            ),
            call(
                2,
                "kawaro_plan",
                json!({"paths": [fx.dir.path().to_str().unwrap()]}),
            ),
        ],
        &cfg,
    );
    assert_eq!(error_code(tool_result(&out, 1)), "OUTSIDE_ROOTS");
    assert_eq!(error_code(tool_result(&out, 2)), "OUTSIDE_ROOTS");
}

#[test]
fn all_write_tools_refuse_in_read_only() {
    let fx = fixture();
    let second = fx.dir.path().join("second.jpg");
    write_photo(&second);
    let pdf = fx.dir.path().join("doc.pdf");
    write_fixture_pdf(&pdf);
    let mut cfg = config_for(fx.dir.path());
    cfg.read_only = true;
    let photo = fx.photo.to_str().unwrap();
    let out = run(
        &[
            call(
                1,
                "kawaro_convert",
                json!({"action": "image.to-png", "paths": [photo]}),
            ),
            call(
                2,
                "kawaro_images_to_pdf",
                json!({"paths": [photo, second.to_str().unwrap()], "preview": false}),
            ),
            call(
                3,
                "kawaro_pdf",
                json!({"op": "rotate", "paths": [pdf.to_str().unwrap()], "preview": false}),
            ),
            call(4, "kawaro_image", json!({"op": "resize", "paths": [photo]})),
        ],
        &cfg,
    );
    for id in 1..=4 {
        let r = tool_result(&out, id);
        assert!(r["isError"].as_bool().unwrap_or(false), "call {id}: {r}");
        assert_eq!(error_code(r), "READ_ONLY", "call {id}: {r}");
        assert!(failure_text(r).contains("READ_ONLY"), "call {id}");
    }
    assert_eq!(
        entries(fx.dir.path()),
        ["doc.pdf", "photo.jpg", "second.jpg"],
        "a refused call writes nothing"
    );
}

#[test]
fn named_tools_map_to_their_action_ids() {
    let fx = fixture();
    let second = fx.dir.path().join("second.jpg");
    write_photo(&second);
    let cfg = config_for(fx.dir.path());
    let photo = fx.photo.to_str().unwrap();

    // kawaro_images_to_pdf maps to image.to-pdf and writes one combined PDF.
    let out = run(
        &[call(
            1,
            "kawaro_images_to_pdf",
            json!({"paths": [photo, second.to_str().unwrap()], "preview": false}),
        )],
        &cfg,
    );
    let r = tool_result(&out, 1);
    assert_eq!(r["isError"].as_bool(), Some(false), "{r}");
    assert_eq!(
        r["structuredContent"]["action"].as_str(),
        Some("image.to-pdf")
    );
    let pdf = PathBuf::from(output_path(r));
    assert_eq!(
        pdf.file_name().and_then(|name| name.to_str()),
        Some("photo.pdf")
    );
    assert!(pdf.is_file(), "the combined PDF is written: {pdf:?}");

    // kawaro_pdf op compress maps to pdf.compress.
    let out = run(
        &[call(
            1,
            "kawaro_pdf",
            json!({"op": "compress", "paths": [pdf.to_str().unwrap()], "preview": false}),
        )],
        &cfg,
    );
    let r = tool_result(&out, 1);
    assert_eq!(r["isError"].as_bool(), Some(false), "{r}");
    assert_eq!(
        r["structuredContent"]["action"].as_str(),
        Some("pdf.compress")
    );
    let compressed = PathBuf::from(output_path(r));
    assert!(
        compressed.is_file(),
        "the compressed PDF is written: {compressed:?}"
    );

    // kawaro_image op resize maps to image.resize-50.
    let out = run(
        &[call(
            1,
            "kawaro_image",
            json!({"op": "resize", "paths": [photo]}),
        )],
        &cfg,
    );
    let r = tool_result(&out, 1);
    assert_eq!(r["isError"].as_bool(), Some(false), "{r}");
    assert_eq!(
        r["structuredContent"]["action"].as_str(),
        Some("image.resize-50")
    );
    let resized = PathBuf::from(output_path(r));
    assert!(
        resized.is_file(),
        "the resized image is written: {resized:?}"
    );
}

#[test]
fn preview_flag_controls_the_image_block() {
    let fx = fixture();
    let cfg = config_for(fx.dir.path());
    let photo = fx.photo.to_str().unwrap();
    let out = run(
        &[
            call(
                1,
                "kawaro_convert",
                json!({"action": "image.to-webp", "paths": [photo], "preview": false}),
            ),
            call(
                2,
                "kawaro_convert",
                json!({"action": "image.to-webp", "paths": [photo], "preview": true}),
            ),
        ],
        &cfg,
    );
    let without = tool_result(&out, 1);
    assert_eq!(without["isError"].as_bool(), Some(false), "{without}");
    assert!(
        !without["content"]
            .as_array()
            .unwrap()
            .iter()
            .any(|block| block["type"] == "image"),
        "preview false sends no image block: {without}"
    );
    let with = tool_result(&out, 2);
    assert_eq!(with["isError"].as_bool(), Some(false), "{with}");
    let block = with["content"]
        .as_array()
        .unwrap()
        .iter()
        .find(|block| block["type"] == "image")
        .expect("preview true sends the image block");
    assert_eq!(block["mimeType"].as_str(), Some("image/png"));
    assert!(!block["data"].as_str().unwrap_or("").is_empty());
}

#[test]
fn options_on_an_action_without_options_are_bad_options() {
    let fx = fixture();
    let cfg = config_for(fx.dir.path());
    let out = run(
        &[call(
            1,
            "kawaro_convert",
            json!({
                "action": "image.rotate-90",
                "paths": [fx.photo.to_str().unwrap()],
                "options": {"x": 1},
            }),
        )],
        &cfg,
    );
    assert_eq!(error_code(tool_result(&out, 1)), "BAD_OPTIONS");
}

#[test]
fn a_corrupt_input_fails_with_the_failed_code() {
    let fx = fixture();
    let broken = fx.dir.path().join("broken.jpg");
    std::fs::write(&broken, b"this is not a jpeg").unwrap();
    let cfg = config_for(fx.dir.path());
    let out = run(
        &[call(
            1,
            "kawaro_convert",
            json!({
                "action": "image.to-webp",
                "paths": [broken.to_str().unwrap()],
            }),
        )],
        &cfg,
    );
    let r = tool_result(&out, 1);
    assert_eq!(r["isError"].as_bool(), Some(true), "{r}");
    assert!(
        failure_text(r).starts_with("ERROR [FAILED]"),
        "{}",
        failure_text(r)
    );
    assert_eq!(
        r["structuredContent"]["error"]["code"].as_str(),
        Some("FAILED")
    );
    assert!(
        r["structuredContent"]["outputs"]
            .as_array()
            .unwrap()
            .is_empty(),
        "{r}"
    );
    assert!(!fx.dir.path().join("broken.webp").exists());
}

#[test]
fn a_write_call_is_one_audit_line_naming_the_output() {
    let fx = fixture();
    let audit_path = fx.dir.path().join("audit").join("mcp-audit.jsonl");
    let mut cfg = config_for(fx.dir.path());
    cfg.audit = true;
    cfg.audit_path = Some(audit_path.clone());
    let out = run(
        &[call(
            1,
            "kawaro_convert",
            json!({
                "action": "image.to-webp",
                "paths": [fx.photo.to_str().unwrap()],
                "preview": false,
            }),
        )],
        &cfg,
    );
    let r = tool_result(&out, 1);
    assert_eq!(r["isError"].as_bool(), Some(false), "{r}");
    let created = output_path(r);
    assert!(Path::new(&created).is_file(), "{created}");

    let text = std::fs::read_to_string(&audit_path).unwrap();
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    assert_eq!(lines.len(), 1, "one line for one tools/call");
    let line: Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(line["tool"].as_str(), Some("kawaro_convert"));
    assert_eq!(line["ok"].as_bool(), Some(true));
    let named = line["outputs"].as_array().unwrap().iter().any(|entry| {
        entry.as_str() == Some(created.as_str())
            || entry.get("path").and_then(Value::as_str) == Some(created.as_str())
    });
    assert!(named, "the audit line names the output: {line}");
}

#[test]
fn tasks_resources_and_prompts_are_method_not_found() {
    let fx = fixture();
    let cfg = config_for(fx.dir.path());
    let out = run(
        &[
            json!({"jsonrpc": "2.0", "id": 1, "method": "tasks/get", "params": {"taskId": "t"}}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "resources/list"}),
            json!({"jsonrpc": "2.0", "id": 3, "method": "prompts/list"}),
        ],
        &cfg,
    );
    assert_eq!(out.len(), 3);
    for id in 1..=3 {
        let r = result_for(&out, id);
        assert_eq!(r["error"]["code"].as_i64(), Some(-32601), "{r}");
        assert_eq!(
            r["error"]["message"].as_str(),
            Some("method not found"),
            "{r}"
        );
    }
}

#[test]
fn an_unknown_tool_is_a_protocol_error_and_is_audited() {
    let fx = fixture();
    let audit_path = fx.dir.path().join("audit").join("mcp-audit.jsonl");
    let mut cfg = config_for(fx.dir.path());
    cfg.audit = true;
    cfg.audit_path = Some(audit_path.clone());
    let out = run(&[call(1, "kawaro_nope", json!({}))], &cfg);
    assert_eq!(out.len(), 1);
    let response = result_for(&out, 1);
    assert_eq!(
        response["error"]["code"].as_i64(),
        Some(-32602),
        "{response}"
    );

    let text = std::fs::read_to_string(&audit_path).unwrap();
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    assert_eq!(lines.len(), 1, "the unknown tool is audited once");
    let line: Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(line["tool"].as_str(), Some("kawaro_nope"));
    assert_eq!(line["ok"].as_bool(), Some(false));
}
