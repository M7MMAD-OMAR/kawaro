//! S-135 AC3 and AC4: the drop mode setting.
//! Contract pinned here (tester): kawaro_core::config::DropMode with
//! DropFirst (default) and ChooseDuringDrag, serde kebab-case values
//! "drop-first" and "choose-during-drag"; Config gains drop_mode with that
//! default; an unknown value falls back to drop-first without failing the
//! file; a save round trip keeps the mode.

use kawaro_core::config::{Config, DropMode};

fn load(text: &str) -> (Config, Option<String>) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, text).unwrap();
    Config::load_from(&path)
}

#[test]
fn ac4_the_default_mode_is_drop_first() {
    assert_eq!(DropMode::default(), DropMode::DropFirst);
    assert_eq!(Config::default().drop_mode, DropMode::DropFirst);
    assert_eq!(load("").0.drop_mode, DropMode::DropFirst);
}

#[test]
fn ac4_the_mode_is_spelled_kebab_case_in_config_toml() {
    assert_eq!(
        serde_json::to_string(&DropMode::DropFirst).unwrap(),
        "\"drop-first\""
    );
    assert_eq!(
        serde_json::to_string(&DropMode::ChooseDuringDrag).unwrap(),
        "\"choose-during-drag\""
    );
    let (config, warning) = load("drop_mode = \"choose-during-drag\"\n");
    assert_eq!(warning, None);
    assert_eq!(config.drop_mode, DropMode::ChooseDuringDrag);
    assert_eq!(
        load("drop_mode = \"drop-first\"\n").0.drop_mode,
        DropMode::DropFirst
    );
}

#[test]
fn ac4_an_unknown_mode_falls_back_to_drop_first() {
    let (config, warning) = load("drop_mode = \"sideways\"\n");
    assert_eq!(
        config.drop_mode,
        DropMode::DropFirst,
        "unknown modes fall back without stopping the daemon"
    );
    assert_eq!(warning, None);
}

#[test]
fn ac4_a_save_round_trip_keeps_the_mode() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let config = Config {
        drop_mode: DropMode::ChooseDuringDrag,
        ..Config::default()
    };
    config.save_to(&path).unwrap();
    let (loaded, warning) = Config::load_from(&path);
    assert_eq!(warning, None);
    assert_eq!(loaded.drop_mode, DropMode::ChooseDuringDrag);
}
