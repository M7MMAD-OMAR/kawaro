//! S-014e1 (backlog/S-014-drop-hive.md, AC19 and the manager's answer 5 of
//! 2026-10-22): a denied portal consent is remembered in `config.toml`, so
//! the dialog does not return at every login, until the user turns it back
//! on in the settings.
//!
//! Contract (written by the tester from the acceptance criteria; do not edit):
//! - `Config::portal_shortcut: bool`, default `true` (register the portal
//!   shortcut at start).
//! - The key in the file is spelled `portal_shortcut`, with an underscore,
//!   as `hold_ms` and `hive_surface` are. Saving writes the same spelling.
//! - Absent: `true`, no warning. Not a boolean: the defaults and a warning,
//!   as for every other key.
//! - The daemon writes `portal_shortcut = false` after a `denied` reply,
//!   keeping the user's other keys (checked live by tests/drop/portal.sh).

use kawaro_core::config::Config;

fn load(text: &str) -> (Config, Option<String>) {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("config.toml");
    std::fs::write(&p, text).unwrap();
    Config::load_from(&p)
}

#[test]
fn ac19_portal_shortcut_defaults_to_on() {
    assert!(Config::default().portal_shortcut);
    let (c, w) = load("hold_ms = 500\n");
    assert!(c.portal_shortcut);
    assert_eq!(w, None);
}

#[test]
fn ac19_portal_shortcut_false_is_read_with_the_other_keys() {
    let (c, w) = load("hold_ms = 500\nportal_shortcut = false\n");
    assert_eq!(w, None);
    assert!(!c.portal_shortcut);
    assert_eq!(u64::from(c.hold_ms), 500);
    let (c, w) = load("portal_shortcut = true\n");
    assert_eq!(w, None);
    assert!(c.portal_shortcut);
}

#[test]
fn ac19_portal_shortcut_not_a_boolean_warns_and_keeps_the_default() {
    let (c, w) = load("portal_shortcut = \"no\"\n");
    assert!(w.is_some());
    assert!(c.portal_shortcut);
}

#[test]
fn ac19_portal_shortcut_is_saved_with_an_underscore() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("config.toml");
    let c = Config {
        portal_shortcut: false,
        ..Config::default()
    };
    c.save_to(&p).unwrap();
    let text = std::fs::read_to_string(&p).unwrap();
    assert!(
        text.lines().any(|l| l.trim() == "portal_shortcut = false"),
        "{text}"
    );
    assert!(!text.contains("portal-shortcut"), "{text}");
    let (back, w) = Config::load_from(&p);
    assert_eq!(w, None);
    assert_eq!(back, c);
}
