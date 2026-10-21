//! S-019a (backlog/S-019-hive-at-pointer.md, AC1 and AC2): the screen reader
//! rule and the surface choice with its reason. No bus, no compositor, no
//! environment: every input is a parameter, so this runs under
//! `cargo mutants` (a11y.rs and layer.rs are excluded from it, so the rule
//! and the reasons must live in overlay.rs).
//!
//! Contract (written by the tester from the acceptance criteria; do not edit):
//! - `overlay::screen_reader_running(flag: bool, reader: bool,
//!   config: HiveSurface) -> bool`: `flag` is `org.a11y.Status.
//!   ScreenReaderEnabled`, `reader` is whether `org.gnome.Orca.Service` has
//!   an owner on the session bus, `config` is `hive_surface` from
//!   config.toml (`sbarhive_core::config::HiveSurface`). True only with the
//!   flag and the owner together and `Auto`.
//! - `overlay::surface_choice(globals: &[&str], wayland: bool, flag: bool,
//!   reader: bool, config: HiveSurface, forced: Option<&str>)
//!   -> (Surface, String)`: the surface and the reason for it. `forced` is
//!   the raw `SBARHIVE_SURFACE`, which beats everything as today (`window`
//!   always wins, `layer` still needs the globals, other values are
//!   ignored). Then `hive_surface = "window"` or `"layer"`, then the reader
//!   rule. Exact reasons from AC2: `screen reader running`,
//!   `screen reader flag set, no reader running`, `config window`, and
//!   `missing <globals>` (starts with `missing ` and names every missing
//!   global). Other cases may give any non empty reason.
//! - `overlay::choice_line(surface: Surface, reason: &str) -> String`: the
//!   line printed on each change, without any environment variable. It
//!   starts with `hive surface choice: layer` or `hive surface choice:
//!   window` and contains the reason; never `hive surface: ` (that prefix
//!   stays the S-018 AC11 geometry line).
//! - `choose_surface` keeps its signature and behaviour (overlay_pure.rs).

use sbarhive_core::config::HiveSurface;
use sbarhive_ui::overlay::{Surface, choice_line, screen_reader_running, surface_choice};

const REQUIRED: [&str; 4] = [
    "zwlr_layer_shell_v1",
    "wl_subcompositor",
    "wp_viewporter",
    "wp_single_pixel_buffer_manager_v1",
];

const RUNNING: &str = "screen reader running";
const FLAG_ONLY: &str = "screen reader flag set, no reader running";
const CONFIG_WINDOW: &str = "config window";

