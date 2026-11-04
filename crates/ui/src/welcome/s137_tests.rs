//! S-137 actual-window tests. The one missing seam is WelcomeUi.film-visibility.
//! Contract: a Slint output struct propagated FilmScene -> FilmCard -> WelcomeUi,
//! bound to the SAME predicates/items that render the scene, not a Rust pose copy.
//! Fields: hint, hint-interactive, thumbnail, hot, pressed, result, source (bool),
//! action-count (int), pressed-action (string, WEBP only while pressed).
//! It reports settled visible state, after animations, for the current page.
use super::*;
use kawaro_core::config::DropMode;
use kawaro_core::welcome::{self, Registration};
use slint::Model;
use std::sync::{Mutex, MutexGuard};

static LOCK: Mutex<()> = Mutex::new(());

fn init() -> MutexGuard<'static, ()> {
    thread_local! { static INIT: std::sync::Once = const { std::sync::Once::new() }; }
    INIT.with(|once| once.call_once(i_slint_backend_testing::init_no_event_loop));
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn settle() {
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(500));
}

#[test]
fn ac2_actual_slint_drag_hint_then_unchosen_actions_then_webp_click_then_result() {
    let _guard = init();
    for lang in [Lang::En, Lang::Ar] {
        for centred in [false, true] {
            let w = WelcomeWindow::new(lang).unwrap();
            w.open(
                &welcome::captions(lang, &Registration::Native("Alt".into())),
                centred,
            )
            .unwrap();
            w.set_reduced_motion(false);
            let (mut hint, mut awaiting, mut clicked, mut result) = (None, None, None, None);
            for i in 0..film::POSES.len() {
                w.freeze(i);
                settle();
                let ui = w.inner.ui.borrow();
                let p = ui.get_pose();
                let model = film::POSES[i];
                assert_eq!(
                    (p.step, p.hive, p.thumbnail, p.hot, p.pressed, p.result),
                    (
                        model.step as i32,
                        model.hive,
                        model.thumbnail,
                        model.hot,
                        model.pressed,
                        model.result
                    ),
                    "actual generated FilmPose mapping at {i}"
                );
                assert_eq!(
                    (p.selected, p.pointer, p.approach, p.lift, p.drag),
                    (
                        model.selected,
                        model.pointer,
                        model.approach,
                        model.lift,
                        model.drag
                    )
                );
                let v = ui.get_film_visibility();
                assert!(v.source, "original remains visible in frame {i}");
                assert!(
                    !(v.hint && v.action_count > 0),
                    "hint and actions overlap at {i}"
                );
                assert!(!v.hint_interactive, "hint cannot accept an action click");
                assert_eq!(
                    (v.thumbnail, v.hot, v.pressed, v.result),
                    (p.thumbnail, p.hot, p.pressed, p.result)
                );
                if v.thumbnail {
                    assert!(awaiting.is_none(), "drag cannot return after dropping");
                    assert!(v.hint, "dragged file must have a drop hint at {i}");
                    assert_eq!(v.action_count, 0, "no drag-time action cells at {i}");
                    assert!(!v.hot && !v.pressed && !v.result);
                    hint = Some(i);
                }
                if v.action_count > 0 && !v.hot && !v.pressed {
                    assert!(!v.thumbnail && !v.hint && !v.result);
                    assert!(hint.is_some(), "choices appear only after a drag");
                    awaiting.get_or_insert(i);
                }
                if v.pressed {
                    assert!(
                        v.action_count > 0 && p.pointer,
                        "click needs visible choices and a pointer"
                    );
                    assert!(awaiting.is_some(), "distinct unchosen frame before click");
                    assert!(!v.thumbnail && !v.result && !v.hint);
                    assert_eq!(v.pressed_action.as_str(), "WEBP");
                    clicked.get_or_insert(i);
                }
                if v.result {
                    assert!(
                        clicked.is_some_and(|click| click < i),
                        "click must precede result"
                    );
                    assert!(!v.thumbnail && !v.hint);
                    result.get_or_insert(i);
                }
            }
            assert!(
                hint.is_some() && awaiting.is_some() && clicked.is_some() && result.is_some(),
                "all four visible stages required: {hint:?} {awaiting:?} {clicked:?} {result:?}"
            );
            w.close();
        }
    }
}

