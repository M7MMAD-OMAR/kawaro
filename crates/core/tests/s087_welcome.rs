//! S-087 first-run welcome (backlog/S-087-first-run-welcome.md): the pure
//! core of the welcome flow. AC1 routes the `settings` entry on the
//! one-shot `welcome_seen` marker, AC3 resolves the demonstration captions
//! from the registration result only, and AC4 and AC5 pin the bilingual
//! window strings and the forbidden words. Nothing here needs a window.

use kawaro_core::config::Config;
use kawaro_core::i18n::Lang;
use kawaro_core::welcome::{self, Registration, Route, Strings};

/// The words AC5 forbids in every visible string of the welcome window.
const FORBIDDEN: [&str; 6] = ["portal", "daemon", "registry", "config", "ipc", "token"];

fn all_registrations() -> Vec<Registration> {
    vec![
        Registration::RightDrag,
        Registration::Native("Alt".into()),
        Registration::Fallback("Super+Z".into()),
        Registration::Unbound,
    ]
}

fn every_string() -> Vec<String> {
    let mut out = Vec::new();
    for lang in [Lang::En, Lang::Ar] {
        let Strings {
            title,
            try_button,
            try_accessible,
            settings_button,
            settings_accessible,
        } = welcome::strings(lang);
        out.extend([
            title,
            try_button,
            try_accessible,
            settings_button,
            settings_accessible,
        ]);
        for reg in all_registrations() {
            out.extend(welcome::captions(lang, &reg).steps);
        }
    }
    out
}

// ------------------------------------------------------------ AC1

#[test]
fn ac1_a_missing_marker_routes_the_settings_entry_to_the_welcome() {
    assert_eq!(welcome::route(&Config::default()), Route::Welcome);
}

#[test]
fn ac1_the_marker_routes_the_settings_entry_to_the_settings_window() {
    let c = Config {
        welcome_seen: true,
        ..Config::default()
    };
    assert_eq!(welcome::route(&c), Route::Settings);
}

#[test]
fn ac1_the_marker_is_one_shot_and_written_only_as_true() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("config.toml");

    // No other write path creates a marker: a saved config with the marker
    // unset carries no `welcome_seen` key at all.
    Config::default().save_to(&p).unwrap();
    let text = std::fs::read_to_string(&p).unwrap();
    assert!(
        !text.contains("welcome_seen"),
        "an unset marker must not be written: {text}"
    );

    // The welcome writes it once, as true, and it survives loading.
    let c = Config {
        welcome_seen: true,
        ..Config::default()
    };
    c.save_to(&p).unwrap();
    let text = std::fs::read_to_string(&p).unwrap();
    assert!(
        text.contains("welcome_seen = true"),
        "the marker is written as true: {text}"
    );
    let (loaded, warn) = Config::load_from(&p);
    assert!(warn.is_none());
    assert!(loaded.welcome_seen);
    assert_eq!(welcome::route(&loaded), Route::Settings);
}

#[test]
fn ac1_a_config_written_by_another_path_does_not_hide_the_welcome() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("config.toml");

    // A portal denial save (S-014f): the daemon writes `portal_shortcut =
    // false` into a config it loaded.
    let c = Config {
        portal_shortcut: false,
        ..Config::default()
    };
    c.save_to(&p).unwrap();
    assert_eq!(welcome::route(&Config::load_from(&p).0), Route::Welcome);

    // A settings save with several values the form shows.
    let mut c = Config {
        theme: kawaro_core::config::Theme::Dark,
        ..Config::default()
    };
    c.output_folder = "Out".into();
    c.save_to(&p).unwrap();
    assert_eq!(welcome::route(&Config::load_from(&p).0), Route::Welcome);

    // An updater write (the channel the updater records).
    let mut c = Config {
        update_channel: kawaro_core::update::Channel::Beta,
        ..Config::default()
    };
    c.update_channel_present = true;
    c.save_to(&p).unwrap();
    assert_eq!(welcome::route(&Config::load_from(&p).0), Route::Welcome);
}

