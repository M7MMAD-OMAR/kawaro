//! S-094b (backlog/S-094b-about-page-actions.md), the daemon half: the About
//! page links go through the injected opener, like the hive's request cell
//! (S-108). Written from the acceptance criteria only.
//!
//! Hooks these tests pin:
//! - `ui::wire_about(win: &SettingsWindow, opener: Rc<dyn Fn(&str)>)` and
//!   `ui::wire_about_welcome(win: &WelcomeWindow, opener: Rc<dyn Fn(&str)>)`
//!   install the window's `on_open_link` handler (the opener gets the
//!   resolved URL; an unknown id never reaches it). Start up calls both with
//!   the real opener (`notify_logic::open_url_command` through
//!   `notify::open_folder`), the tests with a counting spy.
//! - The window hooks of `crates/ui/tests/s094b_about.rs`: `select_page`,
//!   `press_label`, `enter_on_label`, `open_link`.
//! - `wire_about` opens no Kawaro window: its body holds no `show(` and no
//!   window constructor.
//!
//! The opener binary names are never spelled here: the command table is
//! built from the helper's own output (S-108 pitfall).

use crate::notify_logic::{Os, open_url_command};
use crate::ui::{wire_about, wire_about_welcome};
use kawaro_core::about::strings;
use kawaro_core::config::Config;
use kawaro_core::i18n::Lang;
use kawaro_core::welcome::Captions;
use kawaro_ui::SettingsWindow;
use kawaro_ui::welcome::WelcomeWindow;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

const WEBSITE: &str = "https://kawaro.sbarah.com/";
const SOURCE: &str = "https://github.com/M7MMAD-OMAR/kawaro";

fn init() {
    thread_local!(static INIT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) });
    if !INIT.with(|i| i.replace(true)) {
        i_slint_backend_testing::init_no_event_loop();
    }
}

fn settings(spy: &Rc<RefCell<Vec<String>>>) -> SettingsWindow {
    init();
    let w = SettingsWindow::new(Lang::En).unwrap();
    w.open(&Config::default()).unwrap();
    let s = spy.clone();
    wire_about(
        &w,
        Rc::new(move |url: &str| s.borrow_mut().push(url.to_string())),
    );
    w.select_page(4);
    w
}

fn welcome(spy: &Rc<RefCell<Vec<String>>>) -> WelcomeWindow {
    init();
    let w = WelcomeWindow::new(Lang::En).unwrap();
    let captions = Captions {
        steps: std::array::from_fn(|i| format!("step {i}")),
    };
    w.open(&captions, true).unwrap();
    let s = spy.clone();
    wire_about_welcome(
        &w,
        Rc::new(move |url: &str| s.borrow_mut().push(url.to_string())),
    );
    w.select_page(5);
    w
}

#[test]
fn ac2_visiting_about_opens_nothing_and_each_press_opens_once() {
    let spy = Rc::new(RefCell::new(Vec::new()));
    let w = settings(&spy);
    assert_eq!(spy.borrow().len(), 0, "0 calls on show");
    let s = strings(Lang::En);
    assert!(w.press_label(&s.website_label.to_string()));
    assert_eq!(*spy.borrow(), [WEBSITE], "1 call, exactly its URL");
    assert!(w.press_label(&s.source_label.to_string()));
    assert_eq!(*spy.borrow(), [WEBSITE, SOURCE], "2 calls after both");
}

#[test]
fn ac2_enter_on_a_link_opens_it_and_an_unknown_id_opens_nothing() {
    let spy = Rc::new(RefCell::new(Vec::new()));
    let w = settings(&spy);
    let s = strings(Lang::En);
    assert!(w.enter_on_label(&s.source_label.to_string()));
    assert_eq!(*spy.borrow(), [SOURCE]);
    w.open_link("nope");
    w.open_link("");
    assert_eq!(spy.borrow().len(), 1, "an unknown id makes 0 calls");
}

#[test]
fn ac2_the_welcome_window_about_opens_through_the_same_opener() {
    let spy = Rc::new(RefCell::new(Vec::new()));
    let w = welcome(&spy);
    assert_eq!(spy.borrow().len(), 0);
    let s = strings(Lang::En);
    assert!(w.press_label(&s.website_label.to_string()));
    assert_eq!(*spy.borrow(), [WEBSITE]);
    w.open_link("source");
    assert_eq!(*spy.borrow(), [WEBSITE, SOURCE]);
    w.open_link("other");
    assert_eq!(spy.borrow().len(), 2);
}

#[test]
fn ac2_open_url_command_stays_one_argument_per_os() {
    for url in [WEBSITE, SOURCE] {
        for os in [Os::Linux, Os::Windows] {
            let (program, args) = open_url_command(os, url);
            assert!(!program.is_empty());
            assert_eq!(args, [url], "the URL is the only argument");
        }
    }
    let (linux, _) = open_url_command(Os::Linux, WEBSITE);
    let (windows, _) = open_url_command(Os::Windows, WEBSITE);
    assert_ne!(linux, windows, "one opener per platform");
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn above_test(text: &str) -> String {
    text.lines()
        .take_while(|l| l.trim() != "#[cfg(test)]")
        .map(|l| format!("{l}\n"))
        .collect()
}

/// The text of `pub fn <name>` up to its closing line.
fn body_of(text: &str, name: &str) -> String {
    let marker = format!("pub fn {name}");
    let start = text
        .find(&marker)
        .unwrap_or_else(|| panic!("`{marker}` must exist in ui.rs"));
    let rest = &text[start..];
    let end = rest.find("\n}\n").expect("the function ends");
    rest[..end].to_string()
}

#[test]
fn ac2_wire_about_opens_no_kawaro_window() {
    let ui = above_test(&std::fs::read_to_string(root().join("crates/daemon/src/ui.rs")).unwrap());
    for name in ["wire_about", "wire_about_welcome"] {
        let body = body_of(&ui, name);
        for needle in [concat!("show", "("), "Window::new", "Hive::new"] {
            assert_eq!(body.matches(needle).count(), 0, "{name} holds `{needle}`");
        }
        assert!(body.contains("opener"), "{name} goes through the opener");
    }
}

#[test]
fn ac2_start_up_wires_both_windows_with_the_real_opener() {
    let ui = above_test(&std::fs::read_to_string(root().join("crates/daemon/src/ui.rs")).unwrap());
    // Two definitions plus at least one call for each window.
    assert!(
        ui.matches("wire_about(").count() >= 2,
        "settings window wired"
    );
    assert!(
        ui.matches("wire_about_welcome(").count() >= 2,
        "welcome window wired"
    );
    assert!(
        ui.contains("open_url_command"),
        "the real opener builds its command"
    );
}