#[test]
fn ac3_actual_window_timer_captions_pause_replay_and_reduced_poster() {
    let _guard = init();
    for lang in [Lang::En, Lang::Ar] {
        let w = WelcomeWindow::new(lang).unwrap();
        w.open(&welcome::captions(lang, &Registration::Unbound), false)
            .unwrap();
        w.set_reduced_motion(false);
        w.press_film_play_pause();
        assert!(w.film_timer_running());
        let mut steps = Vec::new();
        for i in 0..film::POSES.len() {
            let ui = w.inner.ui.borrow();
            let p = ui.get_pose();
            assert_eq!(ui.get_film_frame(), i as i32);
            assert_eq!(p.step, film::POSES[i].step as i32);
            assert_eq!(
                ui.get_lit_a(),
                p.step,
                "each phase lights its own instruction"
            );
            assert_eq!(
                ui.get_lit_b(),
                -1,
                "drop and choice cannot share a highlight"
            );
            steps.push(p.step);
            drop(ui);
            w.film_tick();
        }
        steps.dedup();
        assert_eq!(steps, [0, 1, 2, 3, 4], "five ordered instructional phases");
        w.press_film_play_pause();
        let held = w.inner.ui.borrow().get_pose();
        w.film_tick();
        assert_eq!(w.inner.ui.borrow().get_pose(), held);
        assert!(!w.film_timer_running());
        w.freeze(film::poster_index());
        w.press_film_play_pause();
        assert_eq!(
            w.inner.ui.borrow().get_pose().step,
            0,
            "replay begins with drag"
        );
        w.set_reduced_motion(true);
        settle();
        assert!(!w.film_timer_running() && !w.film_playing());
        let poster = w.inner.ui.borrow().get_pose();
        assert!(poster.result && !poster.thumbnail && !poster.pressed);
        assert_eq!(poster.step, 4);
        w.film_tick();
        assert_eq!(w.inner.ui.borrow().get_pose(), poster);
        w.close();
    }
}

#[test]
fn ac1_ac3_language_selection_and_settings_reopen_keep_explicit_default_tutorial() {
    let _guard = init();
    for lang in [Lang::En, Lang::Ar] {
        let w = WelcomeWindow::new(lang).unwrap();
        let captions = welcome::captions(lang, &Registration::Fallback("Super+Z".into()));
        w.choose_language();
        w.on_language(|_| true);
        w.open(&captions, true).unwrap();
        assert!(w.press_language_key(LanguageKey::Enter));
        assert!(!w.choose_language_visible());
        let config = Config {
            drop_mode: DropMode::ChooseDuringDrag,
            ..Config::default()
        };
        for reopened in [false, true] {
            if reopened {
                w.show_settings(&config);
                w.show_page(0);
            }
            let ui = w.inner.ui.borrow();
            assert_eq!(ui.get_page(), 0);
            assert_eq!(ui.get_rtl(), lang == Lang::Ar);
            assert!(ui.get_centred());
            let shown: Vec<String> = ui
                .get_caption_steps()
                .iter()
                .map(|s| s.to_string())
                .collect();
            assert_eq!(shown, captions.steps);
            assert!(
                shown.join(" ").contains(if lang == Lang::En {
                    "default"
                } else {
                    "افتراضي"
                }),
                "tutorial must explicitly name the default even with ChooseDuringDrag saved"
            );
            drop(ui);
            w.freeze(film::poster_index());
            assert_eq!(w.inner.ui.borrow().get_pose().step, 4);
        }
        w.close();
    }
}
