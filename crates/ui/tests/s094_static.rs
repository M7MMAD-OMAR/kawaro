//! S-094 (backlog/S-094-settings-app-shell.md): the rows of the test case
//! table that read files only (AC8 shared shell, AC9 image-fit, AC10 tokens
//! and design rules). They compile on their own, so they fail by assertion
//! before the shell exists. The behaviour tests are in `s094_shell.rs`.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    let p = root().join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} must exist: {e}", p.display()))
}

#[test]
fn ac8_settings_and_welcome_share_one_shell_component() {
    let shell = read("crates/ui/ui/shell.slint");
    assert!(
        shell.contains("NavigationRail"),
        "the vendored rail is used"
    );
    for f in ["crates/ui/ui/settings.slint", "crates/ui/ui/welcome.slint"] {
        let src = read(f);
        assert!(
            src.lines()
                .any(|l| l.trim_start().starts_with("import") && l.contains("shell.slint")),
            "{f} must import shell.slint"
        );
    }
}

#[test]
fn ac9_the_film_is_image_fit_contain() {
    let src = read("crates/ui/ui/welcome.slint") + &read("crates/ui/ui/shell.slint");
    assert!(src.contains("image-fit: contain"), "image-fit: contain");
}

// ------------------------------------------------------------ AC10

#[test]
fn ac10_layout_tokens_exist_with_the_stated_values() {
    let tokens: serde_json::Value = serde_json::from_str(&read("design/tokens.json")).unwrap();
    let layout = tokens
        .get("layout")
        .expect("tokens.json has a layout group");
    for (k, v) in [
        ("window-width", 880),
        ("window-height", 640),
        ("window-min-width", 560),
        ("window-min-height", 560),
        ("header-height", 64),
        ("header-logo", 40),
        ("rail-width", 96),
        ("content-max-width", 640),
        ("row-breakpoint", 520),
        ("label-column", 220),
        ("film-max-width", 560),
    ] {
        assert_eq!(
            layout.get(k).and_then(|x| x.as_i64()),
            Some(v),
            "layout.{k} must be {v}"
        );
    }
}

#[test]
fn ac10_the_shell_source_follows_the_design_rules() {
    let src = read("crates/ui/ui/shell.slint");
    for bad in [
        "drop-shadow",
        "box-shadow",
        "linear-gradient",
        "radial-gradient",
    ] {
        assert!(!src.contains(bad), "shell.slint uses {bad}");
    }
    assert!(!src.contains('\u{2014}') && !src.contains('\u{2013}'));
    assert!(!src.contains("border-left") && !src.contains("border-right"));
    assert!(
        !src.lines()
            .any(|l| l.contains("#") && !l.trim_start().starts_with("//")),
        "no hand written colours"
    );
    assert!(
        src.contains("kawaro-cells") || read("crates/ui/ui/welcome.slint").contains("kawaro-cells")
    );
}
