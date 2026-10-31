//! S-094 (backlog/S-094-settings-app-shell.md): the app shell shared by the
//! settings window and the welcome window. Written from the acceptance
//! criteria only.
//!
//! The public surface these tests pin down (the implementer builds it). All
//! hooks are `#[doc(hidden)] pub` inherent methods with the SAME name and
//! signature on `SettingsWindow` and `WelcomeWindow`; geometry is in logical
//! px, in window coordinates, read from the real laid out elements:
//! - `kawaro_ui::Rect { x, y, w, h: f32 }` (Copy, Debug, PartialEq).
//! - `kawaro_ui::reset_page_memory()` and `kawaro_ui::privacy_statement(Lang)`.
//! - Window: `set_window_size(w, h)`, `window_size()`, `min_window_size()`.
//! - Pages: `page()`, `select_page(i)`, `rail_labels()`, `rail_focus()`,
//!   `focus_rail()`, `page_rows()` (row label texts of the current page),
//!   `visible_labels()` (accessible labels on screen), `visible_texts()`
//!   (texts on screen), `unlabelled_controls()` (count of interactive
//!   elements, rail items and the logo with an empty accessible label).
//! - Geometry: `header_rect()`, `logo_rect()`, `name_rect()`,
//!   `version_rect()`, `rail_rect()`, `rail_item_rect(i)`, `page_area_rect()`
//!   (the scrolling viewport beside the rail, under the header, above the
//!   footer), `content_rect()` (the centred column), `footer_rect()`,
//!   `row_rect(i)`, `row_label_rect(i)`, `row_control_rect(i)` (rows of the
//!   current page), `film_rect()` (Some only on Introduction).
//! - `scroll_extent() -> (content_height, visible_height)` of the page area.
//! - Text: `header_version_text()` is exactly the version, no prefix;
//!   `header_name_text()`; `logo_label()`.
//! - State: `unsaved_marker_visible()`, `save_row_visible()`,
//!   `set_theme(Theme)`, `press_save()`, `press_cancel()`,
//!   `send_key(text, ctrl, shift)` (text is a Slint key char as a string).
//! - Files: `crates/ui/ui/shell.slint` holds the shared shell (a
//!   `NavigationRail`) and both `settings.slint` and `welcome.slint` import it.
//! - Tokens: group `layout` in `design/tokens.json` with the S-094 values.
//!
//! Existing tests the implementer may update, and why: the source scanning
//! tests `ac6_rtl_mirrors_the_button_row_in_arabic` and
//! `ac4_both_buttons_carry_descriptive_accessible_labels` in
//! `s087_welcome.rs`, `ac7_slint_source_has_an_updates_section...` in
//! `s090_updates.rs` and `ac6_one_version_line_...` (it greps
//! `welcome.slint` for "version") only because the markup moves into
//! `shell.slint`; they may be pointed at the new file and nothing else about
//! them changes. The four welcome PNG snapshots stay until the manager
//! approves candidates.

use kawaro_core::config::{Config, Theme};
use kawaro_core::welcome::Captions;
use kawaro_ui::{Lang, Rect, SettingsWindow, UpdateState};
use slint::platform::Key;
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Mutex, MutexGuard};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const NAME: &str = "كوارو | Kawaro";
const PRIVACY_EN: &str = "No telemetry. No file leaves your computer.";

#[allow(dead_code)]
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[allow(dead_code)]
fn read(rel: &str) -> String {
    let p = root().join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} must exist: {e}", p.display()))
}

fn has_arabic(s: &str) -> bool {
    s.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c))
}

fn right(r: Rect) -> f32 {
    r.x + r.w
}
fn bottom(r: Rect) -> f32 {
    r.y + r.h
}

/// The page memory is process wide state: every test that opens a window
/// holds this lock for its whole body.
static LOCK: Mutex<()> = Mutex::new(());

