//! S-094b (backlog/S-094b-about-page-actions.md): the About page actions in
//! the real windows. Written from the acceptance criteria only. Headless
//! Slint platform with mocked time, no compositor.
//!
//! Hooks these tests pin. All are `#[doc(hidden)] pub` inherent methods with
//! the SAME name and signature on `SettingsWindow` and `WelcomeWindow`
//! (geometry in logical px, window coordinates, from the real laid out
//! elements; buttons are found by their accessible label and acted on with
//! real pointer or key events):
//! - `on_open_link(f: impl Fn(&str) + 'static)`: called with the resolved
//!   URL, never for an unknown id. `on_copy_version(f: impl Fn(&str) +
//!   'static)`: called with `kawaro_core::about::copy_text(version)`.
//! - `open_link(&self, id: &str)`: the Rust seam behind the link buttons; an
//!   id `link_url` does not know makes 0 calls.
//! - `about_buttons(&self) -> Vec<(String, Rect)>`: accessible label and
//!   rect of every interactive button on the About page, in reading order
//!   (the two links, "Copy version" and "Check for updates").
//! - `press_label(&self, label: &str) -> bool` (pointer move, press, release
//!   at the centre of the element with that accessible label) and
//!   `enter_on_label(&self, label: &str) -> bool` (focus it, send Return);
//!   both return whether the element exists.
//! - `press_copy_version(&self)`, `copy_button_text(&self) -> String` (the
//!   text drawn on the copy button), `press_about_check(&self)` (a real
//!   click on the About "Check for updates" button) and
//!   `about_check_enabled(&self) -> bool`.
//! - `update_check_enabled(&self) -> bool` also on `WelcomeWindow`;
//!   `set_reduced_motion(&self, bool)` also on `SettingsWindow`.
//! - `shell.slint` About block: callbacks `open-link(string)` and
//!   `copy-version()`, no URL literal; `design/tokens.json` gets
//!   `motion.copied-feedback.duration-ms` = 2000.
//! - About is page 4 in the settings window, 5 in the welcome window;
//!   Updates is 3 and 4.

use kawaro_core::about::{copy_text, link_url, strings};
use kawaro_core::config::Config;
use kawaro_core::welcome::Captions;
use kawaro_ui::welcome::WelcomeWindow;
use kawaro_ui::{Lang, Rect, SettingsWindow, UpdateState};
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const WEBSITE: &str = "https://kawaro.sbarah.com/";
const SOURCE: &str = "https://github.com/M7MMAD-OMAR/kawaro";

static LOCK: Mutex<()> = Mutex::new(());

fn init() -> MutexGuard<'static, ()> {
    thread_local! {
        static INIT: std::sync::Once = const { std::sync::Once::new() };
    }
    INIT.with(|once| once.call_once(i_slint_backend_testing::init_no_event_loop));
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn turn(ms: u64) {
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(ms));
    slint::platform::update_timers_and_animations();
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    let p = root().join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} must exist: {e}", p.display()))
}

fn above_test(text: &str) -> String {
    text.lines()
        .take_while(|l| l.trim() != "#[cfg(test)]")
        .map(|l| format!("{l}\n"))
        .collect()
}

