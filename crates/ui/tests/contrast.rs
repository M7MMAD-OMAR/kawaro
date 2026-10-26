//! S-062: the hive's high contrast look. Pure checks on `design/tokens.json`
//! (WCAG 2.1 contrast ratios, both schemes), on the generated `tokens.slint`
//! and on the .slint sources. The rendered pixels are checked by
//! tests/snapshots.sh through tests/high_contrast.py.

use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tokens() -> Value {
    let raw = std::fs::read_to_string(root().join("design/tokens.json")).expect("read tokens.json");
    serde_json::from_str(&raw).expect("parse tokens.json")
}

fn source(rel: &str) -> String {
    std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel))
        .unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

const SCHEMES: [&str; 2] = ["light", "dark"];
const NEW_COLOURS: [&str; 5] = [
    "hive-plate",
    "hive-edge",
    "hex-fill",
    "hex-fill-centre",
    "hex-ink",
];

fn colour(t: &Value, scheme: &str, key: &str) -> String {
    t["color"][scheme][key]
        .as_str()
        .unwrap_or_else(|| panic!("design/tokens.json: color.{scheme}.{key} is missing"))
        .to_string()
}

fn rgb(hex: &str) -> [u8; 3] {
    let h = hex.strip_prefix('#').unwrap_or(hex);
    assert!(
        h.len() == 6 && h.bytes().all(|b| b.is_ascii_hexdigit()),
        "{hex} is not #RRGGBB"
    );
    let v = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap();
    [v(0), v(2), v(4)]
}

/// WCAG 2.1 relative luminance.
fn luminance(hex: &str) -> f64 {
    let lin = |c: u8| {
        let c = f64::from(c) / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let [r, g, b] = rgb(hex);
    0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

fn ratio(a: &str, b: &str) -> f64 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

fn assert_ratio(t: &Value, scheme: &str, a: &str, b: &str, min: f64) {
    let (ca, cb) = (colour(t, scheme, a), colour(t, scheme, b));
    let r = ratio(&ca, &cb);
    assert!(
        r >= min,
        "{scheme}: {a} {ca} vs {b} {cb} is {r:.2}:1, want >= {min}:1"
    );
}

#[test]
fn ac1_new_colour_tokens_exist_in_both_schemes_as_rrggbb() {
    let t = tokens();
    for scheme in SCHEMES {
        for key in NEW_COLOURS {
            let v = colour(&t, scheme, key);
            rgb(&v);
        }
    }
}

#[test]
fn ac1_border_width_hex_is_2_and_font_size_cell_is_14() {
    let t = tokens();
    assert_eq!(
        t["border-width"]["hex"].as_u64(),
        Some(2),
        "border-width.hex"
    );
    assert_eq!(
        t["font"]["size"]["cell"].as_u64(),
        Some(14),
        "font.size.cell"
    );
    assert_eq!(t["font"]["weight"]["semibold"].as_u64(), Some(600));
}

#[test]
fn ac1_generated_tokens_slint_holds_the_same_hex_strings() {
    let t = tokens();
    let generated =
        std::fs::read_to_string(PathBuf::from(env!("OUT_DIR")).join("tokens.slint")).unwrap();
    let lower = generated.to_lowercase();
    for scheme in SCHEMES {
        for key in NEW_COLOURS {
            let v = colour(&t, scheme, key).to_lowercase();
            assert!(
                lower.contains(&v),
                "tokens.slint lacks {key} {v} of the {scheme} scheme"
            );
        }
    }
    for name in ["hive-plate", "hive-edge", "hex-fill-centre", "hex-ink"] {
        assert!(
            generated.contains(&format!("property <color> {name}:")),
            "tokens.slint lacks the property {name}"
        );
    }
    assert!(
        generated.contains("border-hex: 2px"),
        "tokens.slint lacks border-hex: 2px"
    );
    assert!(
        generated.contains("font-cell: 14px"),
        "tokens.slint lacks font-cell: 14px"
    );
}

#[test]
fn ac2_cell_ink_reads_on_both_fills_at_7_to_1() {
    let t = tokens();
    for scheme in SCHEMES {
        assert_ratio(&t, scheme, "hex-ink", "hex-fill", 7.0);
        assert_ratio(&t, scheme, "hex-ink", "hex-fill-centre", 7.0);
    }
}

#[test]
fn ac3_cell_edge_stands_out_from_plate_and_fill_at_3_to_1() {
    let t = tokens();
    for scheme in SCHEMES {
        assert_ratio(&t, scheme, "hive-edge", "hive-plate", 3.0);
        assert_ratio(&t, scheme, "hive-edge", "hex-fill", 3.0);
    }
}

#[test]
fn ac4_plate_rim_stands_out_from_a_plain_backdrop_at_3_to_1() {
    let t = tokens();
    let dark = colour(&t, "dark", "hive-edge");
    let light = colour(&t, "light", "hive-edge");
    let (rd, rl) = (ratio(&dark, "#000000"), ratio(&light, "#FFFFFF"));
    assert!(rd >= 3.0, "dark hive-edge {dark} vs #000000 is {rd:.2}:1");
    assert!(rl >= 3.0, "light hive-edge {light} vs #FFFFFF is {rl:.2}:1");
}

#[test]
fn ac5_highlight_ink_reads_at_7_to_1_and_its_edge_at_3_to_1() {
    let t = tokens();
    for scheme in SCHEMES {
        assert_ratio(&t, scheme, "accent-ink", "accent", 7.0);
        // The highlighted cell keeps the hive-edge stroke.
        assert_ratio(&t, scheme, "hive-edge", "hive-plate", 3.0);
    }
}

// S-062 ac6 (the hive has an opaque plate) is replaced by S-074 AC1, tested in
// crates/ui/src/s074_tests.rs: the plate is gone and the window is transparent.

#[test]
fn ac7_hex_cell_uses_only_the_new_tokens() {
    let cell = source("components/hex-cell.slint");
    for bad in ["surface-", "Tokens.border;"] {
        assert_eq!(
            cell.matches(bad).count(),
            0,
            "hex-cell.slint still uses `{bad}`"
        );
    }
    for good in [
        "Tokens.hex-fill",
        "Tokens.hex-fill-centre",
        "Tokens.hex-ink",
        "Tokens.hive-edge",
        "Tokens.border-hex",
        "Tokens.accent-ink",
    ] {
        assert!(cell.contains(good), "hex-cell.slint does not use {good}");
    }
}

#[test]
fn ac8_design_lint_is_clean_with_the_new_look_in_place() {
    // Lint alone passes today; tie it to the new look so it can only pass
    // once the plate and the tokens exist.
    assert!(
        source("ui/hive.slint").contains("Tokens.hive-plate"),
        "the lint result is only meaningful with the plate in hive.slint"
    );
    let out = Command::new("bash")
        .arg(root().join("scripts/design-lint.sh"))
        .current_dir(root())
        .output()
        .expect("run design-lint.sh");
    assert!(
        out.status.success(),
        "design-lint.sh: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn ac8_no_long_dashes_in_the_files_this_story_touches() {
    let t = std::fs::read_to_string(root().join("design/tokens.json")).unwrap();
    let files = [
        t,
        source("ui/hive.slint"),
        source("components/hex-cell.slint"),
    ];
    for f in files {
        assert!(!f.contains('\u{2013}') && !f.contains('\u{2014}'));
    }
    assert!(
        source("ui/hive.slint").contains("Tokens.hive-plate"),
        "the new look is not in place yet"
    );
}