fn init() -> MutexGuard<'static, ()> {
    thread_local! {
        static INIT: std::sync::Once = const { std::sync::Once::new() };
    }
    INIT.with(|once| once.call_once(i_slint_backend_testing::init_no_event_loop));
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// What both windows expose, so every check runs on both.
trait Shell {
    fn sh_set_window_size(&self, w: f32, h: f32);
    fn sh_window_size(&self) -> (f32, f32);
    fn sh_min_window_size(&self) -> (f32, f32);
    fn sh_page(&self) -> usize;
    fn sh_select_page(&self, i: usize);
    fn sh_rail_labels(&self) -> Vec<String>;
    fn sh_rail_focus(&self) -> usize;
    fn sh_focus_rail(&self);
    fn sh_page_rows(&self) -> Vec<String>;
    fn sh_visible_labels(&self) -> Vec<String>;
    fn sh_visible_texts(&self) -> Vec<String>;
    fn sh_unlabelled_controls(&self) -> usize;
    fn sh_header_rect(&self) -> Rect;
    fn sh_logo_rect(&self) -> Rect;
    fn sh_name_rect(&self) -> Rect;
    fn sh_version_rect(&self) -> Rect;
    fn sh_rail_rect(&self) -> Rect;
    fn sh_rail_item_rect(&self, i: usize) -> Rect;
    fn sh_page_area_rect(&self) -> Rect;
    fn sh_content_rect(&self) -> Rect;
    fn sh_footer_rect(&self) -> Rect;
    fn sh_row_rect(&self, i: usize) -> Rect;
    fn sh_row_label_rect(&self, i: usize) -> Rect;
    fn sh_row_control_rect(&self, i: usize) -> Rect;
    fn sh_film_rect(&self) -> Option<Rect>;
    fn sh_scroll_extent(&self) -> (f32, f32);
    fn sh_header_version_text(&self) -> String;
    fn sh_header_name_text(&self) -> String;
    fn sh_logo_label(&self) -> String;
    fn sh_unsaved_marker_visible(&self) -> bool;
    fn sh_save_row_visible(&self) -> bool;
    fn sh_set_theme(&self, theme: Theme);
    fn sh_press_save(&self);
    fn sh_press_cancel(&self);
    fn sh_send_key(&self, text: &str, ctrl: bool, shift: bool);
    fn sh_is_visible(&self) -> bool;
    fn sh_set_update_view(&self, version: &str, state: UpdateState, service_running: bool);
}

macro_rules! shell {
    ($t:ty) => {
        impl Shell for $t {
            fn sh_set_window_size(&self, w: f32, h: f32) {
                <$t>::set_window_size(self, w, h)
            }
            fn sh_window_size(&self) -> (f32, f32) {
                <$t>::window_size(self)
            }
            fn sh_min_window_size(&self) -> (f32, f32) {
                <$t>::min_window_size(self)
            }
            fn sh_page(&self) -> usize {
                <$t>::page(self)
            }
            fn sh_select_page(&self, i: usize) {
                <$t>::select_page(self, i)
            }
            fn sh_rail_labels(&self) -> Vec<String> {
                <$t>::rail_labels(self)
            }
            fn sh_rail_focus(&self) -> usize {
                <$t>::rail_focus(self)
            }
            fn sh_focus_rail(&self) {
                <$t>::focus_rail(self)
            }
            fn sh_page_rows(&self) -> Vec<String> {
                <$t>::page_rows(self)
            }
            fn sh_visible_labels(&self) -> Vec<String> {
                <$t>::visible_labels(self)
            }
            fn sh_visible_texts(&self) -> Vec<String> {
                <$t>::visible_texts(self)
            }
            fn sh_unlabelled_controls(&self) -> usize {
                <$t>::unlabelled_controls(self)
            }
            fn sh_header_rect(&self) -> Rect {
                <$t>::header_rect(self)
            }
            fn sh_logo_rect(&self) -> Rect {
                <$t>::logo_rect(self)
            }
            fn sh_name_rect(&self) -> Rect {
                <$t>::name_rect(self)
            }
            fn sh_version_rect(&self) -> Rect {
                <$t>::version_rect(self)
            }
            fn sh_rail_rect(&self) -> Rect {
                <$t>::rail_rect(self)
            }
            fn sh_rail_item_rect(&self, i: usize) -> Rect {
                <$t>::rail_item_rect(self, i)
            }
            fn sh_page_area_rect(&self) -> Rect {
                <$t>::page_area_rect(self)
            }
            fn sh_content_rect(&self) -> Rect {
                <$t>::content_rect(self)
            }
            fn sh_footer_rect(&self) -> Rect {
                <$t>::footer_rect(self)
            }
            fn sh_row_rect(&self, i: usize) -> Rect {
                <$t>::row_rect(self, i)
            }
            fn sh_row_label_rect(&self, i: usize) -> Rect {
                <$t>::row_label_rect(self, i)
            }
            fn sh_row_control_rect(&self, i: usize) -> Rect {
                <$t>::row_control_rect(self, i)
            }
            fn sh_film_rect(&self) -> Option<Rect> {
                <$t>::film_rect(self)
            }
            fn sh_scroll_extent(&self) -> (f32, f32) {
                <$t>::scroll_extent(self)
            }
            fn sh_header_version_text(&self) -> String {
                <$t>::header_version_text(self)
            }
            fn sh_header_name_text(&self) -> String {
                <$t>::header_name_text(self)
            }
            fn sh_logo_label(&self) -> String {
                <$t>::logo_label(self)
            }
            fn sh_unsaved_marker_visible(&self) -> bool {
                <$t>::unsaved_marker_visible(self)
            }
            fn sh_save_row_visible(&self) -> bool {
                <$t>::save_row_visible(self)
            }
            fn sh_set_theme(&self, theme: Theme) {
                <$t>::set_theme(self, theme)
            }
            fn sh_press_save(&self) {
                <$t>::press_save(self)
            }
            fn sh_press_cancel(&self) {
                <$t>::press_cancel(self)
            }
            fn sh_send_key(&self, text: &str, ctrl: bool, shift: bool) {
                <$t>::send_key(self, text, ctrl, shift)
            }
            fn sh_is_visible(&self) -> bool {
                <$t>::is_visible(self)
            }
            fn sh_set_update_view(&self, version: &str, state: UpdateState, service_running: bool) {
                <$t>::set_update_view(self, version, state, service_running)
            }
        }
    };
}

