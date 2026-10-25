//! S-054b round 3: `Config` equality and `configured_update_channel`, pinned
//! so no mutant survives.

use std::fs;

use kawaro_core::config::{
    Config, HiveSurface, LangSetting, OutputPlace, Quality, Theme, UpdateMode,
};
use kawaro_core::update::Channel;

#[test]
fn equal_configs_are_equal_and_every_setting_breaks_equality() {
    let base = Config::default();
    assert_eq!(base, Config::default());
    let changes: Vec<(&str, Config)> = vec![
        (
            "language",
            Config {
                language: LangSetting::Ar,
                ..Config::default()
            },
        ),
        (
            "theme",
            Config {
                theme: Theme::Dark,
                ..Config::default()
            },
        ),
        (
            "output",
            Config {
                output: OutputPlace::Subfolder,
                ..Config::default()
            },
        ),
        (
            "output_folder",
            Config {
                output_folder: "Other".into(),
                ..Config::default()
            },
        ),
        (
            "quality",
            Config {
                quality: Quality::Max,
                ..Config::default()
            },
        ),
        (
            "open_folder_when_done",
            Config {
                open_folder_when_done: !base.open_folder_when_done,
                ..Config::default()
            },
        ),
        (
            "notify",
            Config {
                notify: !base.notify,
                ..Config::default()
            },
        ),
        (
            "hive_surface",
            Config {
                hive_surface: HiveSurface::Window,
                ..Config::default()
            },
        ),
        (
            "hold_ms",
            Config {
                hold_ms: base.hold_ms + 1,
                ..Config::default()
            },
        ),
        (
            "portal_shortcut",
            Config {
                portal_shortcut: !base.portal_shortcut,
                ..Config::default()
            },
        ),
        (
            "shortcut",
            Config {
                shortcut: "Ctrl+Alt+F9".into(),
                ..Config::default()
            },
        ),
        (
            "update_check",
            Config {
                update_check: !base.update_check,
                ..Config::default()
            },
        ),
        (
            "update_channel",
            Config {
                update_channel: Channel::Beta,
                ..Config::default()
            },
        ),
        (
            "update_mode",
            Config {
                update_mode: UpdateMode::Auto,
                ..Config::default()
            },
        ),
    ];
    for (name, other) in changes {
        assert_ne!(base, other, "{name} must break equality");
        assert_ne!(other, base, "{name} must break equality");
    }
}

#[test]
fn the_present_flag_alone_does_not_make_configs_unequal() {
    let a = Config::default();
    let b = Config {
        update_channel_present: true,
        ..Config::default()
    };
    assert!(b.update_channel_present && !a.update_channel_present);
    assert_eq!(a, b);
    assert_eq!(b, a);
}

fn load(text: &str) -> Config {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("config.toml");
    fs::write(&p, text).unwrap();
    Config::load_from(&p).0
}

#[test]
fn configured_update_channel_covers_every_branch() {
    // Key present: its value, stable included.
    assert_eq!(
        load("update_channel = \"stable\"\n").configured_update_channel(),
        Some(Channel::Stable)
    );
    assert_eq!(
        load("update_channel = \"beta\"\n").configured_update_channel(),
        Some(Channel::Beta)
    );
    // Absent: none, whatever else the file holds.
    assert_eq!(Config::default().configured_update_channel(), None);
    assert_eq!(load("hold_ms = 500\n").configured_update_channel(), None);
    // Invalid value: the whole file falls back to the defaults.
    assert_eq!(
        load("update_channel = \"nightly\"\n").configured_update_channel(),
        None
    );
    // Set in code to a non default value without the file flag.
    let beta = Config {
        update_channel: Channel::Beta,
        ..Config::default()
    };
    assert!(!beta.update_channel_present);
    assert_eq!(beta.configured_update_channel(), Some(Channel::Beta));
    // Flag set with the default value counts as chosen.
    let stable = Config {
        update_channel_present: true,
        ..Config::default()
    };
    assert_eq!(stable.configured_update_channel(), Some(Channel::Stable));
}

#[test]
fn a_configured_channel_decides_the_effective_channel_and_none_follows_the_build() {
    use kawaro_core::update::effective_channel;
    let none = Config::default().configured_update_channel();
    assert_eq!(effective_channel(none, "0.4.0-beta.1"), Channel::Beta);
    assert_eq!(effective_channel(none, "0.4.0"), Channel::Stable);
    let chosen = load("update_channel = \"stable\"\n").configured_update_channel();
    assert_eq!(effective_channel(chosen, "0.4.0-beta.1"), Channel::Stable);
    let chosen = load("update_channel = \"beta\"\n").configured_update_channel();
    assert_eq!(effective_channel(chosen, "0.4.0"), Channel::Beta);
}
