//! S-017a (AC1 to AC3): glass tokens, motion tokens, generator output,
//! reduced motion, contrast over black and white, design lint and the
//! design-system rows. Written from the acceptance criteria of
//! backlog/S-017-glass-hive-motion.md only, before implementation.
//!
//! Contract these tests pin for the implementer (also in the story review log):
//!
//! - `design/tokens.json` `color.{light,dark}` gains `glass-2` and `glass-3`
//!   with the AC1 hex strings; `motion` gains `ring-delay-ms` 40,
//!   `highlight-move` {`duration-ms` 140, `easing` `cubic-bezier(0.2, 0, 0, 1)`},
//!   `press` {`duration-ms` 70, `scale-to` 0.94}, `hive-close` {`duration-ms`
//!   90, `scale-to` 0.96, `opacity-to` 0, `easing` `ease-in`}.
//! - The `site` block gains `color.glass-2` and `color.glass-3` (the light
//!   values; the site is light only) and `motion` `duration-ring-delay` 40,
//!   `duration-highlight-move` 140, `duration-press` 70,
//!   `duration-hive-close` 90, so `scripts/tokens-to-css.py` writes
//!   `--color-glass-2`, `--color-glass-3` and the four `--duration-*`, and
//!   zeroes the durations under `prefers-reduced-motion: reduce`.
//! - The generated `Tokens` global gains `glass-2` and `glass-3` colours,
//!   durations `ring-delay`, `highlight-move`, `press`, `hive-close`, each
//!   `reduced-motion ? 0ms : Nms`, easings `hive-open-easing`,
//!   `highlight-move-easing`, `hive-close-easing`, and floats
//!   `press-scale-to` 0.94, `hive-close-scale-to` 0.96,
//!   `hive-close-opacity-to` 0.
//! - `scripts/design-lint.sh`, run from a directory, fails on an 8-digit hex
//!   colour in that directory's `design/tokens.json` outside `glass-*` keys.
//! - `docs/design-system.md` carries table rows naming `glass-2`, `glass-3`,
//!   `ring-delay-ms`, `highlight-move`, `press` and `hive-close`.

use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn crate_file(rel: &str) -> String {
    std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel))
        .unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

fn tokens() -> Value {
    let raw = std::fs::read_to_string(root().join("design/tokens.json")).expect("read tokens.json");
    serde_json::from_str(&raw).expect("parse tokens.json")
}

fn colour(t: &Value, scheme: &str, key: &str) -> String {
    // Check if it's a glass color first
    if key.starts_with("glass-") {
        t["glass"][key][scheme]
            .as_str()
            .unwrap_or_else(|| panic!("design/tokens.json: glass.{key}.{scheme} is missing"))
            .to_string()
    } else {
        t["color"][scheme][key]
            .as_str()
            .unwrap_or_else(|| panic!("design/tokens.json: color.{scheme}.{key} is missing"))
            .to_string()
    }
}

/// Drop every whitespace so generated formatting cannot break a match.
fn tight(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

fn generated_tokens() -> String {
    std::fs::read_to_string(PathBuf::from(env!("OUT_DIR")).join("tokens.slint"))
        .expect("read the generated tokens.slint")
}

fn generated_line<'a>(generated: &'a str, needle: &str) -> &'a str {
    generated
        .lines()
        .find(|l| l.contains(needle))
        .unwrap_or_else(|| panic!("tokens.slint has no line with `{needle}`"))
}

