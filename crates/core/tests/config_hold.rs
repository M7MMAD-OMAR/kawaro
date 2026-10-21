//! S-019e (backlog/S-019-hive-at-pointer.md, AC18; manager decision 10
//! approved as proposed): the `hold_ms` key in `config.toml`, the long press
//! threshold in milliseconds. Default 400, accepted range 100 to 2000 (no 0
//! for "at once": that is `sbarhive drop` without `--hold`).
//!
//! Contract (written by the tester from the acceptance criteria; do not edit):
//! - `Config::hold_ms`, an unsigned integer type (`u64::from` must take it;
//!   `u16`, `u32` or `u64`).
//! - The key in the file is spelled `hold_ms`, with an underscore, as the
//!   story writes it and as `hive_surface` is (S-019a), although the other
//!   keys of `Config` are kebab case. Saving writes the same spelling back.
//! - Absent: 400, no warning.
//! - Out of range or not an integer: `hold_ms` is 400 and there is a warning
//!   (one `Some`), as for every other key. Whether the other keys keep their
//!   values then is not fixed by the story, so it is not tested.

use sbarhive_core::config::{Config, LangSetting, Theme};

fn load(text: &str) -> (Config, Option<String>) {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("config.toml");
    std::fs::write(&p, text).unwrap();
    Config::load_from(&p)
}

fn ms(c: &Config) -> u64 {
    u64::from(c.hold_ms)
}

#[test]
fn ac18_hold_ms_default_is_400() {
    assert_eq!(ms(&Config::default()), 400);
}

#[test]
fn ac18_hold_ms_absent_is_400_without_a_warning() {
    let (c, warn) = load("language = \"en\"\ntheme = \"dark\"\n");
    assert!(warn.is_none(), "{warn:?}");
    assert_eq!(ms(&c), 400);
    assert_eq!(c.theme, Theme::Dark);
}

#[test]
fn ac18_hold_ms_100_and_2000_are_accepted() {
    for v in [100u64, 101, 400, 750, 1999, 2000] {
        let (c, warn) = load(&format!("language = \"ar\"\nhold_ms = {v}\n"));
        assert!(warn.is_none(), "hold_ms = {v}: {warn:?}");
        assert_eq!(ms(&c), v);
        assert_eq!(
            c.language,
            LangSetting::Ar,
            "hold_ms = {v}: other keys kept"
        );
    }
}

#[test]
fn ac18_hold_ms_out_of_range_or_not_an_integer_is_400_with_a_warning() {
    for bad in ["99", "0", "2001", "60000", "-5", "\"400\"", "400.0", "true"] {
        let (c, warn) = load(&format!("hold_ms = {bad}\n"));
        assert_eq!(ms(&c), 400, "hold_ms = {bad}");
        assert!(warn.is_some(), "hold_ms = {bad}: one warning in the log");
    }
}

#[test]
fn ac18_hold_ms_saves_under_the_same_key() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("config.toml");
    let mut c = Config::default();
    let (loaded, _) = load("hold_ms = 1500\n");
    c.hold_ms = loaded.hold_ms;
    assert_eq!(ms(&c), 1500);
    c.save_to(&p).unwrap();
    let text = std::fs::read_to_string(&p).unwrap();
    assert!(
        text.lines().any(|l| l.replace(' ', "") == "hold_ms=1500"),
        "saved file lacks hold_ms = 1500:\n{text}"
    );
    let (back, warn) = Config::load_from(&p);
    assert!(warn.is_none(), "{warn:?}");
    assert_eq!(back, c);
}