fn fixture(name: &str) -> Vec<String> {
    let path = format!(
        "{}/../../tests/fixtures/overlay/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let names: Vec<String> = text
        .lines()
        .filter_map(|l| {
            l.trim_start_matches(|c: char| c == '*' || c.is_whitespace())
                .split_whitespace()
                .next()
                .map(str::to_owned)
        })
        .collect();
    assert!(names.len() > 20, "{path}: too few globals parsed");
    names
}

fn kwin() -> Vec<String> {
    fixture("kwin-6.7.5-globals.txt")
}

fn gnome() -> Vec<String> {
    fixture("gnome-50.5-globals.txt")
}

fn refs(v: &[String]) -> Vec<&str> {
    v.iter().map(String::as_str).collect()
}

const BOOLS: [bool; 2] = [false, true];
const CONFIGS: [HiveSurface; 3] = [HiveSurface::Auto, HiveSurface::Layer, HiveSurface::Window];
const FORCED: [Option<&str>; 5] = [None, Some("layer"), Some("window"), Some("junk"), Some("")];

// AC1: the reader rule ------------------------------------------------------

#[test]
fn ac1_reader_rule_every_combination() {
    for flag in BOOLS {
        for reader in BOOLS {
            for config in CONFIGS {
                let want = flag && reader && config == HiveSurface::Auto;
                assert_eq!(
                    screen_reader_running(flag, reader, config),
                    want,
                    "flag {flag}, reader {reader}, config {config:?}"
                );
            }
        }
    }
}

#[test]
fn ac1_flag_without_a_reader_is_not_a_screen_reader() {
    assert!(!screen_reader_running(true, false, HiveSurface::Auto));
}

#[test]
fn ac1_reader_without_the_flag_is_not_a_screen_reader() {
    assert!(!screen_reader_running(false, true, HiveSurface::Auto));
}

#[test]
fn ac1_flag_and_reader_with_auto_is_a_screen_reader() {
    assert!(screen_reader_running(true, true, HiveSurface::Auto));
}

#[test]
fn ac1_config_layer_and_window_beat_flag_and_reader() {
    assert!(!screen_reader_running(true, true, HiveSurface::Layer));
    assert!(!screen_reader_running(true, true, HiveSurface::Window));
}

// AC1 and AC2: the surface and its reason ------------------------------------

/// The expected surface, and the exact reason where AC2 names one.
fn oracle(
    has_globals: bool,
    flag: bool,
    reader: bool,
    config: HiveSurface,
    forced: Option<&str>,
) -> (Surface, Option<&'static str>) {
    match forced {
        Some("window") => return (Surface::Window, None),
        Some("layer") if has_globals => return (Surface::Layer, None),
        Some("layer") => return (Surface::Window, None),
        _ => {}
    }
    match config {
        HiveSurface::Window => (Surface::Window, Some(CONFIG_WINDOW)),
        HiveSurface::Layer if has_globals => (Surface::Layer, None),
        HiveSurface::Layer => (Surface::Window, None),
        HiveSurface::Auto if !has_globals => (Surface::Window, None),
        HiveSurface::Auto if flag && reader => (Surface::Window, Some(RUNNING)),
        HiveSurface::Auto if flag => (Surface::Layer, Some(FLAG_ONLY)),
        HiveSurface::Auto => (Surface::Layer, None),
    }
}

fn missing_of(globals: &[&str]) -> Vec<&'static str> {
    REQUIRED
        .into_iter()
        .filter(|r| !globals.contains(r))
        .collect()
}

#[test]
fn ac1_ac2_every_combination_on_kwin_and_gnome() {
    let k = kwin();
    let g = gnome();
    for (name, globals) in [("kwin", refs(&k)), ("gnome", refs(&g))] {
        let missing = missing_of(&globals);
        for flag in BOOLS {
            for reader in BOOLS {
                for config in CONFIGS {
                    for forced in FORCED {
                        let ctx = format!(
                            "{name}, flag {flag}, reader {reader}, config {config:?}, \
                             SBARHIVE_SURFACE {forced:?}"
                        );
                        let (want, why) = oracle(missing.is_empty(), flag, reader, config, forced);
                        let (got, reason) =
                            surface_choice(&globals, true, flag, reader, config, forced);
                        assert_eq!(got, want, "{ctx}");
                        assert!(!reason.trim().is_empty(), "{ctx}: empty reason");
                        if let Some(why) = why {
                            assert_eq!(reason, why, "{ctx}");
                        }
                        assert!(!reason.contains('\n'), "{ctx}: reason {reason:?}");
                    }
                }
            }
        }
    }
}

#[test]
fn ac2_flag_set_no_reader_is_layer_with_its_reason() {
    let (s, reason) = surface_choice(&refs(&kwin()), true, true, false, HiveSurface::Auto, None);
    assert_eq!((s, reason.as_str()), (Surface::Layer, FLAG_ONLY));
}

#[test]
fn ac2_reader_running_is_window_with_its_reason() {
    let (s, reason) = surface_choice(&refs(&kwin()), true, true, true, HiveSurface::Auto, None);
    assert_eq!((s, reason.as_str()), (Surface::Window, RUNNING));
}

#[test]
fn ac2_config_window_is_window_with_its_reason() {
    let (s, reason) = surface_choice(
        &refs(&kwin()),
        true,
        false,
        false,
        HiveSurface::Window,
        None,
    );
    assert_eq!((s, reason.as_str()), (Surface::Window, CONFIG_WINDOW));
}

#[test]
fn ac1_config_layer_beats_a_running_reader() {
    let (s, _) = surface_choice(&refs(&kwin()), true, true, true, HiveSurface::Layer, None);
    assert_eq!(s, Surface::Layer);
}

