//! S-099b rework: the notes of the version that matters are on screen at the
//! default size, the card order follows the state, and a failed update reads
//! as a plain sentence. Written from the manager rulings of 2026-10-29.

use kawaro_core::config::Config;
use kawaro_core::update::manifest::Notes;
use kawaro_ui::{Lang, SettingsWindow, UpdateState, state_from_status};
use std::sync::{Mutex, MutexGuard};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const MIB: u64 = 1024 * 1024;
/// The bottom of the page area at 880x640: header 64 plus 504.
const PAGE_BOTTOM: f32 = 568.0;

static LOCK: Mutex<()> = Mutex::new(());

fn init() -> MutexGuard<'static, ()> {
    thread_local! {
        static INIT: std::sync::Once = const { std::sync::Once::new() };
    }
    INIT.with(|once| once.call_once(i_slint_backend_testing::init_no_event_loop));
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn window(lang: Lang) -> SettingsWindow {
    let w = SettingsWindow::new(lang).expect("window");
    w.open(&Config::default()).expect("open");
    w.select_page(3);
    w.set_window_size(880.0, 640.0);
    w
}

fn notes() -> Notes {
    Notes {
        en: (1..=5).map(|i| format!("English line {i}")).collect(),
        ar: (1..=5).map(|i| format!("سطر عربي {i}")).collect(),
    }
}

#[test]
fn the_notes_that_matter_start_above_the_page_bottom_in_ready_and_updated() {
    let _g = init();
    for lang in [Lang::En, Lang::Ar] {
        let w = window(lang);
        w.set_update_available(Some("0.4.3"), Some(50 * MIB), Some(&notes()));
        w.set_update_view(VERSION, UpdateState::Ready, true);
        let top = w.update_card_rect(0);
        assert!(
            top.h > 0.0 && top.y < PAGE_BOTTOM - 100.0,
            "{lang:?} ready: {top:?}"
        );
        assert!(
            w.update_card_labels()[0].contains("0.4.3"),
            "{lang:?}: the available version's notes come first: {:?}",
            w.update_card_labels()
        );
        let w = window(lang);
        w.set_update_view(VERSION, UpdateState::Updated(VERSION.into()), true);
        let top = w.update_card_rect(0);
        assert!(
            top.h > 0.0 && top.y < PAGE_BOTTOM - 100.0,
            "{lang:?} updated: {top:?}"
        );
        assert!(w.update_card_labels()[0].contains(VERSION));
    }
}

#[test]
fn the_settings_rows_come_after_the_cards() {
    let _g = init();
    let w = window(Lang::En);
    w.set_window_size(880.0, 1600.0);
    w.set_update_available(Some("0.4.3"), Some(50 * MIB), Some(&notes()));
    w.set_update_view(VERSION, UpdateState::Ready, true);
    let last_card = w.update_card_rect(1);
    let rows = w.update_row_ys();
    assert!(!rows.is_empty());
    assert!(
        rows.iter().all(|y| *y >= last_card.y + last_card.h),
        "{rows:?} {last_card:?}"
    );
}

#[test]
fn a_failed_update_reads_as_a_plain_sentence_in_both_languages() {
    let _g = init();
    assert_eq!(
        state_from_status("failed", Some("0.4.3")),
        UpdateState::Failed(String::new())
    );
    assert_eq!(
        state_from_status("rejected", None),
        UpdateState::Failed(String::new())
    );
    for lang in [Lang::En, Lang::Ar] {
        let w = window(lang);
        w.set_update_view(VERSION, state_from_status("failed", Some("0.4.3")), true);
        let line = w.update_state_line();
        assert!(
            !line.contains("failed") && !line.contains("0.4.3"),
            "{line}"
        );
        assert!(!line.contains('('), "{line}");
        let has_arabic = line.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c));
        assert_eq!(has_arabic, lang == Lang::Ar, "{line}");
    }
}