#[test]
fn ac1_a_saved_marker_stays_set_across_later_writes() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("config.toml");
    let c = Config {
        welcome_seen: true,
        ..Config::default()
    };
    c.save_to(&p).unwrap();
    // A later settings save of the loaded config keeps the marker.
    let (mut loaded, _) = Config::load_from(&p);
    loaded.notify = false;
    loaded.save_to(&p).unwrap();
    let (again, _) = Config::load_from(&p);
    assert!(again.welcome_seen);
    assert_eq!(welcome::route(&again), Route::Settings);
}

// ------------------------------------------------------------ AC3

#[test]
fn ac3_the_windows_trigger_step_names_the_right_button_in_both_languages() {
    let en = welcome::captions(Lang::En, &Registration::RightDrag);
    let ar = welcome::captions(Lang::Ar, &Registration::RightDrag);
    assert!(
        en.steps[1].to_lowercase().contains("right button"),
        "the Windows step names the S-084 winner: {:?}",
        en.steps[1]
    );
    assert!(
        ar.steps[1].contains("الزر الأيمن"),
        "the Arabic Windows step names the right button: {:?}",
        ar.steps[1]
    );
    // It names no key at all (S-084 D3).
    for step in [&en.steps[1], &ar.steps[1]] {
        assert!(!step.contains('+'), "no key chord in {step:?}");
    }
}

#[test]
fn ac3_the_caption_names_the_key_that_works_not_the_configured_one() {
    // The configured drop key is Alt; the live registration is the named
    // portal fallback. The caption names the registration, never the config:
    // `captions` has no config parameter at all, and the registered key is
    // what shows.
    let en = welcome::captions(Lang::En, &Registration::Fallback("Super+Z".into()));
    let ar = welcome::captions(Lang::Ar, &Registration::Fallback("Super+Z".into()));
    for step in [&en.steps[1], &ar.steps[1]] {
        assert!(
            step.contains("Super+Z"),
            "the fallback key is named: {step:?}"
        );
        assert!(
            !step.to_lowercase().contains("alt"),
            "the configured Alt never shows when it is not registered: {step:?}"
        );
    }
}

#[test]
fn ac3_a_native_registration_names_its_key() {
    for key in ["Alt", "Super+Z"] {
        let reg = Registration::Native(key.into());
        for lang in [Lang::En, Lang::Ar] {
            let c = welcome::captions(lang, &reg);
            assert!(
                c.steps[1].contains(key),
                "{lang:?} names the measured key {key}: {:?}",
                c.steps[1]
            );
        }
    }
}

#[test]
fn ac3_no_registration_falls_back_to_key_free_wording() {
    let en = welcome::captions(Lang::En, &Registration::Unbound);
    let ar = welcome::captions(Lang::Ar, &Registration::Unbound);
    let lower = en.steps[1].to_lowercase();
    assert!(lower.contains("drag"), "key-free still says drag: {en:?}");
    assert!(lower.contains("drop"), "key-free still says drop: {en:?}");
    for step in [&en.steps[1], &ar.steps[1]] {
        for name in ["alt", "super", "ctrl", "shift", "win"] {
            assert!(
                !step.to_lowercase().contains(name),
                "key-free names no key: {step:?}"
            );
        }
        assert!(!step.contains('+'), "key-free names no chord: {step:?}");
    }
    assert!(
        ar.steps[1]
            .chars()
            .any(|c| ('\u{0621}'..='\u{064A}').contains(&c)),
        "Arabic letters in {:?}",
        ar.steps[1]
    );
}

