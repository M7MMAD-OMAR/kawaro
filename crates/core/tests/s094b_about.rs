//! S-094b (backlog/S-094b-about-page-actions.md): the pure core of the About
//! page actions. Written from the acceptance criteria only.
//!
//! Hooks these tests pin (new module `kawaro_core::about`):
//! - `LINKS: [(&str, &str); 2]` as `(id, url)`, ids `website` and `source`.
//! - `link_url(id: &str) -> Option<&'static str>`.
//! - `copy_text(version: &str) -> String`.
//! - `strings(Lang) -> AboutStrings`, a struct whose fields are exactly
//!   `website`, `source`, `copy_version`, `copied`, `check_updates`,
//!   `website_label` and `source_label` (each `String` or `&'static str`).
//!   `website_label` and `source_label` are the accessible labels and already
//!   hold the target: English is exactly `Website: <url>` and
//!   `Source code: <url>`. `check_updates` is the text and accessible label of
//!   the About page shortcut button (the planner's list named the other six;
//!   the check button needs its own string).
//! - Docs: one line in `CHANGELOG.md` under `[Unreleased]`, English then
//!   Arabic, naming S-094b.
//!
//! Needles are split with `concat!` so this file never matches itself.

use kawaro_core::about::{LINKS, copy_text, link_url, strings};
use kawaro_core::i18n::Lang;
use std::path::{Path, PathBuf};

const WEBSITE: &str = "https://kawaro.sbarah.com/";
const SOURCE: &str = "https://github.com/M7MMAD-OMAR/kawaro";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    let p = root().join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} must exist: {e}", p.display()))
}

/// The text above the first `#[cfg(test)]` line.
fn above_test(text: &str) -> String {
    text.lines()
        .take_while(|l| l.trim() != "#[cfg(test)]")
        .map(|l| format!("{l}\n"))
        .collect()
}

fn has_long_dash(s: &str) -> bool {
    s.contains('\u{2013}') || s.contains('\u{2014}')
}

fn has_indic_digit(s: &str) -> bool {
    s.chars()
        .any(|c| ('\u{0660}'..='\u{0669}').contains(&c) || ('\u{06F0}'..='\u{06F9}').contains(&c))
}

fn fields(lang: Lang) -> Vec<(&'static str, String)> {
    let s = strings(lang);
    vec![
        ("website", s.website.to_string()),
        ("source", s.source.to_string()),
        ("copy_version", s.copy_version.to_string()),
        ("copied", s.copied.to_string()),
        ("check_updates", s.check_updates.to_string()),
        ("website_label", s.website_label.to_string()),
        ("source_label", s.source_label.to_string()),
    ]
}

#[test]
fn ac1_link_url_resolves_the_two_ids() {
    assert_eq!(link_url("website"), Some(WEBSITE));
    assert_eq!(link_url("source"), Some(SOURCE));
    assert_eq!(LINKS.len(), 2);
    let ids: Vec<&str> = LINKS.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, ["website", "source"]);
}

#[test]
fn ac1_link_url_is_none_for_anything_else() {
    for id in ["", "file", "https://x", "website ", "Website", "SOURCE"] {
        assert_eq!(link_url(id), None, "`{id}`");
    }
}

#[test]
fn ac3_copy_text_is_the_version_character_for_character() {
    assert_eq!(copy_text("0.4.7"), "0.4.7");
    let v = env!("CARGO_PKG_VERSION");
    let t = copy_text(v);
    assert_eq!(t, v);
    assert!(!t.is_empty());
    for bad in ['/', '\\', ' ', '~'] {
        assert!(!t.contains(bad), "`{t}` holds `{bad}`");
    }
}

#[test]
fn ac1_ac5_english_labels_name_the_full_target() {
    let s = strings(Lang::En);
    assert_eq!(s.website_label.to_string(), format!("Website: {WEBSITE}"));
    assert_eq!(s.source_label.to_string(), format!("Source code: {SOURCE}"));
    assert_eq!(s.website.to_string(), "Website");
    assert_eq!(s.source.to_string(), "Source code");
    assert_eq!(s.copy_version.to_string(), "Copy version");
    assert_eq!(s.copied.to_string(), "Copied");
    assert_eq!(s.check_updates.to_string(), "Check for updates");
}

#[test]
fn ac5_arabic_labels_hold_the_target_and_arabic_words() {
    let s = strings(Lang::Ar);
    assert!(s.website_label.to_string().contains(WEBSITE));
    assert!(s.source_label.to_string().contains(SOURCE));
    for (name, text) in fields(Lang::Ar) {
        assert!(
            text.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c)),
            "{name} holds no Arabic: {text}"
        );
    }
}

#[test]
fn ac5_every_field_is_filled_and_the_two_languages_differ_in_each() {
    let (en, ar) = (fields(Lang::En), fields(Lang::Ar));
    assert_eq!(en.len(), ar.len());
    for ((name, e), (_, a)) in en.iter().zip(ar.iter()) {
        assert!(!e.trim().is_empty(), "en {name} is empty");
        assert!(!a.trim().is_empty(), "ar {name} is empty");
        assert_ne!(e, a, "{name} is the same in both languages");
    }
}

#[test]
fn ac5_no_arabic_indic_digits_and_no_long_dashes_in_any_string() {
    for lang in [Lang::En, Lang::Ar] {
        for (name, text) in fields(lang) {
            assert!(!has_indic_digit(&text), "{lang:?} {name}: {text}");
            assert!(!has_long_dash(&text), "{lang:?} {name}: {text}");
        }
    }
}

#[test]
fn ac2_ac6_the_core_module_is_pure_and_holds_no_hand_written_values() {
    let text = above_test(&read("crates/core/src/about.rs"));
    let needles = [
        concat!("std::", "process"),
        concat!("std::", "fs"),
        concat!("std::", "net"),
        concat!("Com", "mand"),
        concat!("xdg", "-open"),
        concat!("explorer", ".exe"),
        concat!("drop", "-shadow"),
        concat!("gra", "dient"),
    ];
    for n in needles {
        assert_eq!(text.matches(n).count(), 0, "about.rs holds `{n}`");
    }
    assert!(!has_long_dash(&text), "about.rs holds a long dash");
    // No colour literal and no number glued to px or ms.
    for w in text.as_bytes().windows(2) {
        assert!(
            !(w[0] == b'#' && w[1].is_ascii_hexdigit()),
            "colour literal in about.rs"
        );
    }
    for unit in ["px", "ms"] {
        for (i, _) in text.match_indices(unit) {
            let prev = text[..i].chars().last();
            let after = text[i + 2..].chars().next();
            let glued = prev.is_some_and(|c| c.is_ascii_digit());
            let word_end = !after.is_some_and(|c| c.is_alphanumeric() || c == '_');
            assert!(!(glued && word_end), "number with `{unit}` in about.rs");
        }
    }
}

#[test]
fn ac7_the_changelog_has_one_bilingual_line_for_s094b() {
    let text = read("CHANGELOG.md");
    // The line sat under [Unreleased] until the 0.4.8 cut; the check keeps
    // guarding the entry form and no longer pins the section a cut moves it out of.
    let lines: Vec<&str> = text
        .lines()
        .filter(|l| l.starts_with("- ") && l.contains("S-094b"))
        .collect();
    assert_eq!(lines.len(), 1, "exactly one S-094b line: {lines:?}");
    let l = lines[0];
    assert!(l.contains(" | "), "English | Arabic like the other lines");
    assert!(
        l.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c)),
        "the line carries the Arabic text"
    );
    assert!(!has_long_dash(l));
}