shell!(SettingsWindow);
shell!(kawaro_ui::welcome::WelcomeWindow);

fn settings(lang: Lang) -> SettingsWindow {
    let w = SettingsWindow::new(lang).expect("window");
    w.open(&Config::default()).expect("open");
    w
}

fn welcome(lang: Lang) -> kawaro_ui::welcome::WelcomeWindow {
    let w = kawaro_ui::welcome::WelcomeWindow::new(lang).expect("window");
    let captions = Captions {
        steps: std::array::from_fn(|i| format!("step {i}")),
    };
    w.open(&captions, true).expect("open");
    w
}

fn key(k: Key) -> String {
    char::from(k).to_string()
}

const SIZES: [(f32, f32); 5] = [
    (560.0, 560.0),
    (700.0, 640.0),
    (880.0, 640.0),
    (1920.0, 1080.0),
    (3840.0, 2160.0),
];

// ------------------------------------------------------------ AC1

#[test]
fn ac1_header_shows_logo_name_and_the_package_version_on_every_page_in_both_languages() {
    let _g = init();
    for lang in [Lang::En, Lang::Ar] {
        let w = settings(lang);
        let first = w.sh_header_rect();
        for p in 0..w.sh_rail_labels().len() {
            w.sh_select_page(p);
            let h = w.sh_header_rect();
            assert_eq!(h, first, "{lang:?} page {p}: header moved");
            assert_eq!(h.y, 0.0);
            assert_eq!(h.h, 64.0, "header-height token");
            let (ww, _) = w.sh_window_size();
            assert!(h.w >= ww - 1.0, "header spans the window");
            assert_eq!(
                w.sh_header_version_text(),
                VERSION,
                "character for character"
            );
            assert_eq!(w.sh_header_name_text(), NAME);
            assert!(!w.sh_logo_label().trim().is_empty(), "logo has a label");
            let logo = w.sh_logo_rect();
            assert_eq!((logo.w, logo.h), (40.0, 40.0), "header-logo token");
            for r in [logo, w.sh_name_rect(), w.sh_version_rect()] {
                assert!(r.y >= h.y && bottom(r) <= bottom(h), "inside the header");
            }
        }
    }
}

// ------------------------------------------------------------ AC2

fn contains_ci(rows: &[String], needle: &str) -> bool {
    rows.iter().any(|r| r.to_lowercase().contains(needle))
}