fn out_text(o: &std::process::Output) -> String {
    format!(
        "exit: {:?}\nstdout:\n{}\nstderr:\n{}",
        o.status.code(),
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

/// The four new durations, as (token name, milliseconds).
const DURATIONS: [(&str, u64); 4] = [
    ("ring-delay", 40),
    ("highlight-move", 140),
    ("press", 70),
    ("hive-close", 90),
];

#[derive(Clone)]
struct Rgba {
    r: f64,
    g: f64,
    b: f64,
    a: f64,
}

fn rgba(hex: &str) -> Rgba {
    let h = hex.strip_prefix('#').unwrap_or(hex);
    assert!(
        h.len() == 6 || h.len() == 8,
        "{hex} is not #RRGGBB or #RRGGBBAA"
    );
    assert!(h.bytes().all(|b| b.is_ascii_hexdigit()), "{hex} is not hex");
    let v = |i: usize| f64::from(u8::from_str_radix(&h[i..i + 2], 16).unwrap());
    Rgba {
        r: v(0),
        g: v(2),
        b: v(4),
        a: if h.len() == 8 { v(6) / 255.0 } else { 1.0 },
    }
}

/// WCAG 2.1 relative luminance.
fn luminance(c: [f64; 3]) -> f64 {
    let lin = |v: f64| {
        let c = v / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(c[0]) + 0.7152 * lin(c[1]) + 0.0722 * lin(c[2])
}

fn ratio(a: [f64; 3], b: [f64; 3]) -> f64 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// Source-over composite of a straight-alpha colour on an opaque backdrop.
fn composite(fg: Rgba, bg: [f64; 3]) -> [f64; 3] {
    [
        fg.r * fg.a + bg[0] * (1.0 - fg.a),
        fg.g * fg.a + bg[1] * (1.0 - fg.a),
        fg.b * fg.a + bg[2] * (1.0 - fg.a),
    ]
}

// ---------------------------------------------------------------- AC1 tokens

#[test]
fn ac1_tokens_json_has_the_glass_colours_of_both_schemes() {
    let t = tokens();
    let expect = [
        ("light", "glass-2", "#E9E1D0B8"),
        ("light", "glass-3", "#DFD3BBD1"),
        ("dark", "glass-2", "#27231CB8"),
        ("dark", "glass-3", "#322C23D1"),
    ];
    for (scheme, key, want) in expect {
        let got = colour(&t, scheme, key);
        assert_eq!(got, want, "color.{scheme}.{key}");
    }
}

#[test]
fn ac1_tokens_json_has_the_motion_tokens_of_decision_5() {
    let t = tokens();
    assert_eq!(
        t["motion"]["ring-delay-ms"].as_f64(),
        Some(40.0),
        "motion.ring-delay-ms"
    );
    let mv = &t["motion"]["highlight-move"];
    assert_eq!(
        mv["duration-ms"].as_f64(),
        Some(140.0),
        "motion.highlight-move.duration-ms"
    );
    assert_eq!(
        mv["easing"].as_str(),
        Some("cubic-bezier(0.2, 0, 0, 1)"),
        "motion.highlight-move.easing"
    );
    let press = &t["motion"]["press"];
    assert_eq!(
        press["duration-ms"].as_f64(),
        Some(70.0),
        "motion.press.duration-ms"
    );
    assert_eq!(
        press["scale-to"].as_f64(),
        Some(0.94),
        "motion.press.scale-to"
    );
    let close = &t["motion"]["hive-close"];
    assert_eq!(
        close["duration-ms"].as_f64(),
        Some(90.0),
        "motion.hive-close.duration-ms"
    );
    assert_eq!(
        close["scale-to"].as_f64(),
        Some(0.96),
        "motion.hive-close.scale-to"
    );
    assert_eq!(
        close["opacity-to"].as_f64(),
        Some(0.0),
        "motion.hive-close.opacity-to"
    );
    assert_eq!(
        close["easing"].as_str(),
        Some("ease-in"),
        "motion.hive-close.easing"
    );
}

#[test]
fn ac1_generated_tokens_carry_glass_and_motion() {
    let t = tokens();
    let generated = generated_tokens();
    let low = generated.to_lowercase();
    for key in ["glass-2", "glass-3"] {
        assert!(
            generated.contains(&format!("property <color> {key}:")),
            "tokens.slint lacks the property {key}"
        );
        for scheme in ["light", "dark"] {
            let v = colour(&t, scheme, key).to_lowercase();
            assert!(
                low.contains(&v),
                "tokens.slint lacks {key} {v} of the {scheme} scheme"
            );
        }
    }
    for (name, ms) in DURATIONS {
        assert!(
            generated.contains(&format!("property <duration> {name}:")),
            "tokens.slint lacks the duration {name} ({ms} ms)"
        );
    }
    for easing in [
        "hive-open-easing",
        "highlight-move-easing",
        "hive-close-easing",
    ] {
        assert!(
            generated.contains(easing),
            "tokens.slint lacks the easing property {easing}"
        );
    }
    let flat = tight(&generated);
    assert!(
        flat.contains("cubic-bezier(0.2,0,0,1)"),
        "tokens.slint lacks the highlight-move cubic-bezier(0.2, 0, 0, 1)"
    );
    for (prop, value) in [
        ("press-scale-to", "0.94"),
        ("hive-close-scale-to", "0.96"),
        ("hive-close-opacity-to", "0"),
    ] {
        let line = generated_line(&generated, prop);
        assert!(
            tight(line).contains(value),
            "tokens.slint line for {prop} does not carry {value}: {line}"
        );
    }
}

#[test]
fn ac1_new_durations_are_zero_under_reduced_motion_in_tokens() {
    let generated = generated_tokens();
    for (name, ms) in DURATIONS {
        let line = generated_line(&generated, &format!("property <duration> {name}:"));
        let flat = tight(line);
        assert!(
            line.contains("reduced-motion") && flat.contains(&format!("0ms:{ms}ms")),
            "the {name} duration is not `reduced-motion ? 0ms : {ms}ms`: {line}"
        );
    }
}

#[test]
fn ac1_site_tokens_css_carries_glass_and_the_new_durations() {
    let t = tokens();
    let css = tight(
        &std::fs::read_to_string(root().join("site/tokens.css")).expect("read site/tokens.css"),
    );
    assert!(
        css.contains("--color-glass-2:#E9E1D0B8;"),
        "site/tokens.css lacks --color-glass-2"
    );
    assert!(
        css.contains("--color-glass-3:#DFD3BBD1;"),
        "site/tokens.css lacks --color-glass-3"
    );
    for (name, ms) in DURATIONS {
        assert!(
            css.contains(&format!("--duration-{name}:{ms}ms;")),
            "site/tokens.css lacks --duration-{name}: {ms}ms"
        );
    }
    // The generated file must track the site block of tokens.json.
    assert_eq!(
        t["site"]["color"]["glass-2"].as_str(),
        Some("#E9E1D0B8"),
        "site.color.glass-2"
    );
    assert_eq!(
        t["site"]["color"]["glass-3"].as_str(),
        Some("#DFD3BBD1"),
        "site.color.glass-3"
    );
    for (name, ms) in DURATIONS {
        assert_eq!(
            t["site"]["motion"][format!("duration-{name}")].as_f64(),
            Some(ms as f64),
            "site.motion.duration-{name}"
        );
    }
}

#[test]
fn ac1_new_durations_are_zero_under_reduced_motion_in_site_tokens_css() {
    let css =
        std::fs::read_to_string(root().join("site/tokens.css")).expect("read site/tokens.css");
    let block = css
        .split("@media (prefers-reduced-motion: reduce)")
        .nth(1)
        .expect("site/tokens.css lacks the prefers-reduced-motion block");
    let block = tight(block);
    for (name, ms) in DURATIONS {
        assert!(
            block.contains(&format!("--duration-{name}:0ms;")),
            "site/tokens.css does not zero --duration-{name} ({ms} ms) under reduced motion"
        );
    }
}

// ------------------------------------------------- AC1 build.rs and hive.slint

#[cfg(unix)]
fn build_script() -> PathBuf {
    let out = PathBuf::from(env!("OUT_DIR"));
    // The compiled binary is in target/debug/build/kawaro-ui-<hash2>/build-script-build
    // OUT_DIR is target/debug/build/kawaro-ui-<hash>/out, so we need to search the build directory
    let build_dir = out
        .parent()
        .expect("OUT_DIR has a parent")
        .parent()
        .expect("OUT_DIR parent has a parent");

    // Find the most recently modified build-script-build in any kawaro-ui-* directory
    let mut candidates = Vec::new();
    for entry in std::fs::read_dir(build_dir).expect("read build directory") {
        let entry = entry.expect("build directory entry");
        let path = entry.path();
        if path.is_dir()
            && path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("kawaro-ui-")
        {
            let exe = path.join("build-script-build");
            if exe.exists() {
                let modified = exe.metadata().unwrap().modified().unwrap();
                candidates.push((modified, exe));
            }
        }
    }

    candidates.sort_by_key(|(modified, _)| *modified);
    candidates
        .last()
        .map(|(_, exe)| exe.clone())
        .expect("no compiled build script found in build directory")
}

#[cfg(unix)]
fn set_motion(t: &mut Value, key: &str, fields: &[(&str, Value)]) {
    let motion = t["motion"]
        .as_object_mut()
        .expect("tokens.json motion object");
    let entry = motion
        .entry(key.to_string())
        .or_insert_with(|| serde_json::json!({}));
    let obj = entry.as_object_mut().expect("motion entry object");
    for (k, v) in fields {
        obj.insert((*k).to_string(), v.clone());
    }
}

/// The real tokens.json with every AC1 motion value merged in, so the
/// fixture stays valid before and after the implementation adds them.
#[cfg(unix)]
fn ac_tokens() -> Value {
    let mut t = tokens();
    t["motion"]
        .as_object_mut()
        .unwrap()
        .insert("ring-delay-ms".to_string(), serde_json::json!(40));
    set_motion(
        &mut t,
        "highlight-move",
        &[
            ("duration-ms", serde_json::json!(140)),
            ("easing", serde_json::json!("cubic-bezier(0.2, 0, 0, 1)")),
        ],
    );
    set_motion(
        &mut t,
        "press",
        &[
            ("duration-ms", serde_json::json!(70)),
            ("scale-to", serde_json::json!(0.94)),
        ],
    );
    set_motion(
        &mut t,
        "hive-close",
        &[
            ("duration-ms", serde_json::json!(90)),
            ("scale-to", serde_json::json!(0.96)),
            ("opacity-to", serde_json::json!(0)),
            ("easing", serde_json::json!("ease-in")),
        ],
    );
    t
}

/// Run the real build script on a temp manifest tree whose only difference
/// from the crate is `design/tokens.json`. The material, ui, components,
/// fonts and icons directories are symlinked from the crate.
#[cfg(unix)]
fn fixture_run(name: &str, t: &Value) -> std::process::Output {
    let tmp = std::env::temp_dir().join(format!("kawaro-s017a-build-{}", name));
    let _ = std::fs::remove_dir_all(&tmp);
    let manifest = tmp.join("kawaro/crates/ui");
    std::fs::create_dir_all(&manifest).unwrap();
    std::fs::create_dir_all(tmp.join("kawaro/design")).unwrap();
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for dir in ["material", "ui", "components", "fonts", "icons"] {
        std::os::unix::fs::symlink(src.join(dir), manifest.join(dir)).unwrap();
    }
    std::fs::write(
        tmp.join("kawaro/design/tokens.json"),
        serde_json::to_string_pretty(t).unwrap(),
    )
    .unwrap();
    let out = tmp.join("out");
    std::fs::create_dir_all(&out).unwrap();
    Command::new(build_script())
        .env("CARGO_MANIFEST_DIR", &manifest)
        .env("OUT_DIR", &out)
        .env("DEBUG", "true")
        .env("OPT_LEVEL", "0")
        .env("PROFILE", "debug")
        .env("NUM_JOBS", "2")
        .output()
        .expect("run the kawaro-ui build script")
}

#[cfg(unix)]
#[test]
fn ac1_build_rejects_an_unknown_easing_name() {
    let control = fixture_run("easing-name-control", &ac_tokens());
    assert!(
        control.status.success(),
        "the control build with the AC1 values must succeed:\n{}",
        out_text(&control)
    );
    let mut bad = ac_tokens();
    bad["motion"]["hive-close"]
        .as_object_mut()
        .unwrap()
        .insert("easing".to_string(), serde_json::json!("ease-wiggle"));
    let r = fixture_run("easing-name-bad", &bad);
    assert!(
        !r.status.success(),
        "build.rs must fail on the unknown easing name `ease-wiggle`:\n{}",
        out_text(&r)
    );
}

#[cfg(unix)]
#[test]
fn ac1_build_rejects_a_bad_bezier_shape() {
    let control = fixture_run("bezier-control", &ac_tokens());
    assert!(
        control.status.success(),
        "the control build with the AC1 values must succeed:\n{}",
        out_text(&control)
    );
    for (i, bad_bezier) in ["cubic-bezier(0.2, 0, 0)", "cubic-bezier(0.2, 0, 0, 1, 2)"]
        .iter()
        .enumerate()
    {
        let mut bad = ac_tokens();
        bad["motion"]["highlight-move"]
            .as_object_mut()
            .unwrap()
            .insert("easing".to_string(), serde_json::json!(bad_bezier));
        let r = fixture_run(&format!("bezier-bad-{i}"), &bad);
        assert!(
            !r.status.success(),
            "build.rs must fail on the bezier shape `{bad_bezier}`:\n{}",
            out_text(&r)
        );
    }
}

#[test]
fn ac1_hive_slint_contains_no_literal_ease_out() {
    let hive = crate_file("ui/hive.slint");
    assert!(
        !hive.contains("ease-out"),
        "ui/hive.slint still writes ease-out literally; take the easing from the Tokens global"
    );
}

// ------------------------------------------------------------------- AC2

#[test]
fn ac2_text_primary_on_glass_is_at_least_45_to_1_in_all_8_cases() {
    let t = tokens();
    let mut worst = f64::MAX;
    for scheme in ["light", "dark"] {
        let text = rgba(&colour(&t, scheme, "text-primary"));
        for key in ["glass-2", "glass-3"] {
            let glass = rgba(&colour(&t, scheme, key));
            let text_rgba = Rgba {
                r: text.r,
                g: text.g,
                b: text.b,
                a: 1.0,
            };
            let glass_rgba = Rgba {
                r: glass.r,
                g: glass.g,
                b: glass.b,
                a: glass.a,
            };

            for (bg_name, bg) in [
                ("pure black", [0.0, 0.0, 0.0]),
                ("pure white", [255.0, 255.0, 255.0]),
            ] {
                let surface = composite(glass_rgba.clone(), bg);
                let r = ratio([text_rgba.r, text_rgba.g, text_rgba.b], surface);
                println!("ac2 {scheme} text-primary on {key} over {bg_name}: {r:.2}:1");
                assert!(
                    r >= 4.5,
                    "{scheme}: text-primary on {key} composited over {bg_name} is {r:.2}:1, want >= 4.5:1"
                );
                worst = worst.min(r);
            }
        }
    }
    println!("ac2 worst of the 8 cases: {worst:.2}:1");
}

// ------------------------------------------------------------------- AC3

#[test]
fn ac3_design_lint_is_clean_on_the_repo() {
    let o = Command::new("bash")
        .arg(root().join("scripts/design-lint.sh"))
        .current_dir(root())
        .output()
        .expect("run design-lint.sh");
    assert!(
        o.status.success(),
        "design-lint.sh must stay clean:\n{}",
        out_text(&o)
    );
    assert!(
        String::from_utf8_lossy(&o.stdout).contains("clean"),
        "design-lint.sh did not report clean:\n{}",
        out_text(&o)
    );
}

/// Run design-lint.sh from a temp tree whose design/tokens.json is the given
/// fixture, exactly as the script reads $SITE_BASE/design/tokens.json.
fn lint_on_fixture(fixture: &str) -> std::process::Output {
    let tmp = std::env::temp_dir().join(format!(
        "kawaro-s017a-lint-{}",
        fixture.split('.').next().unwrap()
    ));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join("design")).unwrap();
    std::fs::create_dir_all(tmp.join("crates")).unwrap();
    std::fs::create_dir_all(tmp.join("site")).unwrap();
    std::fs::copy(
        root().join("tests/fixtures/design-lint").join(fixture),
        tmp.join("design/tokens.json"),
    )
    .unwrap();
    Command::new("bash")
        .arg(root().join("scripts/design-lint.sh"))
        .current_dir(&tmp)
        .output()
        .expect("run design-lint.sh")
}

#[test]
fn ac3_design_lint_fails_on_8_digit_hex_outside_glass_keys() {
    let bad = lint_on_fixture("tokens-bad-hex.json");
    assert!(
        !bad.status.success(),
        "design-lint.sh must fail on an 8-digit hex colour outside glass-* keys:\n{}",
        out_text(&bad)
    );
    let ok = lint_on_fixture("tokens-glass-ok.json");
    assert!(
        ok.status.success(),
        "design-lint.sh must stay clean when every 8-digit hex colour sits on a glass-* key:\n{}",
        out_text(&ok)
    );
}

#[test]
fn ac3_design_system_docs_have_rows_for_glass_and_the_new_motion_tokens() {
    let doc = std::fs::read_to_string(root().join("docs/design-system.md"))
        .expect("read docs/design-system.md");
    let rows: Vec<&str> = doc
        .lines()
        .filter(|l| l.trim_start().starts_with('|'))
        .collect();
    for name in [
        "glass-2",
        "glass-3",
        "ring-delay-ms",
        "highlight-move",
        "press",
        "hive-close",
    ] {
        let tick = format!("`{name}`");
        assert!(
            rows.iter().any(|r| r.contains(&tick)),
            "docs/design-system.md has no table row with {tick}"
        );
    }
}
