//! S-108 (backlog/S-108-s096b-hive-cells.md), core strings that the hive
//! cells need: AC3 (`more_label` for the `+K` cell) and AC8 (`skipped_label`,
//! the accessible text of a greyed cell).
//!
//! Hooks these tests pin (module `kawaro_core::unsupported`):
//! - `skipped_label(ext: &str, reason: &str, lang: Lang) -> String`; `ext`
//!   comes without the dot, as `media::extension` gives it. English is
//!   exactly `.xyz: unavailable. <reason>`; Arabic holds the extension, the
//!   words `غير متاح` and the reason.
//! - `more_label(k: usize, lang: Lang) -> String`; English is exactly
//!   `K more files are not supported`; Arabic holds K and no Arabic-Indic
//!   digit.

use kawaro_core::i18n::Lang;
use kawaro_core::unsupported::{more_label, skipped_label, unsupported_message};

fn no_long_dash(s: &str) {
    assert!(!s.contains('\u{2013}') && !s.contains('\u{2014}'), "{s}");
}

fn western_digits_only(s: &str) {
    assert!(
        !s.chars().any(|c| ('\u{0660}'..='\u{0669}').contains(&c)),
        "{s}"
    );
}

#[test]
fn ac8_skipped_label_english_is_exact() {
    let reason = unsupported_message(Lang::En);
    assert_eq!(
        skipped_label("xyz", reason, Lang::En),
        format!(".xyz: unavailable. {reason}")
    );
}

#[test]
fn ac8_skipped_label_arabic_has_extension_state_and_reason() {
    let reason = unsupported_message(Lang::Ar);
    let label = skipped_label("xyz", reason, Lang::Ar);
    for part in ["xyz", "غير متاح", reason] {
        assert!(label.contains(part), "`{part}` missing in `{label}`");
    }
    no_long_dash(&label);
    western_digits_only(&label);
}

#[test]
fn ac3_more_label_english_is_exact() {
    assert_eq!(more_label(3, Lang::En), "3 more files are not supported");
    assert_eq!(more_label(1, Lang::En), "1 more files are not supported");
}

#[test]
fn ac8_more_label_arabic_has_the_count_in_western_digits() {
    for k in [1usize, 3, 12] {
        let label = more_label(k, Lang::Ar);
        assert!(label.contains(&k.to_string()), "{label}");
        assert!(
            label
                .chars()
                .any(|c| ('\u{0600}'..='\u{06FF}').contains(&c))
        );
        western_digits_only(&label);
        no_long_dash(&label);
    }
}
