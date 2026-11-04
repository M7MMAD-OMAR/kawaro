//! S-130 AC2 and AC4 snapshot path: the centred card with bounded options,
//! the token-only styling, and the --choose-language snapshot harness.
//! Static checks on the files the story names; the rendered comparison runs
//! in the private headless KWin, not here.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    let path = root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} must exist: {e}", path.display()))
}

fn tokens() -> serde_json::Value {
    serde_json::from_str(&read("design/tokens.json")).expect("tokens.json is JSON")
}

fn language_block() -> String {
    let welcome = read("crates/ui/ui/welcome.slint");
    let start = welcome
        .find("if root.choose-language")
        .expect("welcome.slint has the language screen block");
    welcome[start..].to_string()
}

#[test]
fn ac2_the_language_geometry_tokens_exist_with_the_story_values() {
    let layout = &tokens()["layout"];
    assert_eq!(
        layout["language-option-height"].as_f64(),
        Some(56.0),
        "layout.language-option-height"
    );
    assert_eq!(
        layout["language-option-width"].as_f64(),
        Some(320.0),
        "layout.language-option-width"
    );
    assert_eq!(
        layout["language-card-width"].as_f64(),
        Some(520.0),
        "layout.language-card-width"
    );
    let min_width = layout["window-min-width"]
        .as_f64()
        .expect("window-min-width");
    assert_eq!(min_width, 560.0, "the minimum window width of the story");
    let space_6 = tokens()["space"]["space-6"]
        .as_f64()
        .expect("space.space-6");
    assert!(
        520.0 + 2.0 * space_6 <= 560.0,
        "the 520 card fits the 560 window with space-6 padding"
    );
}

#[test]
fn ac2_the_card_is_centred_bounded_and_token_only() {
    let block = language_block();
    for needle in [
        "Tokens.layout.language-card-width",
        "Tokens.layout.language-option-height",
        "Tokens.layout.language-option-width",
        "Tokens.space-6",
        "Tokens.radius-lg",
        "Tokens.border",
        "alignment",
    ] {
        assert!(block.contains(needle), "the language block uses {needle}");
    }
    assert!(
        block.contains("Tokens.surface-0") || block.contains("Tokens.surface-1"),
        "the card uses surface-0 or surface-1"
    );
    assert!(
        !block.contains("height: parent.height"),
        "no option may take the remaining height (the stretched pill bug)"
    );
    assert!(
        block.contains("height: Tokens.layout.language-option-height"),
        "every option has an explicit token height"
    );
    assert!(
        block.contains("max-width: Tokens.layout.language-option-width")
            || block.contains("width: Tokens.layout.language-option-width"),
        "every option has a bounded width from the tokens"
    );
    for forbidden in ["box-shadow", "gradient", "border-left", "border-right"] {
        assert!(
            !block.contains(forbidden),
            "the language block must not use {forbidden}"
        );
    }
}

#[test]
fn ac3_the_language_block_takes_its_strings_from_rust_and_mirrors_by_hand() {
    let block = language_block();
    for needle in [
        "text.language-title",
        "text.language-subtitle",
        "text.language-failed",
        "text.language-english",
        "text.language-english-accessible",
        "text.language-arabic",
        "text.language-arabic-accessible",
    ] {
        assert!(
            block.contains(needle),
            "the language block reads {needle} from the Rust strings"
        );
    }
    for hardcoded in [
        "Choose your language",
        "اختر لغتك",
        "Could not save",
        "تعذر الحفظ",
    ] {
        assert!(
            !block.contains(hardcoded),
            "the language block must not hardcode: {hardcoded}"
        );
    }
    assert!(
        block.contains("root.rtl"),
        "the language block mirrors the card and the option order for Arabic"
    );
    assert!(
        block.contains("text-primary"),
        "the focus outline uses the text-primary token"
    );
    assert!(block.contains("2px"), "the focus outline is 2 px");
}

#[test]
fn ac3_the_keyboard_and_initial_focus_are_wired() {
    let block = language_block();
    for key in ["Key.Tab", "Key.Return", "Key.Space", "Key.Escape"] {
        assert!(block.contains(key), "the language block handles {key}");
    }
    assert!(
        block.contains("focus") || block.contains("has-focus"),
        "the initial focus is on the first option"
    );
}

#[test]
fn ac4_the_snapshot_harness_has_the_choose_language_path() {
    let example = read("crates/ui/examples/welcome_snapshot.rs");
    assert!(
        example.contains("--choose-language"),
        "welcome_snapshot.rs must accept --choose-language"
    );
    let script = read("crates/ui/tests/welcome_snapshots.sh");
    for reference in [
        "welcome-choose-en-dark.png",
        "welcome-choose-en-light.png",
        "welcome-choose-ar-dark.png",
        "welcome-choose-ar-light.png",
    ] {
        assert!(
            script.contains(reference),
            "welcome_snapshots.sh must render and compare {reference}"
        );
    }
    assert!(
        script.contains("--choose-language"),
        "the script passes --choose-language to the example"
    );
    assert!(
        script.to_lowercase().contains("candidate"),
        "without --update a missing reference keeps a candidate folder for the Lead"
    );
}