/// What both windows expose, so every check runs on both.
trait About {
    const ABOUT: usize;
    const UPDATES: usize;
    fn make(lang: Lang) -> Self;
    fn a_select_page(&self, i: usize);
    fn a_page(&self) -> usize;
    fn a_set_window_size(&self, w: f32, h: f32);
    fn a_on_open_link(&self, f: impl Fn(&str) + 'static);
    fn a_on_copy_version(&self, f: impl Fn(&str) + 'static);
    fn a_on_update_check(&self, f: impl Fn() + 'static);
    fn a_open_link(&self, id: &str);
    fn a_buttons(&self) -> Vec<(String, Rect)>;
    fn a_press_label(&self, label: &str) -> bool;
    fn a_enter_on_label(&self, label: &str) -> bool;
    fn a_press_copy(&self);
    fn a_copy_text(&self) -> String;
    fn a_press_check(&self);
    fn a_check_enabled(&self) -> bool;
    fn a_update_check_enabled(&self) -> bool;
    fn a_set_update_view(&self, state: UpdateState);
    fn a_set_reduced(&self, reduced: bool);
    fn a_unlabelled(&self) -> usize;
    fn a_visible_labels(&self) -> Vec<String>;
    fn a_scroll_extent(&self) -> (f32, f32);
    fn a_content_rect(&self) -> Rect;
}

macro_rules! about {
    ($t:ty, $about:expr, $updates:expr, $make:expr) => {
        impl About for $t {
            const ABOUT: usize = $about;
            const UPDATES: usize = $updates;
            fn make(lang: Lang) -> Self {
                $make(lang)
            }
            fn a_select_page(&self, i: usize) {
                <$t>::select_page(self, i)
            }
            fn a_page(&self) -> usize {
                <$t>::page(self)
            }
            fn a_set_window_size(&self, w: f32, h: f32) {
                <$t>::set_window_size(self, w, h)
            }
            fn a_on_open_link(&self, f: impl Fn(&str) + 'static) {
                <$t>::on_open_link(self, f)
            }
            fn a_on_copy_version(&self, f: impl Fn(&str) + 'static) {
                <$t>::on_copy_version(self, f)
            }
            fn a_on_update_check(&self, f: impl Fn() + 'static) {
                <$t>::on_update_check(self, f)
            }
            fn a_open_link(&self, id: &str) {
                <$t>::open_link(self, id)
            }
            fn a_buttons(&self) -> Vec<(String, Rect)> {
                <$t>::about_buttons(self)
            }
            fn a_press_label(&self, label: &str) -> bool {
                <$t>::press_label(self, label)
            }
            fn a_enter_on_label(&self, label: &str) -> bool {
                <$t>::enter_on_label(self, label)
            }
            fn a_press_copy(&self) {
                <$t>::press_copy_version(self)
            }
            fn a_copy_text(&self) -> String {
                <$t>::copy_button_text(self)
            }
            fn a_press_check(&self) {
                <$t>::press_about_check(self)
            }
            fn a_check_enabled(&self) -> bool {
                <$t>::about_check_enabled(self)
            }
            fn a_update_check_enabled(&self) -> bool {
                <$t>::update_check_enabled(self)
            }
            fn a_set_update_view(&self, state: UpdateState) {
                <$t>::set_update_view(self, VERSION, state, true)
            }
            fn a_set_reduced(&self, reduced: bool) {
                <$t>::set_reduced_motion(self, reduced)
            }
            fn a_unlabelled(&self) -> usize {
                <$t>::unlabelled_controls(self)
            }
            fn a_visible_labels(&self) -> Vec<String> {
                <$t>::visible_labels(self)
            }
            fn a_scroll_extent(&self) -> (f32, f32) {
                <$t>::scroll_extent(self)
            }
            fn a_content_rect(&self) -> Rect {
                <$t>::content_rect(self)
            }
        }
    };
}

about!(SettingsWindow, 4, 3, |lang| {
    let w = SettingsWindow::new(lang).expect("window");
    w.open(&Config::default()).expect("open");
    w
});
about!(WelcomeWindow, 5, 4, |lang| {
    let w = WelcomeWindow::new(lang).expect("window");
    let captions = Captions {
        steps: std::array::from_fn(|i| format!("step {i}")),
    };
    w.open(&captions, true).expect("open");
    w
});

/// A window on the About page at 880x640.
fn on_about<W: About>(lang: Lang) -> W {
    let w = W::make(lang);
    w.a_set_window_size(880.0, 640.0);
    w.a_select_page(W::ABOUT);
    w
}

fn count<W: About>(w: &W) -> Rc<Cell<u32>> {
    let n = Rc::new(Cell::new(0u32));
    let n2 = n.clone();
    w.a_on_update_check(move || n2.set(n2.get() + 1));
    n
}

fn spy<W: About>(w: &W) -> Rc<RefCell<Vec<String>>> {
    let log = Rc::new(RefCell::new(Vec::new()));
    let l = log.clone();
    w.a_on_open_link(move |url| l.borrow_mut().push(url.to_string()));
    log
}

fn copier<W: About>(w: &W) -> Rc<RefCell<Vec<String>>> {
    let log = Rc::new(RefCell::new(Vec::new()));
    let l = log.clone();
    w.a_on_copy_version(move |t| l.borrow_mut().push(t.to_string()));
    log
}

// ---------------------------------------------------------------- AC1

fn ac1_two_links_with_exact_labels<W: About>() {
    let _g = init();
    for lang in [Lang::En, Lang::Ar] {
        let s = strings(lang);
        let w = on_about::<W>(lang);
        let labels: Vec<String> = w.a_buttons().into_iter().map(|(l, _)| l).collect();
        let links: Vec<&String> = labels.iter().filter(|l| l.contains("https://")).collect();
        assert_eq!(links.len(), 2, "{lang:?}: {labels:?}");
        for want in [s.website_label.to_string(), s.source_label.to_string()] {
            assert_eq!(
                labels.iter().filter(|l| **l == want).count(),
                1,
                "{lang:?}: `{want}` in {labels:?}"
            );
        }
        if lang == Lang::En {
            assert!(labels.contains(&format!("Website: {WEBSITE}")));
            assert!(labels.contains(&format!("Source code: {SOURCE}")));
        }
        assert!(w.a_visible_labels().contains(&s.website_label.to_string()));
    }
}

#[test]
fn ac1_settings_about_shows_exactly_two_links_labelled_with_their_target() {
    ac1_two_links_with_exact_labels::<SettingsWindow>();
}

#[test]
fn ac1_welcome_about_shows_exactly_two_links_labelled_with_their_target() {
    ac1_two_links_with_exact_labels::<WelcomeWindow>();
}

// ---------------------------------------------------------------- AC2

fn ac2_opener<W: About>() {
    let _g = init();
    let s = strings(Lang::En);
    let w = W::make(Lang::En);
    let log = spy(&w);
    w.a_select_page(W::ABOUT);
    assert_eq!(
        log.borrow().len(),
        0,
        "0 calls on show and on visiting About"
    );
    assert!(w.a_press_label(&s.website_label.to_string()));
    assert_eq!(*log.borrow(), [WEBSITE]);
    assert!(w.a_press_label(&s.source_label.to_string()));
    assert_eq!(*log.borrow(), [WEBSITE, SOURCE]);
}

fn ac2_enter<W: About>() {
    let _g = init();
    let s = strings(Lang::En);
    let w = on_about::<W>(Lang::En);
    let log = spy(&w);
    assert!(w.a_enter_on_label(&s.website_label.to_string()));
    assert_eq!(*log.borrow(), [WEBSITE]);
    assert!(w.a_enter_on_label(&s.source_label.to_string()));
    assert_eq!(*log.borrow(), [WEBSITE, SOURCE]);
}

fn ac2_unknown_id<W: About>() {
    let _g = init();
    let w = on_about::<W>(Lang::En);
    let log = spy(&w);
    for id in ["", "file", "https://x", "website "] {
        w.a_open_link(id);
    }
    assert_eq!(log.borrow().len(), 0);
    w.a_open_link("website");
    w.a_open_link("source");
    assert_eq!(*log.borrow(), [WEBSITE, SOURCE]);
    assert_eq!(link_url("website"), Some(WEBSITE));
}

#[test]
fn ac2_settings_each_link_press_calls_the_opener_once_with_its_url() {
    ac2_opener::<SettingsWindow>();
}

#[test]
fn ac2_welcome_each_link_press_calls_the_opener_once_with_its_url() {
    ac2_opener::<WelcomeWindow>();
}

#[test]
fn ac2_settings_enter_on_the_focused_link_opens_it() {
    ac2_enter::<SettingsWindow>();
}

#[test]
fn ac2_welcome_enter_on_the_focused_link_opens_it() {
    ac2_enter::<WelcomeWindow>();
}

#[test]
fn ac2_settings_an_unknown_id_through_the_seam_makes_no_call() {
    ac2_unknown_id::<SettingsWindow>();
}

#[test]
fn ac2_welcome_an_unknown_id_through_the_seam_makes_no_call() {
    ac2_unknown_id::<WelcomeWindow>();
}

#[test]
fn ac2_the_ui_crate_spells_no_opener_binary_and_no_process_api() {
    let needles = [
        concat!("xdg", "-open"),
        concat!("explorer", ".exe"),
        concat!("Com", "mand"),
        concat!("std::", "process"),
    ];
    let mut files: Vec<(String, String)> = Vec::new();
    for dir in ["crates/ui/src", "crates/ui/ui"] {
        for e in std::fs::read_dir(root().join(dir)).unwrap() {
            let p = e.unwrap().path();
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            let is_src = name.ends_with(".rs") || name.ends_with(".slint");
            // motion.rs holds the old gsettings reduced motion probe, which
            // predates S-094b and is not an opener.
            if is_src && !name.ends_with("_tests.rs") && name != "motion.rs" {
                let text = std::fs::read_to_string(&p).unwrap();
                files.push((name, above_test(&text)));
            }
        }
    }
    assert!(files.len() > 5, "the scan found the sources");
    for (name, text) in &files {
        for n in needles {
            assert_eq!(text.matches(n).count(), 0, "{name} holds `{n}`");
        }
    }
    let lib = above_test(&read("crates/ui/src/lib.rs"));
    assert_eq!(
        lib.matches("fn open_url").count(),
        0,
        "no open_url in kawaro_ui"
    );
}

#[test]
fn ac2_ac6_no_url_literal_in_any_slint_file() {
    for e in std::fs::read_dir(root().join("crates/ui/ui")).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_some_and(|x| x == "slint") {
            let text = std::fs::read_to_string(&p).unwrap();
            assert_eq!(text.matches("http").count(), 0, "{}", p.display());
        }
    }
}

// ---------------------------------------------------------------- AC3

fn copy_label(lang: Lang, copied: bool) -> String {
    let s = strings(lang);
    if copied { s.copied } else { s.copy_version }.to_string()
}

fn ac3_copies_once<W: About>() {
    let _g = init();
    let w = on_about::<W>(Lang::En);
    let log = copier(&w);
    assert_eq!(log.borrow().len(), 0);
    w.a_press_copy();
    assert_eq!(*log.borrow(), [copy_text(VERSION)]);
    assert_eq!(log.borrow()[0], VERSION);
}

fn ac3_timing<W: About>(lang: Lang) {
    let _g = init();
    let w = on_about::<W>(lang);
    let log = copier(&w);
    turn(0);
    assert_eq!(w.a_copy_text(), copy_label(lang, false));
    w.a_press_copy();
    assert_eq!(w.a_copy_text(), copy_label(lang, true), "0 ms");
    turn(1999);
    assert_eq!(w.a_copy_text(), copy_label(lang, true), "1999 ms");
    turn(1);
    assert_eq!(w.a_copy_text(), copy_label(lang, false), "2000 ms");
    assert_eq!(log.borrow().len(), 1);
}

fn ac3_restart<W: About>() {
    let _g = init();
    let w = on_about::<W>(Lang::En);
    let log = copier(&w);
    turn(0);
    w.a_press_copy();
    turn(1000);
    w.a_press_copy();
    assert_eq!(log.borrow().len(), 2, "the second press copies again");
    turn(1999);
    assert_eq!(w.a_copy_text(), "Copied", "2999 ms after the first press");
    turn(1);
    assert_eq!(w.a_copy_text(), "Copy version", "3000 ms");
}

#[test]
fn ac3_settings_a_press_calls_the_copier_once_with_the_version() {
    ac3_copies_once::<SettingsWindow>();
}

#[test]
fn ac3_welcome_a_press_calls_the_copier_once_with_the_version() {
    ac3_copies_once::<WelcomeWindow>();
}

#[test]
fn ac3_settings_the_label_is_copied_for_2000_ms_in_en_and_ar() {
    ac3_timing::<SettingsWindow>(Lang::En);
    ac3_timing::<SettingsWindow>(Lang::Ar);
}

#[test]
fn ac3_welcome_the_label_is_copied_for_2000_ms_in_en_and_ar() {
    ac3_timing::<WelcomeWindow>(Lang::En);
    ac3_timing::<WelcomeWindow>(Lang::Ar);
}

#[test]
fn ac3_settings_a_second_press_restarts_the_window() {
    ac3_restart::<SettingsWindow>();
}

#[test]
fn ac3_welcome_a_second_press_restarts_the_window() {
    ac3_restart::<WelcomeWindow>();
}

// ---------------------------------------------------------------- AC4

fn ac4_shortcut<W: About>() {
    let _g = init();
    let w = on_about::<W>(Lang::En);
    let n = count(&w);
    assert_eq!(w.a_page(), W::ABOUT);
    w.a_press_check();
    assert_eq!(w.a_page(), W::UPDATES);
    assert_eq!(n.get(), 1);
}

fn ac4_disabled<W: About>() {
    let _g = init();
    let w = on_about::<W>(Lang::En);
    let n = count(&w);
    w.a_set_update_view(UpdateState::Checking);
    assert!(!w.a_check_enabled(), "disabled while a check runs");
    w.a_press_check();
    assert_eq!(n.get(), 0);
    assert_eq!(w.a_page(), W::ABOUT, "a disabled button does not navigate");
    for state in [
        UpdateState::Ready,
        UpdateState::UpToDate,
        UpdateState::Checking,
    ] {
        w.a_set_update_view(state);
        w.a_select_page(W::UPDATES);
        let on_updates = w.a_update_check_enabled();
        w.a_select_page(W::ABOUT);
        assert_eq!(w.a_check_enabled(), on_updates);
    }
}

#[test]
fn ac4_settings_check_for_updates_selects_page_3_and_fires_once() {
    ac4_shortcut::<SettingsWindow>();
}

#[test]
fn ac4_welcome_check_for_updates_selects_page_4_and_fires_once() {
    ac4_shortcut::<WelcomeWindow>();
}

#[test]
fn ac4_settings_the_button_is_disabled_while_a_check_runs() {
    ac4_disabled::<SettingsWindow>();
}

#[test]
fn ac4_welcome_the_button_is_disabled_while_a_check_runs() {
    ac4_disabled::<WelcomeWindow>();
}

// ---------------------------------------------------------------- AC5

fn ac5_labels_and_mirroring<W: About>() {
    let _g = init();
    for lang in [Lang::En, Lang::Ar] {
        let s = strings(lang);
        let w = on_about::<W>(lang);
        assert_eq!(w.a_unlabelled(), 0, "{lang:?}: an unlabelled control");
        let buttons = w.a_buttons();
        assert_eq!(buttons.len(), 4, "{lang:?}: 2 links, copy, check");
        for (label, _) in &buttons {
            assert!(!label.trim().is_empty(), "{lang:?}: empty label");
        }
        let labels: Vec<&str> = buttons.iter().map(|(l, _)| l.as_str()).collect();
        for want in [
            s.website_label.to_string(),
            s.source_label.to_string(),
            s.check_updates.to_string(),
        ] {
            assert!(
                labels.contains(&want.as_str()),
                "{lang:?}: `{want}` in {labels:?}"
            );
        }
        let (first, last) = (buttons[0].1, buttons[3].1);
        if lang == Lang::Ar {
            assert!(first.x > last.x, "Arabic mirrors: {first:?} vs {last:?}");
        } else {
            assert!(first.x < last.x, "English reads left to right");
        }
        let content = w.a_content_rect();
        for (label, r) in &buttons {
            assert!(
                r.x >= content.x - 0.5 && r.x + r.w <= content.x + content.w + 0.5,
                "{lang:?}: `{label}` sticks out of the column: {r:?} in {content:?}"
            );
        }
    }
}

#[test]
fn ac5_settings_every_control_is_labelled_and_the_row_mirrors_in_arabic() {
    ac5_labels_and_mirroring::<SettingsWindow>();
}

#[test]
fn ac5_welcome_every_control_is_labelled_and_the_row_mirrors_in_arabic() {
    ac5_labels_and_mirroring::<WelcomeWindow>();
}

// ---------------------------------------------------------------- AC6

fn ac6_copied_swaps_at_once<W: About>() {
    let _g = init();
    for reduced in [true, false] {
        let w = on_about::<W>(Lang::En);
        w.a_set_reduced(reduced);
        let _log = copier(&w);
        turn(0);
        assert_eq!(w.a_copy_text(), "Copy version");
        w.a_press_copy();
        // No frame is advanced: the swap needs 0 animation frames.
        assert_eq!(w.a_copy_text(), "Copied", "reduced motion {reduced}");
    }
}

#[test]
fn ac6_settings_the_copied_swap_needs_no_animation_frame() {
    ac6_copied_swaps_at_once::<SettingsWindow>();
}

#[test]
fn ac6_welcome_the_copied_swap_needs_no_animation_frame() {
    ac6_copied_swaps_at_once::<WelcomeWindow>();
}

/// The text of the block opened by the first `{` after `marker`.
fn block_after(text: &str, marker: &str) -> String {
    let start = text
        .find(marker)
        .unwrap_or_else(|| panic!("`{marker}` not found"));
    let open = start + text[start..].find('{').expect("an opening brace");
    let mut depth = 0usize;
    for (i, c) in text[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return text[start..open + i + 1].to_string();
                }
            }
            _ => {}
        }
    }
    panic!("unbalanced block after `{marker}`");
}