#[test]
fn ac3_the_captions_are_five_steps_in_both_languages() {
    for reg in all_registrations() {
        for lang in [Lang::En, Lang::Ar] {
            let c = welcome::captions(lang, &reg);
            assert_eq!(c.steps.len(), 5, "{lang:?} {reg:?}");
            for step in &c.steps {
                assert!(!step.trim().is_empty(), "empty step in {lang:?} {reg:?}");
                assert_eq!(step, step.trim(), "padding in {step:?}");
                assert!(
                    !step.contains('\u{2014}') && !step.contains('\u{2013}'),
                    "no long dash in {step:?}"
                );
                if lang == Lang::Ar {
                    assert!(
                        step.chars().any(|c| ('\u{0621}'..='\u{064A}').contains(&c)),
                        "Arabic letters in {step:?}"
                    );
                } else {
                    assert!(
                        step.chars().any(|c| c.is_ascii_alphabetic()),
                        "Latin letters in {step:?}"
                    );
                    assert!(
                        !step.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c)),
                        "no Arabic letters in the English step {step:?}"
                    );
                }
            }
            let mut distinct = c.steps.to_vec();
            distinct.sort();
            distinct.dedup();
            assert_eq!(distinct.len(), 5, "duplicate steps in {lang:?} {reg:?}");
        }
    }
}

// ------------------------------------------------------------ AC4, AC5

#[test]
fn ac4_the_try_button_wording_is_pinned_in_both_languages() {
    assert_eq!(welcome::strings(Lang::En).try_button, "Try on an image");
    assert_eq!(welcome::strings(Lang::Ar).try_button, "جرّب على صورة");
}

#[test]
fn ac5_the_settings_button_wording_is_pinned_in_both_languages() {
    assert_eq!(welcome::strings(Lang::En).settings_button, "Open Settings");
    assert_eq!(welcome::strings(Lang::Ar).settings_button, "افتح الإعدادات");
}

#[test]
fn ac5_no_welcome_string_contains_a_forbidden_word() {
    for s in every_string() {
        let lower = s.to_lowercase();
        for word in FORBIDDEN {
            assert!(
                !lower.contains(word),
                "the forbidden word {word:?} appears in {s:?}"
            );
        }
    }
}

#[test]
fn ac4_the_accessible_labels_are_descriptive_and_bilingual() {
    for lang in [Lang::En, Lang::Ar] {
        let s = welcome::strings(lang);
        for (label, button) in [
            (&s.try_accessible, &s.try_button),
            (&s.settings_accessible, &s.settings_button),
        ] {
            assert!(
                !label.trim().is_empty(),
                "empty accessible label in {lang:?}"
            );
            assert_ne!(label, button, "the label says more than the button text");
            assert!(
                !label.contains('\u{2014}') && !label.contains('\u{2013}'),
                "no long dash in {label:?}"
            );
        }
        // The try label names what is tried: an image.
        if lang == Lang::En {
            assert!(
                s.try_accessible.to_lowercase().contains("image"),
                "the try label names the image: {:?}",
                s.try_accessible
            );
        } else {
            assert!(
                s.try_accessible.contains("صورة"),
                "the try label names the image: {:?}",
                s.try_accessible
            );
        }
    }
    let (en, ar) = (welcome::strings(Lang::En), welcome::strings(Lang::Ar));
    assert_ne!(en.try_accessible, ar.try_accessible);
    assert_ne!(en.settings_accessible, ar.settings_accessible);
    assert!(
        ar.settings_accessible
            .chars()
            .any(|c| ('\u{0621}'..='\u{064A}').contains(&c))
    );
    assert!(
        !en.settings_accessible
            .chars()
            .any(|c| ('\u{0600}'..='\u{06FF}').contains(&c))
    );
}

#[test]
fn ac5_the_window_title_is_bilingual() {
    let (en, ar) = (
        welcome::strings(Lang::En).title,
        welcome::strings(Lang::Ar).title,
    );
    assert!(!en.trim().is_empty() && !ar.trim().is_empty());
    assert_ne!(en, ar);
    assert!(
        ar.chars().any(|c| ('\u{0621}'..='\u{064A}').contains(&c)),
        "Arabic letters in {ar:?}"
    );
    assert!(
        !en.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c)),
        "no Arabic letters in {en:?}"
    );
}
