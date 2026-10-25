//! S-019a (backlog/S-019-hive-at-pointer.md, AC1): the `hive_surface` key in
//! `config.toml`. Values `auto`, `layer` or `window`; absent means `auto`.
//!
//! Contract (written by the tester from the acceptance criteria; do not edit):
//! - `kawaro_core::config::HiveSurface { Auto, Layer, Window }`, with
//!   `Default` = `Auto`, `Clone`, `Copy`, `Debug`, `PartialEq`.
//! - `Config::hive_surface: HiveSurface`.
//! - The key in the file is spelled `hive_surface`, with an underscore, as
//!   the story writes it, although the other keys of `Config` are kebab case
//!   (`output-folder`). A file with `hive_surface = "window"` must give
//!   `Window`, and saving writes the same spelling back.
//! - A value outside the three is a broken file: defaults plus a warning, as
//!   for every other key today.

use kawaro_core::config::{Config, HiveSurface};

fn load(text: &str) -> (Config, Option<String>) {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("config.toml");
    std::fs::write(&p, text).unwrap();
    Config::load_from(&p)
}

#[test]
fn ac1_hive_surface_default_is_auto() {
    assert_eq!(HiveSurface::default(), HiveSurface::Auto);
    assert_eq!(Config::default().hive_surface, HiveSurface::Auto);
}

#[test]
fn ac1_hive_surface_absent_is_auto() {
    let (c, warn) = load("language = \"en\"\ntheme = \"auto\"\n");
    assert!(warn.is_none(), "{warn:?}");
    assert_eq!(c.hive_surface, HiveSurface::Auto);
}

#[test]
fn ac1_hive_surface_auto() {
    let (c, warn) = load("hive_surface = \"auto\"\n");
    assert!(warn.is_none(), "{warn:?}");
    assert_eq!(c.hive_surface, HiveSurface::Auto);
}

#[test]
fn ac1_hive_surface_layer() {
    let (c, warn) = load("language = \"en\"\nhive_surface = \"layer\"\n");
    assert!(warn.is_none(), "{warn:?}");
    assert_eq!(c.hive_surface, HiveSurface::Layer);
    assert_eq!(c.language, kawaro_core::config::LangSetting::En);
}

#[test]
fn ac1_hive_surface_window() {
    let (c, warn) = load("hive_surface = \"window\"\ntheme = \"dark\"\n");
    assert!(warn.is_none(), "{warn:?}");
    assert_eq!(c.hive_surface, HiveSurface::Window);
    assert_eq!(c.theme, kawaro_core::config::Theme::Dark);
}

#[test]
fn ac1_hive_surface_unknown_value_falls_back_with_a_warning() {
    let (c, warn) = load("hive_surface = \"overlay\"\n");
    assert_eq!(c, Config::default());
    assert!(warn.is_some_and(|w| w.contains("config.toml")));
}

#[test]
fn ac1_hive_surface_saves_under_the_same_key() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("config.toml");
    let c = Config {
        hive_surface: HiveSurface::Window,
        ..Config::default()
    };
    c.save_to(&p).unwrap();
    let text = std::fs::read_to_string(&p).unwrap();
    assert!(
        text.lines()
            .any(|l| l.replace(' ', "") == "hive_surface=\"window\""),
        "saved file lacks hive_surface = \"window\":\n{text}"
    );
    let (back, warn) = Config::load_from(&p);
    assert!(warn.is_none(), "{warn:?}");
    assert_eq!(back, c);
}
