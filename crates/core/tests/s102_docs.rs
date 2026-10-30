//! S-102 (backlog/S-102-alt-doctor-and-docs.md), AC3: `docs/drop-shortcut.md`
//! and `docs/drop-shortcut.ar.md` carry the written evaluation of opening the
//! hive from a bare Alt tap.
//!
//! Contract (tester): the English file has a heading containing
//! `Alt tap` and, in the text after it, the three options by name
//! (`clipboard`, `kawaro-open-selection`, `AT-SPI`), a sentence that a
//! Wayland client cannot read a file manager selection directly, a line with
//! `Measured` and a heading or bold label `Recommendation`. The Arabic copy
//! has the same Latin names, a heading containing `Alt` and `النقرة`, and the
//! label `التوصية` and `قيس`. Both keep the language switch on line 1 and
//! contain no long dash.

use std::path::PathBuf;

fn read(rel: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} must exist: {e}", p.display()))
}

fn after_alt_tap(text: &str, heading_word: &str) -> String {
    let at = text
        .lines()
        .position(|l| l.trim_start().starts_with('#') && l.contains(heading_word))
        .unwrap_or_else(|| panic!("no heading with `{heading_word}`"));
    text.lines().skip(at).collect::<Vec<_>>().join("\n")
}

#[test]
fn ac3_english_evaluation_names_the_options_the_limit_and_one_recommendation() {
    let t = read("docs/drop-shortcut.md");
    let s = after_alt_tap(&t, "Alt tap");
    let low = s.to_lowercase();
    for needle in ["clipboard", "kawaro-open-selection", "at-spi", "measured", "recommendation"] {
        assert!(low.contains(needle), "missing `{needle}` in the Alt tap section");
    }
    assert!(
        low.contains("cannot read") && low.contains("selection"),
        "must state that a Wayland client cannot read a file manager selection"
    );
}

#[test]
fn ac3_arabic_copy_has_the_same_names_and_the_labels() {
    let t = read("docs/drop-shortcut.ar.md");
    let at = t
        .lines()
        .position(|l| l.contains('#') && l.contains("Alt") && l.contains("النقرة"))
        .expect("an Arabic heading with Alt and النقرة");
    let s = t.lines().skip(at).collect::<Vec<_>>().join("\n");
    for needle in ["kawaro-open-selection", "AT-SPI", "التوصية", "قيس"] {
        assert!(s.contains(needle), "missing `{needle}` in the Arabic Alt tap section");
    }
    assert!(s.to_lowercase().contains("clipboard") || s.contains("الحافظة"));
}

#[test]
fn ac3_both_files_keep_the_language_switch_and_have_no_long_dash() {
    for f in ["docs/drop-shortcut.md", "docs/drop-shortcut.ar.md"] {
        let t = read(f);
        assert_eq!(
            t.lines().next().unwrap(),
            "[English](drop-shortcut.md) | [العربية](drop-shortcut.ar.md)"
        );
        assert!(!t.contains('\u{2014}') && !t.contains('\u{2013}'), "{f} has a long dash");
    }
}
