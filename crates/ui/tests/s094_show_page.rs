//! S-094: the `show_page` snapshot hooks of both windows move the shell to
//! the page they name and leave the remembered page alone.

use kawaro_core::config::Config;
use kawaro_core::welcome::Captions;
use kawaro_ui::welcome::WelcomeWindow;
use kawaro_ui::{Lang, SettingsWindow};
use std::sync::Mutex;

static LOCK: Mutex<()> = Mutex::new(());

fn init() -> std::sync::MutexGuard<'static, ()> {
    thread_local! {
        static INIT: std::sync::Once = const { std::sync::Once::new() };
    }
    INIT.with(|once| once.call_once(i_slint_backend_testing::init_no_event_loop));
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[test]
fn settings_show_page_changes_the_page_and_not_the_memory() {
    let _g = init();
    kawaro_ui::reset_page_memory();
    let w = SettingsWindow::new(Lang::En).unwrap();
    w.open(&Config::default()).unwrap();
    assert_eq!(w.page(), 0);
    for p in [3, 1, 4, 0] {
        w.show_page(p);
        assert_eq!(w.page(), p);
    }
    w.show_page(2);
    let again = SettingsWindow::new(Lang::En).unwrap();
    again.open(&Config::default()).unwrap();
    assert_eq!(again.page(), 0, "show_page does not touch the memory");
}

#[test]
fn welcome_show_page_changes_the_page() {
    let _g = init();
    let w = WelcomeWindow::new(Lang::En).unwrap();
    let captions = Captions {
        steps: std::array::from_fn(|i| format!("step {i}")),
    };
    w.open(&captions, true).unwrap();
    assert_eq!(w.page(), 0);
    for p in [5, 2, 1, 0] {
        w.show_page(p);
        assert_eq!(w.page(), p);
    }
}

fn welcome(lang: Lang) -> WelcomeWindow {
    let w = WelcomeWindow::new(lang).unwrap();
    let captions = Captions {
        steps: std::array::from_fn(|i| format!("step {i}")),
    };
    w.open(&captions, true).unwrap();
    w
}

#[test]
fn welcome_asks_for_the_saved_settings_once_when_a_settings_page_is_picked() {
    use std::cell::Cell;
    use std::rc::Rc;
    let _g = init();
    let w = welcome(Lang::En);
    let asked = Rc::new(Cell::new(0));
    let a = asked.clone();
    w.on_settings(move || a.set(a.get() + 1));
    w.select_page(0);
    assert_eq!(asked.get(), 0, "the Introduction needs no settings");
    w.select_page(1);
    assert_eq!(asked.get(), 1, "a settings page asks for them");
    w.show_settings(&Config::default());
    w.select_page(2);
    w.select_page(0);
    assert_eq!(asked.get(), 1, "once loaded it never asks again");
}

#[test]
fn welcome_loaded_settings_are_not_unsaved_and_an_edit_is() {
    let _g = init();
    let w = welcome(Lang::En);
    assert!(!w.unsaved_marker_visible(), "defaults at open are clean");
    let saved = Config {
        quality: kawaro_core::config::Quality::Max,
        notify: false,
        ..Config::default()
    };
    w.show_settings(&saved);
    assert!(!w.unsaved_marker_visible(), "loaded values are clean");
    w.set_theme(kawaro_core::config::Theme::Dark);
    assert!(w.unsaved_marker_visible());
}

#[test]
fn welcome_save_hands_over_the_edit_in_both_directions() {
    use kawaro_core::config::{LangSetting, Theme};
    use std::cell::RefCell;
    use std::rc::Rc;
    let _g = init();
    for (lang, setting) in [(Lang::En, LangSetting::En), (Lang::Ar, LangSetting::Ar)] {
        let w = welcome(lang);
        let saved: Rc<RefCell<Vec<Config>>> = Rc::default();
        let s = saved.clone();
        w.on_settings_save(move |c| {
            s.borrow_mut().push(c);
            true
        });
        w.show_settings(&Config {
            language: setting,
            ..Config::default()
        });
        w.set_theme(Theme::Dark);
        w.press_save();
        let got = saved.borrow();
        assert_eq!(got.len(), 1, "{lang:?}: Save reaches the callback");
        assert_eq!(got[0].theme, Theme::Dark, "{lang:?}");
        assert_eq!(w.page(), 0, "{lang:?}: back on the Introduction");
    }
}