#[test]
fn ac1_forced_window_beats_config_layer() {
    let (s, _) = surface_choice(
        &refs(&kwin()),
        true,
        false,
        false,
        HiveSurface::Layer,
        Some("window"),
    );
    assert_eq!(s, Surface::Window);
}

#[test]
fn ac1_forced_layer_beats_config_window_and_a_running_reader() {
    let (s, _) = surface_choice(
        &refs(&kwin()),
        true,
        true,
        true,
        HiveSurface::Window,
        Some("layer"),
    );
    assert_eq!(s, Surface::Layer);
}

#[test]
fn ac2_missing_globals_names_them_on_gnome() {
    let g = gnome();
    let globals = refs(&g);
    let missing = missing_of(&globals);
    assert!(missing.contains(&"zwlr_layer_shell_v1"));
    for config in [HiveSurface::Auto, HiveSurface::Layer] {
        let (s, reason) = surface_choice(&globals, true, false, false, config, None);
        assert_eq!(s, Surface::Window, "config {config:?}");
        assert!(
            reason.starts_with("missing "),
            "config {config:?}: {reason:?}"
        );
        for m in &missing {
            assert!(
                reason.contains(m),
                "config {config:?}: {reason:?} lacks {m}"
            );
        }
        for present in REQUIRED.iter().filter(|r| !missing.contains(r)) {
            assert!(
                !reason.contains(present),
                "config {config:?}: {reason:?} names {present}, which is advertised"
            );
        }
    }
}

#[test]
fn ac2_missing_each_required_global_is_named_alone() {
    let k = kwin();
    for name in REQUIRED {
        let globals: Vec<&str> = refs(&k).into_iter().filter(|g| *g != name).collect();
        let (s, reason) = surface_choice(&globals, true, false, false, HiveSurface::Auto, None);
        assert_eq!(s, Surface::Window, "without {name}");
        assert!(reason.starts_with("missing "), "{reason:?}");
        assert!(reason.contains(name), "{reason:?} lacks {name}");
        for other in REQUIRED.iter().filter(|r| **r != name) {
            assert!(!reason.contains(other), "{reason:?} names {other}");
        }
    }
}

#[test]
fn ac2_forced_layer_without_globals_says_what_is_missing() {
    let (s, reason) = surface_choice(
        &refs(&gnome()),
        true,
        false,
        false,
        HiveSurface::Auto,
        Some("layer"),
    );
    assert_eq!(s, Surface::Window);
    assert!(reason.contains("zwlr_layer_shell_v1"), "{reason:?}");
}

#[test]
fn ac1_not_a_wayland_session_is_a_window() {
    for config in [HiveSurface::Auto, HiveSurface::Window] {
        let (s, _) = surface_choice(&refs(&kwin()), false, false, false, config, None);
        assert_eq!(s, Surface::Window, "config {config:?}");
    }
}

#[test]
fn ac1_only_the_four_required_globals_are_enough() {
    let (s, _) = surface_choice(&REQUIRED, true, true, false, HiveSurface::Auto, None);
    assert_eq!(s, Surface::Layer);
}

// AC2: the line -----------------------------------------------------------

#[test]
fn ac2_choice_line_prefix_kind_and_reason() {
    for (surface, kind) in [(Surface::Layer, "layer"), (Surface::Window, "window")] {
        for reason in [
            RUNNING,
            FLAG_ONLY,
            CONFIG_WINDOW,
            "missing zwlr_layer_shell_v1",
        ] {
            let line = choice_line(surface, reason);
            assert!(
                line.starts_with(&format!("hive surface choice: {kind}")),
                "{line:?}"
            );
            assert!(line.contains(reason), "{line:?} lacks {reason:?}");
            assert!(!line.starts_with("hive surface: "), "{line:?}");
            assert!(!line.contains('\n'), "{line:?}");
        }
    }
}

#[test]
fn ac2_choice_line_kind_follows_the_surface() {
    let l = choice_line(Surface::Layer, FLAG_ONLY);
    let w = choice_line(Surface::Window, FLAG_ONLY);
    assert!(!l.contains("window"), "{l:?}");
    assert!(!w.starts_with("hive surface choice: layer"), "{w:?}");
}