#[test]
fn ac2_the_rail_has_five_items_in_order_and_each_page_shows_only_its_rows() {
    let _g = init();
    let w = settings(Lang::En);
    let labels = w.sh_rail_labels();
    assert_eq!(labels.len(), 5, "{labels:?}");
    for (l, want) in
        labels
            .iter()
            .zip(["general", "conversion", "notifications", "updates", "about"])
    {
        assert!(l.to_lowercase().contains(want), "{l:?} should name {want}");
    }
    assert!(
        labels[2].to_lowercase().contains("integration"),
        "Notifications and integration: {}",
        labels[2]
    );

    let mut pages: Vec<Vec<String>> = Vec::new();
    for p in 0..5 {
        w.sh_select_page(p);
        // S-099 ruling: the Updates page scrolls at 880x640, so its rows are
        // brought into view by a tall window before they are read.
        if p == 3 {
            w.sh_set_window_size(880.0, 1600.0);
        }
        assert_eq!(w.sh_page(), p);
        pages.push(w.sh_page_rows());
    }
    // General: language, theme.
    assert_eq!(pages[0].len(), 2, "{:?}", pages[0]);
    assert!(contains_ci(&pages[0], "language") && contains_ci(&pages[0], "theme"));
    // Conversion: where results go, folder name, quality.
    assert_eq!(pages[1].len(), 3, "{:?}", pages[1]);
    assert!(contains_ci(&pages[1], "folder") && contains_ci(&pages[1], "quality"));
    // Notifications and integration: the notification list (S-109), open
    // folder, drop shortcut row.
    assert_eq!(pages[2].len(), 3, "{:?}", pages[2]);
    assert!(contains_ci(&pages[2], "notif"));
    assert!(contains_ci(&pages[2], "open"));
    assert!(contains_ci(&pages[2], "shortcut"));
    // Updates: the S-090 section, at least its three settings.
    assert!(pages[3].len() >= 3, "{:?}", pages[3]);
    assert!(contains_ci(&pages[3], "update") || contains_ci(&pages[3], "channel"));

    // Selecting an item shows only its rows: the labels on screen contain the
    // rows of this page and none of another page.
    for p in 0..5 {
        w.sh_select_page(p);
        let on_screen = w.sh_visible_labels();
        for row in &pages[p] {
            assert!(
                on_screen.contains(row),
                "page {p}: row {row:?} missing in {on_screen:?}"
            );
        }
        for (q, rows) in pages.iter().enumerate().filter(|(q, _)| *q != p) {
            for row in rows {
                assert!(
                    !on_screen.contains(row),
                    "page {p} shows page {q}'s row {row:?}"
                );
            }
        }
    }
}

#[test]
fn ac2_about_shows_name_version_licence_description_and_the_exact_privacy_statement() {
    let _g = init();
    assert_eq!(env!("CARGO_PKG_LICENSE"), "GPL-3.0-or-later");
    assert_eq!(kawaro_ui::privacy_statement(Lang::En), PRIVACY_EN);
    let ar_privacy = kawaro_ui::privacy_statement(Lang::Ar);
    assert!(has_arabic(&ar_privacy) && !ar_privacy.contains('\u{2014}'));

    for (lang, privacy) in [
        (Lang::En, PRIVACY_EN.to_string()),
        (Lang::Ar, ar_privacy.clone()),
    ] {
        let w = settings(lang);
        w.sh_select_page(4);
        let texts = w.sh_visible_texts();
        let has = |s: &str| texts.iter().any(|t| t.contains(s));
        assert!(has(NAME) || has("Kawaro"), "{lang:?}: name in {texts:?}");
        assert!(has(VERSION), "{lang:?}: version");
        assert!(has("GPL-3.0-or-later"), "{lang:?}: licence");
        assert!(
            texts.iter().any(|t| t == &privacy),
            "{lang:?}: exact privacy string in {texts:?}"
        );
        // One line description besides name, version, licence, privacy.
        let other = texts
            .iter()
            .filter(|t| {
                let t = t.as_str();
                !t.is_empty()
                    && !t.contains(VERSION)
                    && !t.contains("GPL-3.0-or-later")
                    && t != privacy
                    && !t.contains("Kawaro")
            })
            .count();
        assert!(other >= 1, "{lang:?}: a description line is missing");
    }
}

// ------------------------------------------------------------ AC3

#[test]
fn ac3_content_is_centred_and_at_most_640_wide_at_1920() {
    let _g = init();
    for lang in [Lang::En, Lang::Ar] {
        let w = settings(lang);
        w.sh_set_window_size(1920.0, 1080.0);
        for p in 0..5 {
            w.sh_select_page(p);
            let (area, col) = (w.sh_page_area_rect(), w.sh_content_rect());
            assert!(col.w <= 640.0 + 0.5, "{lang:?} page {p}: {}", col.w);
            assert!(col.w > 0.0);
            let left = col.x - area.x;
            let rightm = right(area) - right(col);
            assert!(
                (left - rightm).abs() <= 1.0,
                "{lang:?} page {p}: margins {left} vs {rightm}"
            );
        }
    }
}

// ------------------------------------------------------------ AC4

fn h_gap(a: Rect, b: Rect) -> f32 {
    (a.x.max(b.x) - right(a).min(right(b))).max(0.0)
}

fn v_overlap(a: Rect, b: Rect) -> bool {
    a.y < bottom(b) && b.y < bottom(a)
}

