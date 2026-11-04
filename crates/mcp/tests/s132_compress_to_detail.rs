//! S-132 AC3: the MCP result carries the structured size detail.
//! Contract pinned by this test (tester): each structuredContent.outputs entry
//! gains width, height, source_width, source_height and downscaled, and the
//! result text names the final dimensions and bytes.

use kawaro_mcp::{Config, serve};
use serde_json::{Value, json};
use std::path::Path;

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

fn call_compress(photo: &Path) -> Value {
    let dir = photo.parent().unwrap();
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "kawaro_convert",
            "arguments": {
                "action": "image.compress-to",
                "paths": [photo.to_str().unwrap()],
                "options": { "target_bytes": 50000 },
                "preview": false,
            },
        },
    });
    let mut input = request.to_string();
    input.push('\n');
    let mut output = Vec::new();
    serve(std::io::Cursor::new(input), &mut output, &config_for(dir)).expect("serve");
    let text = String::from_utf8(output).unwrap();
    let line: Value = serde_json::from_str(text.lines().next().expect("one response")).unwrap();
    line["result"].clone()
}

#[test]
fn ac3_compress_to_outputs_carry_dimensions_and_source_size() {
    let dir = tempfile::tempdir().unwrap();
    let photo = dir.path().join("photo.jpg");
    let image = image::RgbImage::from_fn(2400, 1600, |x, y| {
        image::Rgb([
            ((x * 37 + y * 17) % 256) as u8,
            ((x * 53 + y * 29) % 256) as u8,
            ((x * 11 + y * 47) % 256) as u8,
        ])
    });
    image.save(&photo).unwrap();

    let result = call_compress(&photo);
    assert_eq!(result["isError"].as_bool(), Some(false), "{result}");
    let structured = &result["structuredContent"];
    let output = &structured["outputs"][0];
    let width = output["width"].as_u64().unwrap_or(0) as u32;
    let height = output["height"].as_u64().unwrap_or(0) as u32;
    let bytes = output["bytes"].as_u64().unwrap_or(u64::MAX);
    assert!(
        width > 0 && height > 0,
        "final dimensions are present: {output}"
    );
    assert!(
        width < 2400 && height < 1600,
        "the fallback downscaled: {output}"
    );
    assert_eq!(output["source_width"].as_u64(), Some(2400));
    assert_eq!(output["source_height"].as_u64(), Some(1600));
    assert_eq!(output["downscaled"].as_bool(), Some(true));
    assert!(bytes <= 50_000, "the target is met: {bytes}");

    let text = result["content"][0]["text"].as_str().unwrap_or("");
    assert!(
        text.contains(&width.to_string()),
        "the width is in the text: {text}"
    );
    assert!(
        text.contains(&height.to_string()),
        "the height is in the text: {text}"
    );
    assert!(
        text.contains(&bytes.to_string()),
        "the bytes are in the text: {text}"
    );
}
