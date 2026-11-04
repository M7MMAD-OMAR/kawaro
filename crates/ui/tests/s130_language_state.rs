//! S-130 AC3 and AC4: the language screen state and its keyboard.
//!
//! Contract pinned by this test (tester), all on kawaro_ui::welcome:
//! - pub enum LanguageKey { Tab, Enter, Space, Escape }
//! - WelcomeWindow::choose_language_visible(&self) -> bool
//! - WelcomeWindow::language_save_failed(&self) -> bool
//! - WelcomeWindow::language_failure_text(&self) -> Option<String>
//! - WelcomeWindow::language_focus(&self) -> usize
//! - WelcomeWindow::language_option_order(&self) -> Vec<Lang>
//! - WelcomeWindow::language_rtl(&self) -> bool
//! - WelcomeWindow::press_language_key(&self, key: LanguageKey) -> bool
//!   (true when a language was saved; Tab moves the focus; Escape closes
//!   the welcome window without saving)
//!
//! The success path leaves choose_language false with no failure line; the
//! failure path keeps the choice visible with the Rust failure line.

use kawaro_core::i18n::Lang;
use kawaro_core::welcome::{Registration, captions, strings};
use kawaro_ui::welcome::{LanguageKey, WelcomeWindow};
use std::cell::RefCell;
use std::rc::Rc;

fn captions_en() -> kawaro_core::welcome::Captions {
    captions(Lang::En, &Registration::Unbound)
}

fn window(lang: Lang, saved: bool, calls: Rc<RefCell<Vec<Lang>>>) -> WelcomeWindow {
    let win = WelcomeWindow::new(lang).expect("a welcome window");
    win.on_language(move |lang| {
        calls.borrow_mut().push(lang);
        saved
    });
    win.open(&captions_en(), false).expect("the window opens");
    win.choose_language();
    win
}

#[test]
fn ac4_a_successful_save_hides_the_choice_and_clears_the_failure_line() {
    i_slint_backend_testing::init_no_event_loop();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let win = window(Lang::En, true, calls.clone());
    assert!(win.choose_language_visible(), "the choice starts visible");
    assert_eq!(win.language_focus(), 0, "the first option has the focus");
    assert!(!win.language_save_failed());

    assert!(
        win.press_language_key(LanguageKey::Enter),
        "Enter saves the focused language"
    );
    assert!(
        !win.choose_language_visible(),
        "the choice disappears after a successful save"
    );
    assert!(!win.language_save_failed(), "no failure line after success");
    assert_eq!(win.language_failure_text(), None);
    assert_eq!(*calls.borrow(), vec![Lang::En]);
}

#[test]
fn ac4_a_failed_save_keeps_the_choice_with_the_rust_failure_line() {
    i_slint_backend_testing::init_no_event_loop();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let win = window(Lang::En, false, calls.clone());

    assert!(
        !win.press_language_key(LanguageKey::Enter),
        "Enter reports a failed save"
    );
    assert!(
        win.choose_language_visible(),
        "a failed save keeps the choice visible"
    );
    assert!(win.language_save_failed(), "the failure line is on");
    assert_eq!(
        win.language_failure_text(),
        Some(strings(Lang::En).language_failed),
        "the failure line is the Rust string"
    );
    assert_eq!(*calls.borrow(), vec![Lang::En]);
}

#[test]
fn ac3_tab_moves_the_focus_between_the_two_options() {
    i_slint_backend_testing::init_no_event_loop();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let win = window(Lang::En, true, calls.clone());
    assert_eq!(win.language_focus(), 0);
    assert!(!win.press_language_key(LanguageKey::Tab));
    assert_eq!(win.language_focus(), 1, "Tab moves to the second option");
    assert!(!win.press_language_key(LanguageKey::Tab));
    assert_eq!(
        win.language_focus(),
        0,
        "Tab wraps back to the first option"
    );
    assert!(calls.borrow().is_empty(), "Tab never saves");
}

#[test]
fn ac3_space_selects_the_focused_option() {
    i_slint_backend_testing::init_no_event_loop();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let win = window(Lang::En, true, calls.clone());
    assert!(!win.press_language_key(LanguageKey::Tab));
    assert_eq!(win.language_focus(), 1);
    assert!(
        win.press_language_key(LanguageKey::Space),
        "Space saves the focused language"
    );
    assert_eq!(*calls.borrow(), vec![Lang::Ar]);
    assert!(!win.choose_language_visible());
}

#[test]
fn ac3_escape_closes_the_welcome_window_without_saving() {
    i_slint_backend_testing::init_no_event_loop();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let win = window(Lang::En, true, calls.clone());
    assert!(!win.press_language_key(LanguageKey::Escape));
    assert!(calls.borrow().is_empty(), "Escape must not save a language");
    assert!(!win.is_visible(), "Escape closes the welcome window");
}

#[test]
fn ac3_the_arabic_build_mirrors_the_card_and_the_option_order() {
    i_slint_backend_testing::init_no_event_loop();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let en = window(Lang::En, true, calls.clone());
    assert!(!en.language_rtl());
    assert_eq!(en.language_option_order(), vec![Lang::En, Lang::Ar]);

    let ar = window(Lang::Ar, true, calls);
    assert!(ar.language_rtl(), "the Arabic build sets the RTL property");
    assert_eq!(
        ar.language_option_order(),
        vec![Lang::Ar, Lang::En],
        "the Arabic build mirrors the option order by hand"
    );
}