#[test]
fn ac4_rows_are_label_plus_control_pairs_that_never_drift_apart() {
    let _g = init();
    for lang in [Lang::En, Lang::Ar] {
        let w = settings(lang);
        for (ww, hh) in SIZES {
            w.sh_set_window_size(ww, hh);
            for p in 0..5 {
                w.sh_select_page(p);
                // S-099 ruling: the Updates page scrolls, so a tall window
                // brings its rows into view.
                if p == 3 {
                    w.sh_set_window_size(ww, 2000.0);
                }
                let content = w.sh_content_rect();
                let rows = w.sh_page_rows();
                if p == 4 {
                    continue; // About holds statements, not setting rows.
                }
                assert!(!rows.is_empty());
                for i in 0..rows.len() {
                    let (l, c) = (w.sh_row_label_rect(i), w.sh_row_control_rect(i));
                    let ctx = format!("{lang:?} {ww}x{hh} page {p} row {i}");
                    assert!(h_gap(l, c) <= 640.0, "{ctx}: gap {}", h_gap(l, c));
                    if content.w >= 520.0 {
                        assert!(v_overlap(l, c), "{ctx}: one row at wide content");
                        assert!((l.w - 220.0).abs() <= 1.0, "{ctx}: label column {}", l.w);
                        // The control comes after the label in reading order.
                        match lang {
                            Lang::En => assert!(c.x >= right(l) - 0.5, "{ctx}"),
                            Lang::Ar => assert!(right(c) <= l.x + 0.5, "{ctx}"),
                        }
                    } else {
                        assert!(c.y > l.y, "{ctx}: control below label");
                        assert!(c.y >= bottom(l) - 0.5, "{ctx}: stacked, not overlapping");
                    }
                }
            }
        }
    }
}

#[test]
fn ac4_the_breakpoint_is_crossed_by_resizing() {
    let _g = init();
    let w = settings(Lang::En);
    w.sh_select_page(0);
    w.sh_set_window_size(560.0, 560.0);
    assert!(
        w.sh_content_rect().w < 520.0,
        "560 px window is a narrow content"
    );
    assert!(w.sh_row_control_rect(0).y > w.sh_row_label_rect(0).y);
    w.sh_set_window_size(880.0, 640.0);
    assert!(w.sh_content_rect().w >= 520.0);
    assert!(v_overlap(w.sh_row_label_rect(0), w.sh_row_control_rect(0)));
}

// ------------------------------------------------------------ AC5

#[test]
fn ac5_default_and_minimum_window_sizes() {
    let _g = init();
    for lang in [Lang::En, Lang::Ar] {
        let w = settings(lang);
        assert_eq!(w.sh_window_size(), (880.0, 640.0));
        assert_eq!(w.sh_min_window_size(), (560.0, 560.0));
    }
}

#[test]
fn ac5_no_page_scrolls_at_the_default_size_in_both_languages() {
    let _g = init();
    for lang in [Lang::En, Lang::Ar] {
        let w = settings(lang);
        w.sh_set_window_size(880.0, 640.0);
        // S-099 ruling: the Updates page (3) may scroll; the other four may not.
        for p in [0, 1, 2, 4] {
            w.sh_select_page(p);
            let (content, visible) = w.sh_scroll_extent();
            assert!(visible > 0.0);
            assert!(
                content - visible <= 0.0,
                "{lang:?} page {p}: scrolls by {} px",
                content - visible
            );
        }
    }
}

#[test]
fn ac5_the_rail_shows_all_items_unclipped_at_the_minimum_size() {
    let _g = init();
    for lang in [Lang::En, Lang::Ar] {
        let w = settings(lang);
        w.sh_set_window_size(560.0, 560.0);
        let rail = w.sh_rail_rect();
        assert_eq!(w.sh_rail_labels().len(), 5);
        for i in 0..5 {
            let it = w.sh_rail_item_rect(i);
            assert!(it.w > 0.0 && it.h > 0.0);
            assert!(
                it.x >= rail.x - 0.5
                    && it.y >= rail.y - 0.5
                    && right(it) <= right(rail) + 0.5
                    && bottom(it) <= bottom(rail) + 0.5,
                "{lang:?} item {i} {it:?} outside the rail {rail:?}"
            );
        }
        assert_eq!(rail.w, 96.0, "rail-width token");
    }
}

// ------------------------------------------------------------ AC6

#[test]
fn ac6_save_and_cancel_stay_visible_at_the_minimum_size_on_every_page() {
    let _g = init();
    for lang in [Lang::En, Lang::Ar] {
        let w = settings(lang);
        w.sh_set_window_size(560.0, 560.0);
        let (ww, wh) = w.sh_window_size();
        for p in 0..5 {
            w.sh_select_page(p);
            assert!(w.sh_save_row_visible(), "{lang:?} page {p}");
            let f = w.sh_footer_rect();
            assert!(f.h > 0.0);
            assert!(f.x >= -0.5 && right(f) <= ww + 0.5, "{lang:?} page {p}");
            assert!(f.y >= 0.0 && bottom(f) <= wh + 0.5, "{lang:?} page {p}");
            let area = w.sh_page_area_rect();
            assert!(f.y >= bottom(area) - 0.5, "footer is below the scroll area");
        }
    }
}

