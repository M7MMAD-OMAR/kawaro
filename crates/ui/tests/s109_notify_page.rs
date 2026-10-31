//! S-109 AC3 (backlog/S-109-first-run-remainder.md): the settings page has
//! one list of three notification options instead of two switches, in the
//! settings window and in the welcome window. Written from the acceptance
//! criteria only.
//!
//! Hooks these tests pin, `#[doc(hidden)] pub` with the same name and
//! signature on `SettingsWindow` and `WelcomeWindow` (next to the S-094 shell
//! hooks, behind `testing-hooks`):
//! - `notify_options() -> Vec<(String, String, bool)>`: for each of the three
//!   options in LOGICAL order (Off, All, Long; not mirrored in Arabic) the
//!   visible text, the accessible label and whether it is selected.
//! - `pick_notify_option(i: usize)`: what a click on logical option `i` does.
//! - `notify_list_label() -> String`: the accessible label of the list.
//! - `Config.notify_mode` (see `crates/core/tests/s109_notify_mode.rs`).
//! - The two notify switches and their strings are gone from both windows.

use kawaro_core::config::{Config, NotifyMode};
use kawaro_core::welcome::Captions;
use kawaro_ui::welcome::WelcomeWindow;
use kawaro_ui::{Lang, SettingsWindow};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Mutex;

static LOCK: Mutex<()> = Mutex::new(());

fn init() -> std::sync::MutexGuard<'static, ()> {
    thread_local! {
        static INIT: std::sync::Once = const { std::sync::Once::new() };
    }
    INIT.with(|once| once.call_once(i_slint_backend_testing::init_no_event_loop));
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

const MODES: [NotifyMode; 3] = [NotifyMode::Off, NotifyMode::All, NotifyMode::Long];

const EN: [&str; 3] = [
    "Off",
    "Every finished job",
    "Only jobs that took 10 seconds or more",
];
const AR: [&str; 3] = ["إيقاف", "كل عملية تنتهي", "فقط ما استغرق 10 ثوانٍ أو أكثر"];

fn cfg(mode: NotifyMode) -> Config {
    Config {
        notify_mode: mode,
        ..Config::default()
    }
}

/// What both windows expose for this story, so every check runs on both.
trait Page {
    fn open_with(lang: Lang, c: &Config, saved: Rc<RefCell<Vec<Config>>>) -> Self;
    fn options(&self) -> Vec<(String, String, bool)>;
    fn pick(&self, i: usize);
    fn list_label(&self) -> String;
    fn unsaved(&self) -> bool;
    fn cancel(&self);
    fn save(&self);
    fn go_to_notifications(&self);
    fn rows(&self) -> Vec<String>;
    fn texts(&self) -> Vec<String>;
}

macro_rules! page {
    ($t:ty, $open:expr) => {
        impl Page for $t {
            fn open_with(lang: Lang, c: &Config, saved: Rc<RefCell<Vec<Config>>>) -> Self {
                $open(lang, c, saved)
            }
            fn options(&self) -> Vec<(String, String, bool)> {
                <$t>::notify_options(self)
            }
            fn pick(&self, i: usize) {
                <$t>::pick_notify_option(self, i)
            }
            fn list_label(&self) -> String {
                <$t>::notify_list_label(self)
            }
            fn unsaved(&self) -> bool {
                <$t>::unsaved_marker_visible(self)
            }
            fn cancel(&self) {
                <$t>::press_cancel(self)
            }
            fn save(&self) {
                <$t>::press_save(self)
            }
            fn go_to_notifications(&self) {
                // The page whose rows include the notification list.
                for p in 0..6 {
                    <$t>::select_page(self, p);
                    if <$t>::page_rows(self)
                        .iter()
                        .any(|r| r == "Notifications" || r == "الإشعارات")
                    {
                        return;
                    }
                }
                panic!("no page has a Notifications row");
            }
            fn rows(&self) -> Vec<String> {
                <$t>::page_rows(self)
            }
            fn texts(&self) -> Vec<String> {
                <$t>::visible_texts(self)
            }
        }
    };
}

page!(
    SettingsWindow,
    |lang, c: &Config, saved: Rc<RefCell<Vec<Config>>>| {
        let w = SettingsWindow::new(lang).unwrap();
        w.on_save(move |c| saved.borrow_mut().push(c));
        w.open(c).unwrap();
        w
    }
);

page!(
    WelcomeWindow,
    |lang, c: &Config, saved: Rc<RefCell<Vec<Config>>>| {
        let w = WelcomeWindow::new(lang).unwrap();
        let captions = Captions {
            steps: std::array::from_fn(|i| format!("step {i}")),
        };
        w.on_settings_save(move |c| {
            saved.borrow_mut().push(c);
            true
        });
        w.open(&captions, true).unwrap();
        w.show_settings(c);
        w
    }
);

fn selected(options: &[(String, String, bool)]) -> Vec<usize> {
    options
        .iter()
        .enumerate()
        .filter(|(_, o)| o.2)
        .map(|(i, _)| i)
        .collect()
}

fn each_window(
    check: impl Fn(&str, &dyn Fn(Lang, &Config) -> (Box<dyn PageDyn>, Rc<RefCell<Vec<Config>>>)),
) {
    check("settings", &|lang, c| {
        let saved: Rc<RefCell<Vec<Config>>> = Rc::default();
        let w = <SettingsWindow as Page>::open_with(lang, c, saved.clone());
        (Box::new(w), saved)
    });
    check("welcome", &|lang, c| {
        let saved: Rc<RefCell<Vec<Config>>> = Rc::default();
        let w = <WelcomeWindow as Page>::open_with(lang, c, saved.clone());
        (Box::new(w), saved)
    });
}

