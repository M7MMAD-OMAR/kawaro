//! S-018a (backlog/S-018-hive-overlay.md, AC1 to AC3): the pure parts of the
//! layer-shell overlay. No compositor, no environment: every input is a
//! parameter, so these run in parallel and under `cargo mutants`.
//!
//! Fixtures: `tests/fixtures/overlay/*-globals.txt` are the registry dumps of
//! the hidden KWin 6.7.5 and GNOME Shell 50.5 runs of `spikes/overlay/`
//! (`overlay-spike globals`), one global per line as `[*] name vN`.

use sbarhive_ui::overlay::{
    Placement, Surface, choose_surface, key_for_evdev, place, scheme_is_dark, wants_cursor_shape,
};
use slint::platform::Key;

const REQUIRED: [&str; 4] = [
    "zwlr_layer_shell_v1",
    "wl_subcompositor",
    "wp_viewporter",
    "wp_single_pixel_buffer_manager_v1",
];

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

fn without(v: &[String], name: &str) -> Vec<String> {
    let out: Vec<String> = v.iter().filter(|g| *g != name).cloned().collect();
    assert_eq!(out.len() + 1, v.len(), "{name} must be in the fixture once");
    out
}

// AC1 ---------------------------------------------------------------------

#[test]
fn ac1_fixtures_hold_what_the_story_says() {
    let k = kwin();
    let g = gnome();
    for name in REQUIRED {
        assert!(k.iter().any(|x| x == name), "KWin lacks {name}");
    }
    assert!(!g.iter().any(|x| x == "zwlr_layer_shell_v1"));
}

#[test]
fn ac1_kwin_globals_choose_layer() {
    let (s, _) = choose_surface(&refs(&kwin()), false, true, None);
    assert_eq!(s, Surface::Layer);
}

#[test]
fn ac1_gnome_globals_choose_window() {
    let (s, _) = choose_surface(&refs(&gnome()), false, true, None);
    assert_eq!(s, Surface::Window);
}

#[test]
fn ac1_each_missing_required_global_chooses_window() {
    let k = kwin();
    for name in REQUIRED {
        let g = without(&k, name);
        let (s, _) = choose_surface(&refs(&g), false, true, None);
        assert_eq!(s, Surface::Window, "without {name}");
    }
}

#[test]
fn ac1_only_the_four_required_globals_choose_layer() {
    let (s, _) = choose_surface(&REQUIRED, false, true, None);
    assert_eq!(s, Surface::Layer);
}

#[test]
fn ac1_screen_reader_on_chooses_window() {
    let (s, _) = choose_surface(&refs(&kwin()), true, true, None);
    assert_eq!(s, Surface::Window);
}

#[test]
fn ac1_not_a_wayland_session_chooses_window() {
    let (s, _) = choose_surface(&refs(&kwin()), false, false, None);
    assert_eq!(s, Surface::Window);
}

#[test]
fn ac1_forced_window_on_kwin_chooses_window() {
    let (s, _) = choose_surface(&refs(&kwin()), false, true, Some("window"));
    assert_eq!(s, Surface::Window);
}

#[test]
fn ac1_forced_layer_overrides_the_screen_reader() {
    let (s, _) = choose_surface(&refs(&kwin()), true, true, Some("layer"));
    assert_eq!(s, Surface::Layer);
}

#[test]
fn ac1_forced_layer_without_globals_chooses_window_with_a_log_line() {
    let (s, log) = choose_surface(&refs(&gnome()), false, true, Some("layer"));
    assert_eq!(s, Surface::Window);
    let line = log.expect("forced layer without the globals must log a line");
    assert!(!line.trim().is_empty());
}

#[test]
fn ac1_forced_layer_missing_one_global_chooses_window_with_a_log_line() {
    let g = without(&kwin(), "wp_viewporter");
    let (s, log) = choose_surface(&refs(&g), false, true, Some("layer"));
    assert_eq!(s, Surface::Window);
    assert!(log.is_some_and(|l| !l.trim().is_empty()));
}

// AC2 ---------------------------------------------------------------------

/// The hive today: `Layout::extent` with two rings, 393 x 362 logical px.
const CELL: (u32, u32) = (393, 362);

fn check(screen: (u32, u32), scale120: u32, pos: (i32, i32), buffer: (u32, u32)) {
    let p: Placement = place(screen, CELL, scale120);
    assert_eq!((p.x, p.y), pos, "position on {screen:?} at {scale120}/120");
    assert_eq!(p.buffer, buffer, "buffer on {screen:?} at {scale120}/120");
    assert_eq!(
        p.destination, CELL,
        "destination on {screen:?} at {scale120}/120"
    );
}

#[test]
fn ac2_1280x800_scale_1() {
    check((1280, 800), 120, (444, 219), (393, 362));
}

#[test]
fn ac2_1280x800_scale_1_5() {
    check((1280, 800), 180, (444, 219), (590, 543));
}

#[test]
fn ac2_1920x1080_scale_2() {
    check((1920, 1080), 240, (764, 359), (786, 724));
}

#[test]
fn ac2_3840x2160_scale_1_25() {
    check((3840, 2160), 150, (1724, 899), (492, 453));
}

// AC3 ---------------------------------------------------------------------

const KEYS: [(u32, Key); 9] = [
    (1, Key::Escape),
    (28, Key::Return),
    (96, Key::Return),
    (57, Key::Space),
    (15, Key::Tab),
    (103, Key::UpArrow),
    (108, Key::DownArrow),
    (105, Key::LeftArrow),
    (106, Key::RightArrow),
];

#[test]
fn ac3_mapped_evdev_codes() {
    for (code, key) in KEYS {
        assert_eq!(key_for_evdev(code), Some(key), "evdev {code}");
    }
}

#[test]
fn ac3_every_other_evdev_code_is_nothing() {
    for code in (0..=767).chain([1000, 0x1_0000, u32::MAX]) {
        if KEYS.iter().any(|(c, _)| *c == code) {
            continue;
        }
        assert_eq!(key_for_evdev(code), None, "evdev {code}");
    }
}

#[test]
fn ac3_portal_1_is_dark() {
    assert!(scheme_is_dark(None, Some(1)));
}

#[test]
fn ac3_portal_2_is_light() {
    assert!(!scheme_is_dark(None, Some(2)));
}

#[test]
fn ac3_portal_0_is_light() {
    assert!(!scheme_is_dark(None, Some(0)));
}

#[test]
fn ac3_portal_absent_or_error_is_light() {
    assert!(!scheme_is_dark(None, None));
}

#[test]
fn ac3_env_dark_beats_portal_light() {
    assert!(scheme_is_dark(Some("dark"), Some(2)));
    assert!(scheme_is_dark(Some("dark"), None));
}

#[test]
fn ac3_env_light_beats_portal_dark() {
    assert!(!scheme_is_dark(Some("light"), Some(1)));
}

#[test]
fn ac3_cursor_shape_asked_when_advertised() {
    assert!(wants_cursor_shape(&refs(&kwin())));
    assert!(wants_cursor_shape(&refs(&gnome())));
    assert!(wants_cursor_shape(&["wp_cursor_shape_manager_v1"]));
}

#[test]
fn ac3_cursor_shape_not_asked_when_absent() {
    let g = without(&kwin(), "wp_cursor_shape_manager_v1");
    assert!(!wants_cursor_shape(&refs(&g)));
    assert!(!wants_cursor_shape(&[]));
}