#[test]
fn ac6_unsaved_marker_follows_the_difference_from_the_loaded_values() {
    let _g = init();
    for lang in [Lang::En, Lang::Ar] {
        let w = settings(lang);
        assert!(!w.sh_unsaved_marker_visible(), "{lang:?}: clean at open");
        w.sh_set_theme(Theme::Dark);
        assert!(w.sh_unsaved_marker_visible(), "{lang:?}: changed");
        let shown = w.sh_visible_texts();
        assert!(
            shown.iter().any(|t| {
                let l = t.to_lowercase();
                if lang == Lang::En {
                    l.contains("unsaved")
                } else {
                    has_arabic(t)
                }
            }),
            "{lang:?}: marker text in {shown:?}"
        );
        w.sh_set_theme(Theme::Auto);
        assert!(!w.sh_unsaved_marker_visible(), "{lang:?}: reverted");
        w.sh_set_theme(Theme::Light);
        assert!(w.sh_unsaved_marker_visible());
        w.sh_press_save();
        assert!(
            !w.sh_unsaved_marker_visible(),
            "{lang:?}: hidden after Save"
        );
    }
}

#[test]
fn ac6_the_marker_is_visible_on_the_page_where_the_change_was_not_made() {
    let _g = init();
    let w = settings(Lang::En);
    w.sh_set_theme(Theme::Dark);
    for p in 0..5 {
        w.sh_select_page(p);
        assert!(w.sh_unsaved_marker_visible(), "page {p}");
    }
}

#[test]
fn ac6_the_selected_page_is_remembered_in_memory_and_first_open_is_general() {
    let _g = init();
    kawaro_ui::reset_page_memory();
    {
        let w = settings(Lang::En);
        assert_eq!(w.sh_page(), 0, "first ever open is General");
        w.sh_select_page(2);
    }
    let again = settings(Lang::Ar);
    assert_eq!(again.sh_page(), 2, "a new window opens on the last page");
    again.sh_select_page(4);
    drop(again);
    assert_eq!(settings(Lang::En).page(), 4);
    kawaro_ui::reset_page_memory();
    assert_eq!(settings(Lang::En).page(), 0);
}

#[test]
fn ac6_the_page_is_never_written_to_config_toml() {
    let _g = init();
    let w = settings(Lang::En);
    w.sh_select_page(3);
    let saved: Rc<std::cell::RefCell<Option<Config>>> = Rc::default();
    let s2 = saved.clone();
    w.on_save(move |c| *s2.borrow_mut() = Some(c));
    w.sh_press_save();
    let cfg = saved.borrow().clone().expect("saved");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    cfg.save_to(&path).unwrap();
    let text = std::fs::read_to_string(&path).unwrap().to_lowercase();
    assert!(!text.contains("page"), "no page key in:\n{text}");
    assert!(!text.contains("tab"), "no tab key in:\n{text}");
    let ours = Config::default();
    let ref_path = dir.path().join("default.toml");
    ours.save_to(&ref_path).unwrap();
    assert_eq!(
        text,
        std::fs::read_to_string(&ref_path).unwrap().to_lowercase()
    );
}

#[test]
fn ac6_starting_a_check_moves_no_row_and_the_last_row_is_never_cut_off() {
    let _g = init();
    for lang in [Lang::En, Lang::Ar] {
        let w = settings(lang);
        w.sh_set_window_size(880.0, 640.0);
        w.sh_select_page(3);
        // S-099 ruling: a tall window brings the scrolled rows into view.
        w.sh_set_window_size(880.0, 1400.0);
        w.sh_set_update_view(VERSION, UpdateState::UpToDate, true);
        let n = w.sh_page_rows().len();
        let before: Vec<Rect> = (0..n).map(|i| w.sh_row_rect(i)).collect();
        for st in [
            UpdateState::Checking,
            UpdateState::Downloading(7),
            UpdateState::Ready,
            UpdateState::Failed("offline".into()),
            UpdateState::Failed("x ".repeat(120)),
            UpdateState::Unavailable,
        ] {
            w.sh_set_update_view(VERSION, st.clone(), true);
            for (i, was) in before.iter().enumerate() {
                let now = w.sh_row_rect(i);
                assert_eq!(now.y, was.y, "{lang:?} {st:?}: row {i} moved");
            }
            let area = w.sh_page_area_rect();
            let last = w.sh_row_rect(n - 1);
            assert!(
                bottom(last) <= bottom(area) + 0.5,
                "{lang:?} {st:?}: last row cut off"
            );
        }
    }
}