trait PageDyn {
    fn options(&self) -> Vec<(String, String, bool)>;
    fn pick(&self, i: usize);
    fn list_label(&self) -> String;
    fn unsaved(&self) -> bool;
    fn cancel(&self);
    fn save(&self);
    fn go(&self);
    fn rows(&self) -> Vec<String>;
    fn texts(&self) -> Vec<String>;
}

impl<T: Page> PageDyn for T {
    fn options(&self) -> Vec<(String, String, bool)> {
        Page::options(self)
    }
    fn pick(&self, i: usize) {
        Page::pick(self, i)
    }
    fn list_label(&self) -> String {
        Page::list_label(self)
    }
    fn unsaved(&self) -> bool {
        Page::unsaved(self)
    }
    fn cancel(&self) {
        Page::cancel(self)
    }
    fn save(&self) {
        Page::save(self)
    }
    fn go(&self) {
        Page::go_to_notifications(self)
    }
    fn rows(&self) -> Vec<String> {
        Page::rows(self)
    }
    fn texts(&self) -> Vec<String> {
        Page::texts(self)
    }
}

#[test]
fn ac3_exactly_one_option_is_selected_and_it_matches_the_stored_mode() {
    let _g = init();
    kawaro_ui::reset_page_memory();
    each_window(|name, open| {
        for (i, mode) in MODES.into_iter().enumerate() {
            let (w, _) = open(Lang::En, &cfg(mode));
            w.go();
            let options = w.options();
            assert_eq!(options.len(), 3, "{name}");
            assert_eq!(selected(&options), vec![i], "{name} {mode:?}");
            assert!(!w.unsaved(), "{name}: a loaded mode is not unsaved");
        }
    });
}

#[test]
fn ac3_the_english_texts_and_the_labels_equal_the_visible_text() {
    let _g = init();
    kawaro_ui::reset_page_memory();
    each_window(|name, open| {
        let (w, _) = open(Lang::En, &cfg(NotifyMode::All));
        w.go();
        assert_eq!(w.list_label(), "Notifications", "{name}");
        let options = w.options();
        for (i, (text, label, _)) in options.iter().enumerate() {
            assert_eq!(text, EN[i], "{name}");
            assert_eq!(label, text, "{name}: the label equals the visible text");
        }
        let texts = w.texts();
        for t in EN {
            assert!(texts.iter().any(|x| x == t), "{name}: {t:?} not on screen");
        }
    });
}

#[test]
fn ac3_the_arabic_texts_and_the_labels_equal_the_visible_text() {
    let _g = init();
    kawaro_ui::reset_page_memory();
    each_window(|name, open| {
        let (w, _) = open(Lang::Ar, &cfg(NotifyMode::Long));
        w.go();
        assert_eq!(w.list_label(), "الإشعارات", "{name}");
        let options = w.options();
        for (i, (text, label, _)) in options.iter().enumerate() {
            assert_eq!(text, AR[i], "{name}");
            assert_eq!(label, text, "{name}");
        }
        let texts = w.texts();
        for t in AR {
            assert!(texts.iter().any(|x| x == t), "{name}: {t:?} not on screen");
        }
    });
}

#[test]
fn ac3_no_string_has_a_long_dash_and_the_two_switches_are_gone() {
    let _g = init();
    kawaro_ui::reset_page_memory();
    each_window(|name, open| {
        for lang in [Lang::En, Lang::Ar] {
            let (w, _) = open(lang, &cfg(NotifyMode::All));
            w.go();
            let mut all = w.rows();
            all.extend(w.texts());
            for (t, l, _) in w.options() {
                all.push(t);
                all.push(l);
            }
            all.push(w.list_label());
            for s in &all {
                assert!(
                    !s.contains('\u{2013}') && !s.contains('\u{2014}'),
                    "{name}: long dash in {s:?}"
                );
            }
            for old in [
                "Notify when a file or operation finishes",
                "Only if it took 10 seconds or more",
                "إشعار عند انتهاء ملف أو عملية",
                "فقط إن استغرق 10 ثوانٍ أو أكثر",
            ] {
                assert!(
                    !all.iter().any(|s| s == old),
                    "{name}: the old switch text {old:?} is still shown"
                );
            }
            assert_eq!(w.rows().len(), 3, "{name}: list, open folder, shortcut");
        }
    });
}

#[test]
fn ac3_picking_another_option_is_unsaved_and_cancel_restores_the_stored_mode() {
    let _g = init();
    kawaro_ui::reset_page_memory();
    each_window(|name, open| {
        let (w, saved) = open(Lang::En, &cfg(NotifyMode::All));
        w.go();
        w.pick(2);
        assert_eq!(selected(&w.options()), vec![2], "{name}");
        assert!(w.unsaved(), "{name}: a pick marks the form unsaved");
        w.cancel();
        w.go();
        assert_eq!(
            selected(&w.options()),
            vec![1],
            "{name}: Cancel restores All"
        );
        assert!(saved.borrow().is_empty(), "{name}: Cancel never saves");
    });
}

#[test]
fn ac3_save_writes_the_picked_mode_for_each_of_the_three() {
    let _g = init();
    kawaro_ui::reset_page_memory();
    each_window(|name, open| {
        for (i, mode) in MODES.into_iter().enumerate() {
            // Start from a different stored mode so the pick is a change.
            let stored = MODES[(i + 1) % 3];
            let (w, saved) = open(Lang::En, &cfg(stored));
            w.go();
            w.pick(i);
            w.save();
            let got = saved.borrow();
            assert_eq!(got.len(), 1, "{name} {mode:?}: Save reaches the callback");
            assert_eq!(got[0].notify_mode, mode, "{name}");
        }
    });
}
