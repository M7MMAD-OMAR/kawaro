//! S-131 AC3: Escape on the welcome window closes the embedded settings page
//! like Back and hides the window, including while a text field has focus.
//! Contract pinned by this test (tester), doc(hidden) hooks on WelcomeWindow:
//! - pub fn press_escape(&self): dispatches the Escape key through the shell
//!   (the same path as pressing Escape, not a direct press_cancel).
//! - pub fn focus_settings_text_field(&self) -> bool: focuses the first text
//!   field of the open settings page; true when one was found.
//!
//! The shell escape callback is already declared; welcome.slint forwards it
//! to Rust.

use kawaro_core::config::Config;
use kawaro_core::i18n::Lang;
use kawaro_core::welcome::{Registration, captions};
use kawaro_ui::welcome::WelcomeWindow;
use std::cell::Cell;
use std::rc::Rc;

fn window() -> WelcomeWindow {
    i_slint_backend_testing::init_no_event_loop();
    let win = WelcomeWindow::new(Lang::En).expect("a welcome window");
    let steps = captions(Lang::En, &Registration::Unbound);
    win.open(&steps, false).expect("the window opens");
    win
}

#[test]
fn ac3_escape_on_the_introduction_hides_the_welcome_window() {
    let win = window();
    assert!(win.is_visible());
    win.press_escape();
    assert!(
        !win.is_visible(),
        "Escape on the Introduction hides the welcome window"
    );
}

#[test]
fn ac3_escape_on_a_settings_page_cancels_and_hides_without_saving() {
    let win = window();
    let saved = Config::default();
    let saves = Rc::new(Cell::new(0));
    let counter = saves.clone();
    win.on_settings_save(move |_config| {
        counter.set(counter.get() + 1);
        true
    });
    win.show_settings(&saved);
    win.show_settings_page(2);
    assert!(win.settings_visible());

    win.press_escape();
    assert!(
        !win.settings_visible(),
        "the page returns to the Introduction"
    );
    assert!(!win.is_visible(), "Escape then hides the window");
    assert_eq!(saves.get(), 0, "Escape never saves");

    // The saved values are still the ones that were opened.
    win.show_settings(&saved);
    assert!(win.settings_visible(), "the settings can be opened again");
    assert_eq!(saves.get(), 0, "no save happened while cancelling");
}

#[test]
fn ac3_escape_works_while_a_text_field_has_focus() {
    let win = window();
    let saves = Rc::new(Cell::new(0));
    let counter = saves.clone();
    win.on_settings_save(move |_config| {
        counter.set(counter.get() + 1);
        true
    });
    win.show_settings(&Config::default());
    assert!(
        win.focus_settings_text_field(),
        "the settings page has a text field to focus"
    );

    win.press_escape();
    assert!(!win.settings_visible());
    assert!(!win.is_visible());
    assert_eq!(
        saves.get(),
        0,
        "Escape with a focused text field never saves"
    );
}