fn glued_unit(text: &str, unit: &str) -> bool {
    text.match_indices(unit).any(|(i, _)| {
        let prev = text[..i].chars().last();
        let after = text[i + unit.len()..].chars().next();
        prev.is_some_and(|c| c.is_ascii_digit())
            && !after.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '-')
    })
}

#[test]
fn ac6_the_about_block_holds_no_hand_written_value_and_no_animation() {
    let shell = read("crates/ui/ui/shell.slint");
    let about = block_after(&shell, "if root.sp == 4");
    assert!(
        about.contains("open-link"),
        "the About block calls open-link"
    );
    assert!(
        about.contains("copy-version"),
        "the About block calls copy-version"
    );
    assert!(
        about.contains("update-check"),
        "the About block reuses update-check"
    );
    assert_eq!(about.matches("animate").count(), 0, "no animate statement");
    assert_eq!(about.matches("drop-shadow").count(), 0);
    assert_eq!(about.matches("gradient").count(), 0);
    assert!(!about.contains('\u{2013}') && !about.contains('\u{2014}'));
    let b = about.as_bytes();
    for (i, w) in b.windows(2).enumerate() {
        assert!(
            !(w[0] == b'#' && w[1].is_ascii_hexdigit()),
            "colour literal near byte {i}"
        );
    }
    for unit in ["px", "ms"] {
        assert!(!glued_unit(&about, unit), "a hand written `{unit}` number");
    }
}

#[test]
fn ac6_the_copied_feedback_duration_is_a_motion_token() {
    let tokens: serde_json::Value = serde_json::from_str(&read("design/tokens.json")).unwrap();
    let ms = &tokens["motion"]["copied-feedback"]["duration-ms"];
    assert_eq!(
        ms.as_u64(),
        Some(2000),
        "motion.copied-feedback.duration-ms"
    );
}

// ---------------------------------------------------------------- AC7

fn ac7_no_scroll<W: About>() {
    let _g = init();
    for lang in [Lang::En, Lang::Ar] {
        let w = on_about::<W>(lang);
        let (content, visible) = w.a_scroll_extent();
        assert!(
            content <= visible + 0.5,
            "{lang:?}: About scrolls, content {content} in {visible}"
        );
    }
}

#[test]
fn ac7_settings_about_needs_no_scrolling_at_880_by_640() {
    ac7_no_scroll::<SettingsWindow>();
}

#[test]
fn ac7_welcome_about_needs_no_scrolling_at_880_by_640() {
    ac7_no_scroll::<WelcomeWindow>();
}
