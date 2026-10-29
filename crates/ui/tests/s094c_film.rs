//! S-094c (backlog/S-094c-introduction-film.md): the Introduction film.
//! Written from the acceptance criteria only.
//!
//! Hooks these tests pin (the implementer builds them; all `#[doc(hidden)]
//! pub`, on `WelcomeWindow` unless a path is given). Geometry is logical px in
//! window coordinates, as in `s094_shell.rs`.
//! - `kawaro_ui::film::frames() -> Vec<FilmFrame>`: the pure scene model, one
//!   entry per frame of the loop. `FilmFrame { step: usize (0..4, step 0 is
//!   "select a file", 1 "hive appears", 2 "drop on a cell", 3 "result"),
//!   hive_opacity: f32, elements: Vec<String> }`. Element names:
//!   `file-manager`, `source-file`, `pointer`, `hive`, `thumbnail`,
//!   `result-file`.
//! - `kawaro_ui::film::poster_index() -> usize`: the poster frame index.
//! - `film_frame() -> usize` (index into `frames()` now shown), `film_step()`,
//!   `film_is_poster() -> bool`, `film_playing() -> bool`,
//!   `film_timer_running() -> bool` (a Slint timer is active),
//!   `film_tick()` (advance one frame exactly as the timer would, only when
//!   playing), `press_film_play_pause()`, `film_button_label() -> String`
//!   (the accessible label of the Play/Pause button, also listed in
//!   `visible_labels()`), `set_reduced_motion(bool)`.
//! - `film_card_rect() -> Rect` (the card with film, button and captions),
//!   `film_rect()` is the picture inside it (S-094).
//! - `set_scale_factor(f32)` and `film_render_pixels() -> (u32, u32)`: the
//!   pixel size the film is rendered at for the current scale factor (the
//!   source image size for raster frames; displayed size x scale for a live
//!   vector scene).

use kawaro_core::config::Theme;
use kawaro_core::welcome::Captions;
use kawaro_ui::film;
use kawaro_ui::welcome::WelcomeWindow;
use kawaro_ui::{Lang, Rect};
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

static LOCK: Mutex<()> = Mutex::new(());

fn init() -> MutexGuard<'static, ()> {
    thread_local! {
        static INIT: std::sync::Once = const { std::sync::Once::new() };
    }
    INIT.with(|once| once.call_once(i_slint_backend_testing::init_no_event_loop));
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn captions() -> Captions {
    Captions {
        steps: std::array::from_fn(|i| format!("caption number {i}")),
    }
}

fn welcome(lang: Lang) -> WelcomeWindow {
    let w = WelcomeWindow::new(lang).expect("window");
    w.open(&captions(), true).expect("open");
    w
}

fn right(r: Rect) -> f32 {
    r.x + r.w
}
fn bottom(r: Rect) -> f32 {
    r.y + r.h
}

fn has(f: &film::FilmFrame, e: &str) -> bool {
    f.elements.iter().any(|x| x == e)
}

#[test]
fn ac1_frames_render_at_displayed_size_times_scale_at_1_2_3() {
    let _g = init();
    for lang in [Lang::En, Lang::Ar] {
        for (ww, hh) in [(880.0, 640.0), (1920.0, 1080.0), (3840.0, 2160.0)] {
            for scale in [1.0_f32, 2.0, 3.0] {
                let w = welcome(lang);
                w.set_window_size(ww, hh);
                w.set_scale_factor(scale);
                let film = w.film_rect().expect("Introduction shows the film");
                let (pw, ph) = w.film_render_pixels();
                let ctx = format!("{lang:?} {ww}x{hh} scale {scale}: {film:?} -> {pw}x{ph}");
                assert!(pw as f32 >= (film.w * scale).round(), "{ctx}: blurry width");
                assert!(
                    ph as f32 >= (film.h * scale).round(),
                    "{ctx}: blurry height"
                );
            }
        }
    }
}

#[test]
fn ac1_any_shipped_raster_frame_is_at_least_scale_3_of_the_max_width() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/welcome");
    let mut seen = 0;
    for e in std::fs::read_dir(&dir)
        .expect("resources/welcome")
        .flatten()
    {
        let name = e.file_name().to_string_lossy().to_string();
        if !(name.starts_with("frame-") && name.ends_with(".png")) {
            continue;
        }
        let b = std::fs::read(e.path()).unwrap();
        let w = u32::from_be_bytes(b[16..20].try_into().unwrap());
        let h = u32::from_be_bytes(b[20..24].try_into().unwrap());
        assert!(
            w >= 1680 && h >= 945,
            "{name} is {w}x{h}, needs 1680x945 (film-max-width 560 x 3)"
        );
        seen += 1;
    }
    let _ = seen; // zero frames is fine: the scene may be drawn live as vectors
}

