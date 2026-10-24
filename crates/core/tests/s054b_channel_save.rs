//! S-054b manager ruling (3): the beta default channel survives a Settings
//! save. Contract for the implementer:
//! - `Config` gains `pub update_channel_present: bool`, `#[serde(skip)]`,
//!   default false, set to true by `Config::load_from` when the file text has
//!   an `update_channel` key.
//! - `Config::save_to` writes `update_channel` only when
//!   `update_channel_present` is true or `update_channel != Channel::Stable`.
//! - The flag must NOT take part in `PartialEq` (hand written `PartialEq`, or
//!   the flag in a wrapper type that always compares equal): otherwise
//!   `ac8_keys_are_saved_with_underscores_and_round_trip` in
//!   `s053a_update.rs` (`Config::load_from(&p) == (c, None)` where `c` is a
//!   literal with Beta and flag false, loaded back with flag true) fails.
//!
//! Ruling (4): the `RepoUpgrade` plan text starts with the absolute
//! `/usr/bin/pkexec` (the plan carries text only; `apply_now` splits it).

use std::fs;
use std::path::Path;

use sbarhive_core::config::{Config, UpdateMode};
use sbarhive_core::update::Channel;
use sbarhive_core::update::apply::{Facts, PkgKind, Plan, linux_plan};

fn load(dir: &Path, text: &str) -> Config {
    let p = dir.join("config.toml");
    fs::write(&p, text).unwrap();
    let (c, w) = Config::load_from(&p);
    assert_eq!(w, None);
    c
}

fn save(dir: &Path, c: &Config) -> String {
    let p = dir.join("saved.toml");
    c.save_to(&p).unwrap();
    fs::read_to_string(&p).unwrap()
}

fn has_channel_line(text: &str) -> bool {
    text.lines()
        .any(|l| l.trim_start().starts_with("update_channel"))
}

#[test]
fn a_file_without_the_key_is_saved_without_it() {
    let t = tempfile::tempdir().unwrap();
    let c = load(t.path(), "hold_ms = 500\n");
    assert!(!c.update_channel_present);
    let saved = save(t.path(), &c);
    assert!(!has_channel_line(&saved), "{saved}");
    assert_eq!(Config::explicit_update_channel(&saved), None);
    // The default Config behaves the same.
    let saved = save(t.path(), &Config::default());
    assert!(!has_channel_line(&saved), "{saved}");
}

#[test]
fn an_explicit_stable_stays_written_and_explicit() {
    let t = tempfile::tempdir().unwrap();
    let c = load(t.path(), "update_channel = \"stable\"\n");
    assert!(c.update_channel_present);
    assert_eq!(c.update_channel, Channel::Stable);
    let saved = save(t.path(), &c);
    assert!(
        saved
            .lines()
            .any(|l| l.trim() == "update_channel = \"stable\""),
        "{saved}"
    );
    assert_eq!(
        Config::explicit_update_channel(&saved),
        Some(Channel::Stable)
    );
}

#[test]
fn setting_beta_on_a_default_config_writes_it() {
    let t = tempfile::tempdir().unwrap();
    let c = Config {
        update_channel: Channel::Beta,
        ..Config::default()
    };
    assert!(!c.update_channel_present);
    let saved = save(t.path(), &c);
    assert!(
        saved
            .lines()
            .any(|l| l.trim() == "update_channel = \"beta\""),
        "{saved}"
    );
    assert_eq!(Config::explicit_update_channel(&saved), Some(Channel::Beta));
}

#[test]
fn an_explicit_beta_is_written_and_the_flag_is_set_on_the_next_load() {
    let t = tempfile::tempdir().unwrap();
    let c = load(t.path(), "update_channel = \"beta\"\n");
    assert!(c.update_channel_present);
    let saved = save(t.path(), &c);
    assert_eq!(Config::explicit_update_channel(&saved), Some(Channel::Beta));
    assert!(load(t.path(), &saved).update_channel_present);
}

#[test]
fn three_round_trips_of_a_file_without_the_key_keep_it_out() {
    let t = tempfile::tempdir().unwrap();
    let mut text = "hold_ms = 700\nupdate_mode = \"auto\"\n".to_string();
    for round in 0..3 {
        let c = load(t.path(), &text);
        assert!(!c.update_channel_present, "round {round}");
        text = save(t.path(), &c);
        assert!(!has_channel_line(&text), "round {round}: {text}");
        assert_eq!(Config::explicit_update_channel(&text), None);
    }
}

#[test]
fn the_other_keys_still_round_trip() {
    let t = tempfile::tempdir().unwrap();
    let c = load(
        t.path(),
        "hold_ms = 700\nupdate_check = false\nupdate_mode = \"auto\"\n",
    );
    let saved = save(t.path(), &c);
    let back = load(t.path(), &saved);
    assert_eq!(back.hold_ms, 700);
    assert!(!back.update_check);
    assert_eq!(back.update_mode, UpdateMode::Auto);
    assert_eq!(back.update_channel, Channel::Stable);
    assert!(
        saved.lines().any(|l| l.trim() == "update_check = false"),
        "{saved}"
    );
    assert!(
        saved.lines().any(|l| l.trim() == "update_mode = \"auto\""),
        "{saved}"
    );
    assert!(
        saved.lines().any(|l| l.trim() == "hold_ms = 700"),
        "{saved}"
    );
}

#[test]
fn the_repo_upgrade_plan_uses_the_absolute_pkexec() {
    let facts = |dnf, apt, kind| Facts {
        has_dnf: dnf,
        has_apt: apt,
        has_pkexec: true,
        repo_configured: true,
        installed_kind: Some(kind),
    };
    assert_eq!(
        linux_plan(&facts(true, false, PkgKind::Rpm)),
        Plan::RepoUpgrade("/usr/bin/pkexec dnf upgrade -y sbarhive".to_string())
    );
    assert_eq!(
        linux_plan(&facts(false, true, PkgKind::Deb)),
        Plan::RepoUpgrade("/usr/bin/pkexec apt-get install -y --only-upgrade sbarhive".to_string())
    );
}