#[test]
fn ac6_press_update_check_does_not_move_rows_either() {
    let _g = init();
    let w = settings(Lang::En);
    w.sh_select_page(3);
    // S-099 ruling: a tall window brings the scrolled rows into view.
    w.sh_set_window_size(880.0, 1400.0);
    w.sh_set_update_view(VERSION, UpdateState::UpToDate, true);
    let n = w.sh_page_rows().len();
    let before: Vec<f32> = (0..n).map(|i| w.sh_row_rect(i).y).collect();
    let clicks = Rc::new(Cell::new(0));
    let c2 = clicks.clone();
    w.on_update_check(move || c2.set(c2.get() + 1));
    w.press_update_check();
    assert_eq!(clicks.get(), 1);
    let after: Vec<f32> = (0..n).map(|i| w.sh_row_rect(i).y).collect();
    assert_eq!(before, after);
}

// ------------------------------------------------------------ AC7

#[test]
fn ac7_arabic_mirrors_rail_header_and_rows() {
    let _g = init();
    let en = settings(Lang::En);
    let ar = settings(Lang::Ar);
    for w in [&en, &ar] {
        w.sh_set_window_size(1920.0, 1080.0);
        w.sh_select_page(0);
    }
    // English: rail on the left, header logo, name, version left to right.
    assert!(en.sh_rail_rect().x < en.sh_content_rect().x);
    assert!(en.sh_logo_rect().x < en.sh_name_rect().x);
    assert!(en.sh_name_rect().x < en.sh_version_rect().x);
    // Arabic: rail on the right, header order mirrored.
    assert!(ar.sh_rail_rect().x > ar.sh_content_rect().x);
    assert!(ar.sh_logo_rect().x > ar.sh_name_rect().x);
    assert!(ar.sh_name_rect().x > ar.sh_version_rect().x);
    // Rows: label on the reading side, control after it.
    assert!(en.sh_row_label_rect(0).x < en.sh_row_control_rect(0).x);
    assert!(ar.sh_row_label_rect(0).x > ar.sh_row_control_rect(0).x);
}

#[test]
fn ac7_every_control_rail_item_and_the_logo_has_a_label_in_both_languages() {
    let _g = init();
    for lang in [Lang::En, Lang::Ar] {
        let w = settings(lang);
        assert!(w.sh_rail_labels().iter().all(|l| !l.trim().is_empty()));
        assert!(!w.sh_logo_label().trim().is_empty());
        for p in 0..5 {
            w.sh_select_page(p);
            assert_eq!(w.sh_unlabelled_controls(), 0, "{lang:?} page {p}");
            assert!(w.sh_visible_labels().iter().all(|l| !l.trim().is_empty()));
        }
        if lang == Lang::Ar {
            assert!(
                w.sh_rail_labels().iter().all(|l| has_arabic(l)),
                "{:?}",
                w.sh_rail_labels()
            );
        }
    }
}

#[test]
fn ac7_ctrl_tab_walks_the_pages_and_wraps_after_five_presses() {
    let _g = init();
    for lang in [Lang::En, Lang::Ar] {
        kawaro_ui::reset_page_memory();
        let w = settings(lang);
        let mut seen = vec![w.sh_page()];
        for _ in 0..5 {
            w.sh_send_key(&key(Key::Tab), true, false);
            seen.push(w.sh_page());
        }
        assert_eq!(seen, [0, 1, 2, 3, 4, 0], "{lang:?}");
        w.sh_send_key(&key(Key::Tab), true, true);
        assert_eq!(w.sh_page(), 4, "{lang:?}: Ctrl+Shift+Tab wraps backwards");
        w.sh_send_key(&key(Key::Tab), true, true);
        assert_eq!(w.sh_page(), 3);
    }
}

#[test]
fn ac7_up_and_down_move_inside_the_focused_rail() {
    let _g = init();
    kawaro_ui::reset_page_memory();
    let w = settings(Lang::En);
    w.sh_focus_rail();
    let start = w.sh_rail_focus();
    w.sh_send_key(&key(Key::DownArrow), false, false);
    assert_eq!(w.sh_rail_focus(), start + 1);
    w.sh_send_key(&key(Key::UpArrow), false, false);
    assert_eq!(w.sh_rail_focus(), start);
}

#[test]
fn ac7_escape_closes_without_saving() {
    let _g = init();
    let w = settings(Lang::En);
    let saved = Rc::new(Cell::new(false));
    let s2 = saved.clone();
    w.on_save(move |_| s2.set(true));
    w.sh_set_theme(Theme::Dark);
    assert!(w.sh_is_visible());
    w.sh_send_key(&key(Key::Escape), false, false);
    assert!(!w.sh_is_visible(), "Esc closes the window");
    assert!(!saved.get(), "and saves nothing");
}

