//! S-090 (backlog/S-090-settings-updates-about.md): the Updates section of
//! the settings panel embedded in the welcome window. Same checks as
//! `tests/s090_updates.rs` (AC1 to AC3), driven through the welcome window's
//! own properties and callbacks. Lives beside `welcome.rs` because the
//! window's component is private.

use super::*;
use crate::updates::UpdateState;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn window(lang: Lang) -> WelcomeWindow {
    thread_local! {
        static INIT: std::sync::Once = const { std::sync::Once::new() };
    }
    INIT.with(|once| once.call_once(i_slint_backend_testing::init_no_event_loop));
    WelcomeWindow::new(lang).expect("window")
}

fn states() -> Vec<UpdateState> {
    vec![
        UpdateState::UpToDate,
        UpdateState::Checking,
        UpdateState::Downloading(42),
        UpdateState::Ready,
        UpdateState::Failed("offline".into()),
        UpdateState::Unavailable,
    ]
}

fn has_arabic(s: &str) -> bool {
    s.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c))
}

fn line(w: &WelcomeWindow) -> String {
    w.inner.ui.borrow().get_settings_update_state_line().into()
}

fn version_text(w: &WelcomeWindow) -> String {
    w.inner
        .ui
        .borrow()
        .get_settings_update_version_text()
        .into()
}

fn check_enabled(w: &WelcomeWindow) -> bool {
    w.inner.ui.borrow().get_settings_update_check_enabled()
}

fn apply_enabled(w: &WelcomeWindow) -> bool {
    w.inner.ui.borrow().get_settings_update_apply_enabled()
}

fn press_check(w: &WelcomeWindow) {
    w.inner.ui.borrow().invoke_update_check();
}

fn press_apply(w: &WelcomeWindow) {
    w.inner.ui.borrow().invoke_update_apply();
}

#[test]
fn ac1_welcome_settings_show_the_version_and_six_distinct_states_in_both_languages() {
    for lang in [Lang::En, Lang::Ar] {
        let w = window(lang);
        let mut lines = Vec::new();
        for st in states() {
            w.set_update_view(VERSION, st.clone(), true);
            assert!(version_text(&w).contains(VERSION), "{lang:?}: version");
            let l = line(&w);
            assert!(!l.trim().is_empty(), "{lang:?} {st:?}: empty state line");
            assert_eq!(has_arabic(&l), lang == Lang::Ar, "{lang:?} {st:?}: {l}");
            assert_eq!(has_arabic(&version_text(&w)), lang == Lang::Ar, "{lang:?}");
            lines.push(l);
        }
        let mut uniq = lines.clone();
        uniq.sort();
        uniq.dedup();
        assert_eq!(uniq.len(), 6, "{lang:?}: states must read differently");
        assert!(lines[2].contains("42"));
        assert!(lines[4].contains("offline"));
    }
}

#[test]
fn ac1_the_service_off_flag_disables_both_buttons() {
    let w = window(Lang::En);
    w.set_update_view(VERSION, UpdateState::Ready, true);
    assert!(check_enabled(&w) && apply_enabled(&w));
    w.set_update_view(VERSION, UpdateState::Ready, false);
    assert!(!check_enabled(&w) && !apply_enabled(&w));
}

#[test]
fn ac2_welcome_check_sends_one_request_and_disables_the_button() {
    for lang in [Lang::En, Lang::Ar] {
        let w = window(lang);
        let n = Rc::new(Cell::new(0u32));
        let n2 = n.clone();
        w.on_update_check(move || n2.set(n2.get() + 1));
        w.set_update_view(VERSION, UpdateState::UpToDate, true);
        let before = line(&w);
        assert!(check_enabled(&w));
        press_check(&w);
        assert_eq!(n.get(), 1, "{lang:?}: exactly one update-check request");
        assert!(!check_enabled(&w), "disabled while the check runs");
        let during = line(&w);
        assert_ne!(during, before, "the line reads checking at once");
        assert_eq!(has_arabic(&during), lang == Lang::Ar, "{lang:?}: {during}");
        press_check(&w);
        assert_eq!(n.get(), 1, "a second click while it runs sends nothing");
        w.set_update_view(VERSION, UpdateState::UpToDate, true);
        assert!(check_enabled(&w), "enabled again after the answer");
    }
}

#[test]
fn ac3_welcome_update_now_is_enabled_only_in_ready_and_sends_one_apply() {
    for lang in [Lang::En, Lang::Ar] {
        let w = window(lang);
        let n = Rc::new(Cell::new(0u32));
        let n2 = n.clone();
        w.on_update_apply(move || n2.set(n2.get() + 1));
        for st in states() {
            w.set_update_view(VERSION, st.clone(), true);
            assert_eq!(apply_enabled(&w), st == UpdateState::Ready, "{st:?}");
            let before = n.get();
            press_apply(&w);
            let expected = u32::from(st == UpdateState::Ready);
            assert_eq!(n.get() - before, expected, "{lang:?} {st:?}");
        }
        assert_eq!(n.get(), 1, "only the Ready state sends update-apply");
        assert!(!apply_enabled(&w), "disabled once the apply was sent");
        press_apply(&w);
        assert_eq!(n.get(), 1, "a second click sends nothing");
    }
}

#[test]
fn ac3_welcome_deferred_answer_shows_the_waiting_text_in_the_window_language() {
    for lang in [Lang::En, Lang::Ar] {
        let w = window(lang);
        w.set_update_view(VERSION, UpdateState::Ready, true);
        let ready = line(&w);
        w.show_update_deferred();
        let l = line(&w);
        assert!(!l.trim().is_empty());
        assert_ne!(l, ready);
        assert_eq!(has_arabic(&l), lang == Lang::Ar, "{lang:?}: {l}");
        assert!(!apply_enabled(&w), "Update now stays off while it waits");
        assert!(
            !l.to_lowercase().contains("fail") && !l.to_lowercase().contains("error"),
            "deferred is not an error: {l}"
        );
    }
}

#[test]
fn ac1_opening_the_settings_panel_keeps_the_update_section_in_the_window_language() {
    for lang in [Lang::En, Lang::Ar] {
        let w = window(lang);
        w.set_update_view(VERSION, UpdateState::UpToDate, true);
        let config = Config {
            language: if lang == Lang::Ar {
                kawaro_core::config::LangSetting::Ar
            } else {
                kawaro_core::config::LangSetting::En
            },
            ..Config::default()
        };
        w.show_settings(&config);
        assert_eq!(has_arabic(&line(&w)), lang == Lang::Ar, "{lang:?}");
        assert!(version_text(&w).contains(VERSION));
    }
}