#[test]
fn ac2_scene_model_shows_the_four_steps_in_order_and_never_a_half_scene() {
    let frames = film::frames();
    assert!(frames.len() >= 4, "at least one frame per step");
    let steps: Vec<usize> = frames.iter().map(|f| f.step).collect();
    assert!(
        steps.windows(2).all(|p| p[0] <= p[1]),
        "steps in order: {steps:?}"
    );
    for s in 0..4 {
        assert!(steps.contains(&s), "step {s} is missing");
    }
    assert!(steps.iter().all(|s| *s < 4));
    for (i, f) in frames.iter().enumerate() {
        let ctx = format!("frame {i} (step {})", f.step);
        let (need, forbid): (&[&str], &[&str]) = match f.step {
            0 => (
                &["file-manager", "source-file"],
                &["hive", "thumbnail", "result-file"],
            ),
            1 => (
                &["file-manager", "source-file", "hive", "pointer"],
                &["thumbnail", "result-file"],
            ),
            2 => (
                &["file-manager", "source-file", "hive", "thumbnail"],
                &["result-file"],
            ),
            _ => (
                &["file-manager", "source-file", "result-file"],
                &["thumbnail"],
            ),
        };
        for n in need {
            assert!(has(f, n), "{ctx}: must show {n}, has {:?}", f.elements);
        }
        for n in forbid {
            assert!(!has(f, n), "{ctx}: must not show {n}");
        }
        if has(f, "thumbnail") {
            assert!(
                has(f, "source-file"),
                "{ctx}: dragged thumbnail without its source"
            );
        }
        if f.step == 1 || f.step == 2 {
            assert_eq!(f.hive_opacity, 1.0, "{ctx}: hive opacity");
        }
    }
}

#[test]
fn ac2_the_window_shows_the_same_step_as_the_model_with_its_caption() {
    let _g = init();
    for lang in [Lang::En, Lang::Ar] {
        let w = welcome(lang);
        w.press_film_play_pause();
        for _ in 0..film::frames().len() * 2 {
            let f = &film::frames()[w.film_frame()];
            assert_eq!(w.film_step(), f.step);
            let cap = format!("caption number {}", f.step);
            assert!(
                w.visible_texts().iter().any(|t| t.contains(&cap)),
                "{lang:?}: caption of step {} on screen",
                f.step
            );
            w.film_tick();
        }
    }
}

#[test]
fn ac3_starts_on_the_poster_and_play_pause_toggles_the_loop() {
    let _g = init();
    for lang in [Lang::En, Lang::Ar] {
        let w = welcome(lang);
        assert!(w.film_is_poster(), "{lang:?}: poster before play");
        assert_eq!(w.film_frame(), film::poster_index());
        assert_eq!(
            film::frames()[film::poster_index()].step,
            3,
            "poster shows the result"
        );
        assert!(has(&film::frames()[film::poster_index()], "result-file"));
        assert!(!w.film_playing());
        w.film_tick();
        assert_eq!(
            w.film_frame(),
            film::poster_index(),
            "no tick moves a paused film"
        );

        let play = w.film_button_label();
        assert!(!play.is_empty());
        assert!(
            w.visible_labels().contains(&play),
            "{lang:?}: button on screen"
        );
        w.press_film_play_pause();
        assert!(w.film_playing() && w.film_timer_running(), "{lang:?}");
        assert!(!w.film_is_poster());
        let pause = w.film_button_label();
        assert!(
            !pause.is_empty() && pause != play,
            "{lang:?}: label toggles"
        );
        let before = w.film_frame();
        w.film_tick();
        assert_ne!(w.film_frame(), before, "{lang:?}: advances after 1 tick");

        w.press_film_play_pause();
        assert!(!w.film_playing() && !w.film_timer_running(), "{lang:?}");
        let held = w.film_frame();
        w.film_tick();
        w.film_tick();
        assert_eq!(w.film_frame(), held, "{lang:?}: pause keeps the frame");
        assert_eq!(w.film_button_label(), play);
    }
}

#[test]
fn ac3_never_auto_plays_and_reduced_motion_is_a_static_poster_with_a_caption_list() {
    let _g = init();
    for lang in [Lang::En, Lang::Ar] {
        for reduced in [false, true] {
            let w = welcome(lang);
            w.set_reduced_motion(reduced);
            i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(3000));
            let ctx = format!("{lang:?} reduced {reduced}");
            assert!(w.film_is_poster(), "{ctx}: still the poster after 3000 ms");
            assert_eq!(w.film_frame(), film::poster_index(), "{ctx}");
            assert!(!w.film_playing(), "{ctx}");
            assert!(!w.film_timer_running(), "{ctx}: no timer without a press");
            if reduced {
                let texts = w.visible_texts();
                for i in 0..4 {
                    let cap = format!("caption number {i}");
                    assert!(texts.iter().any(|t| t.contains(&cap)), "{ctx}: caption {i}");
                }
            }
        }
    }
}

#[test]
fn ac4_card_is_inside_the_window_and_not_taller_than_the_content_area() {
    let _g = init();
    for lang in [Lang::En, Lang::Ar] {
        for theme in [Theme::Light, Theme::Dark] {
            for reduced in [false, true] {
                for (ww, hh) in [(880.0, 640.0), (1920.0, 1080.0), (3840.0, 2160.0)] {
                    let w = welcome(lang);
                    w.set_reduced_motion(reduced);
                    w.set_theme(theme);
                    w.set_window_size(ww, hh);
                    let card = w.film_card_rect();
                    let film = w.film_rect().expect("film");
                    let area = w.page_area_rect();
                    let ctx = format!("{lang:?} {theme:?} reduced {reduced} {ww}x{hh}: {card:?}");
                    assert!(card.x >= 0.0 && card.y >= 0.0, "{ctx}");
                    assert!(right(card) <= ww && bottom(card) <= hh, "{ctx}: overflow");
                    assert!(card.h <= area.h + 0.5, "{ctx}: taller than {}", area.h);
                    assert!(
                        film.x >= card.x - 0.5 && right(film) <= right(card) + 0.5,
                        "{ctx}: film outside card"
                    );
                    assert!(
                        film.y >= card.y - 0.5 && bottom(film) <= bottom(card) + 0.5,
                        "{ctx}"
                    );
                    assert!(film.w <= 560.0 + 0.5, "{ctx}: film-max-width");
                }
            }
        }
    }
}