// ------------------------------------------------------------ AC8

#[test]
fn ac8_welcome_has_introduction_first_and_the_settings_window_never_does() {
    let _g = init();
    for lang in [Lang::En, Lang::Ar] {
        kawaro_ui::reset_page_memory();
        let wel = welcome(lang);
        let set = settings(lang);
        let (wl, sl) = (wel.sh_rail_labels(), set.sh_rail_labels());
        assert_eq!(wl.len(), 6, "{wl:?}");
        assert_eq!(sl.len(), 5, "{sl:?}");
        assert!(
            wl[0].to_lowercase().contains("intro") || (lang == Lang::Ar && has_arabic(&wl[0])),
            "{lang:?}: {}",
            wl[0]
        );
        assert_eq!(&wl[1..], &sl[..], "{lang:?}: same items after Introduction");
        assert!(!sl.contains(&wl[0]), "settings never shows Introduction");
        assert_eq!(wel.sh_page(), 0, "the first run opens on Introduction");
    }
}

#[test]
fn ac8_both_windows_expose_the_same_rows_for_the_same_page() {
    let _g = init();
    for lang in [Lang::En, Lang::Ar] {
        let wel = welcome(lang);
        let set = settings(lang);
        for p in 0..5 {
            set.sh_select_page(p);
            wel.sh_select_page(p + 1);
            assert_eq!(wel.sh_page_rows(), set.sh_page_rows(), "{lang:?} page {p}");
            assert_eq!(wel.sh_unlabelled_controls(), 0);
        }
    }
}

#[test]
fn ac8_the_old_top_tab_buttons_are_gone_and_cancel_returns_to_introduction() {
    let _g = init();
    let wel = welcome(Lang::En);
    assert_eq!(wel.sh_page(), 0);
    assert!(
        !wel.sh_save_row_visible(),
        "the Introduction has no Save row"
    );
    wel.sh_select_page(2);
    assert!(wel.sh_save_row_visible(), "settings pages keep Save");
    wel.sh_press_cancel();
    assert_eq!(wel.sh_page(), 0, "Cancel returns to Introduction");
    assert!(wel.sh_is_visible(), "and the welcome window stays open");
    assert!(!wel.sh_save_row_visible());
}

#[test]
fn ac8_ctrl_tab_wraps_over_six_pages_in_the_welcome_window() {
    let _g = init();
    let wel = welcome(Lang::En);
    let mut seen = vec![wel.sh_page()];
    for _ in 0..6 {
        wel.sh_send_key(&key(Key::Tab), true, false);
        seen.push(wel.sh_page());
    }
    assert_eq!(seen, [0, 1, 2, 3, 4, 5, 0]);
}

// ------------------------------------------------------------ AC9

#[test]
fn ac9_the_film_card_is_bounded_16_9_and_inside_the_window() {
    let _g = init();
    for lang in [Lang::En, Lang::Ar] {
        let wel = welcome(lang);
        for (ww, hh) in [(880.0, 640.0), (1920.0, 1080.0), (3840.0, 2160.0)] {
            wel.sh_set_window_size(ww, hh);
            wel.sh_select_page(0);
            let film = wel.sh_film_rect().expect("Introduction shows the film");
            let ctx = format!("{lang:?} {ww}x{hh}: {film:?}");
            assert!(film.x >= 0.0 && film.y >= 0.0, "{ctx}");
            assert!(right(film) <= ww && bottom(film) <= hh, "{ctx}: overflow");
            assert!(film.w <= 560.0 + 0.5, "{ctx}: film-max-width");
            assert!(
                (film.w * 9.0 - film.h * 16.0).abs() <= 16.0,
                "{ctx}: aspect 16:9"
            );
            let area = wel.sh_page_area_rect();
            assert!(
                film.h <= area.h + 0.5,
                "{ctx}: taller than the visible area"
            );
            assert!(
                film.y >= area.y - 0.5 && bottom(film) <= bottom(area) + 0.5,
                "{ctx}"
            );
            // Centred in the content column.
            let col = wel.sh_content_rect();
            let mid = (film.x + right(film)) / 2.0 - (col.x + right(col)) / 2.0;
            assert!(mid.abs() <= 1.0, "{ctx}: not centred ({mid})");
        }
    }
}

#[test]
fn ac9_no_film_on_a_settings_page() {
    let _g = init();
    let wel = welcome(Lang::En);
    wel.sh_select_page(1);
    assert!(wel.sh_film_rect().is_none(), "no film on a settings page");
}
