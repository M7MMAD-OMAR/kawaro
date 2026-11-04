//! S-130 AC3: the language screen strings come from kawaro_core::welcome.
//!
//! Contract pinned by this test (tester): Strings gains seven fields so the
//! language screen has no hardcoded text: language_title, language_subtitle,
//! language_failed, language_english, language_english_accessible,
//! language_arabic and language_arabic_accessible, in Arabic and English.

use kawaro_core::i18n::Lang;
use kawaro_core::welcome::strings;

fn has_arabic(text: &str) -> bool {
    text.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c))
}

fn has_latin(text: &str) -> bool {
    text.chars().any(|c| c.is_ascii_alphabetic())
}

fn check(lang: Lang, arabic: bool) {
    let s = strings(lang);
    let fields = [
        ("language_title", &s.language_title),
        ("language_subtitle", &s.language_subtitle),
        ("language_failed", &s.language_failed),
        ("language_english", &s.language_english),
        (
            "language_english_accessible",
            &s.language_english_accessible,
        ),
        ("language_arabic", &s.language_arabic),
        ("language_arabic_accessible", &s.language_arabic_accessible),
    ];
    for (name, value) in fields {
        assert!(!value.trim().is_empty(), "{name} must not be empty");
        assert!(
            !value.contains('\u{2013}') && !value.contains('\u{2014}'),
            "{name} has a long dash: {value}"
        );
    }
    if arabic {
        assert!(has_arabic(&s.language_title), "{}", s.language_title);
        assert!(has_arabic(&s.language_subtitle), "{}", s.language_subtitle);
        assert!(has_arabic(&s.language_failed), "{}", s.language_failed);
        assert!(has_arabic(&s.language_arabic), "{}", s.language_arabic);
    } else {
        assert!(has_latin(&s.language_title), "{}", s.language_title);
        assert!(has_latin(&s.language_subtitle), "{}", s.language_subtitle);
        assert!(has_latin(&s.language_failed), "{}", s.language_failed);
        assert!(has_latin(&s.language_english), "{}", s.language_english);
    }
    // The accessible labels say more than the visible name and stay distinct.
    assert_ne!(s.language_english, s.language_english_accessible);
    assert_ne!(s.language_arabic, s.language_arabic_accessible);
    assert_ne!(s.language_title, s.language_subtitle);
}

#[test]
fn the_language_screen_strings_exist_in_english() {
    check(Lang::En, false);
}

#[test]
fn the_language_screen_strings_exist_in_arabic() {
    check(Lang::Ar, true);
}

#[test]
fn the_two_languages_use_their_own_words() {
    let en = strings(Lang::En);
    let ar = strings(Lang::Ar);
    for (name, a, b) in [
        ("language_title", &en.language_title, &ar.language_title),
        (
            "language_subtitle",
            &en.language_subtitle,
            &ar.language_subtitle,
        ),
        ("language_failed", &en.language_failed, &ar.language_failed),
    ] {
        assert_ne!(a, b, "{name} must be translated, not copied");
    }
    // The two option names are the languages themselves, in their own script,
    // in both builds.
    for s in [&en, &ar] {
        assert!(
            !s.language_english.is_empty() && s.language_english.is_ascii(),
            "the English option keeps its Latin name: {}",
            s.language_english
        );
        assert!(
            has_arabic(&s.language_arabic),
            "the Arabic option keeps its Arabic name: {}",
            s.language_arabic
        );
    }
}
