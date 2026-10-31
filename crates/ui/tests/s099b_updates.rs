//! S-099b (backlog/S-099-update-experience.md): messages, versions, polling
//! and the "What is new" cards of the Updates page. Written from the
//! acceptance criteria only (B1 to B4, B8).
//!
//! The surface these tests pin down (the implementer builds it; it is also in
//! the story's "Test hooks (S-099b)" section):
//! - `kawaro_ui::UpdateState` gains `Installing` and `Updated(String)`;
//!   `Downloading(u8)` is unchanged.
//! - `SettingsWindow::set_update_available(version: Option<&str>,
//!   size: Option<u64>, notes: Option<&Notes>)`: the available version, the
//!   total download size in bytes and the `notes` of the reply. Independent
//!   of `set_update_view` (either order).
//! - Hidden hooks: `update_available_text() -> Option<String>`,
//!   `update_cards() -> Vec<(String, Vec<String>)>` (title, bullets, in
//!   display order), `update_card_labels() -> Vec<String>` (accessible label
//!   of each card), `update_card_rect(i) -> Rect`,
//!   `update_card_title_rect(i) -> Rect`, `update_row_ys() -> Vec<f32>` (y of
//!   every row of the Updates page below the cards), `update_banner() ->
//!   Option<String>`, `poll_tick(now_ms: u64)`.
//! - `poll_tick` is what the 250 ms window timer calls with a monotonic
//!   millisecond clock: in Downloading or Installing it fires the
//!   `on_update_check` callback once per 1000 ms (the first tick after
//!   entering the state starts the clock), in every other state or when the
//!   window is closed it does nothing.

use kawaro_core::config::Config;
use kawaro_core::update::manifest::Notes;
use kawaro_core::update::whats_new;
use kawaro_ui::{Lang, Rect, SettingsWindow, UpdateState};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::{Mutex, MutexGuard};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const MIB: u64 = 1024 * 1024;
const UPDATES_PAGE: usize = 3;

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
    w.select_page(UPDATES_PAGE);
    w
}

fn has_arabic(s: &str) -> bool {
    s.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c))
}

fn has_arabic_indic_digit(s: &str) -> bool {
    s.chars()
        .any(|c| ('\u{0660}'..='\u{0669}').contains(&c) || ('\u{06F0}'..='\u{06F9}').contains(&c))
}

fn lines(prefix: &str, n: usize) -> Vec<String> {
    (1..=n).map(|i| format!("{prefix} line {i}")).collect()
}

fn good_notes(n: usize) -> Notes {
    Notes {
        en: lines("English", n),
        ar: (1..=n).map(|i| format!("سطر عربي {i}")).collect(),
    }
}

// ------------------------------------------------------------ B1

#[test]
fn ac_b1_downloading_line_has_percent_and_size_in_both_languages() {
    let _g = init();
    // 24 percent of 50 MiB is 12.0 MB (the story's example string).
    let en = window(Lang::En);
    en.set_update_available(Some("0.4.3"), Some(50 * MIB), None);
    en.set_update_view(VERSION, UpdateState::Downloading(24), true);
    assert_eq!(
        en.update_state_line(),
        "Downloading 0.4.3: 24% (12.0 of 50.0 MB)"
    );
    en.set_update_view(VERSION, UpdateState::Downloading(42), true);
    assert_eq!(
        en.update_state_line(),
        "Downloading 0.4.3: 42% (21.0 of 50.0 MB)"
    );
    let ar = window(Lang::Ar);
    ar.set_update_available(Some("0.4.3"), Some(50 * MIB), None);
    ar.set_update_view(VERSION, UpdateState::Downloading(24), true);
    assert_eq!(
        ar.update_state_line(),
        "جارٍ تنزيل 0.4.3: 24% (12.0 من 50.0 ميغابايت)"
    );
    assert!(!has_arabic_indic_digit(&ar.update_state_line()));
}

#[test]
fn ac_b1_without_a_size_the_parenthesis_is_absent() {
    let _g = init();
    for (lang, want) in [
        (Lang::En, "Downloading 0.4.3: 42%"),
        (Lang::Ar, "جارٍ تنزيل 0.4.3: 42%"),
    ] {
        let w = window(lang);
        w.set_update_available(Some("0.4.3"), None, None);
        w.set_update_view(VERSION, UpdateState::Downloading(42), true);
        assert_eq!(w.update_state_line(), want);
        assert!(!w.update_state_line().contains('('));
    }
}

#[test]
fn ac_b1_ready_installing_and_updated_lines_in_both_languages() {
    let _g = init();
    let cases = [
        (
            UpdateState::Ready,
            "Download finished. Ready to install 0.4.3.",
            "اكتمل التنزيل. التحديث 0.4.3 جاهز للتثبيت.",
        ),
        (
            UpdateState::Installing,
            "Installing 0.4.3. Kawaro will restart by itself.",
            "جارٍ تثبيت 0.4.3. سيعيد كوارو التشغيل تلقائياً.",
        ),
        (
            UpdateState::Updated("0.4.3".into()),
            "Updated to 0.4.3",
            "تم التحديث إلى 0.4.3",
        ),
    ];
    for (state, en_line, ar_line) in cases {
        let en = window(Lang::En);
        en.set_update_available(Some("0.4.3"), Some(50 * MIB), None);
        en.set_update_view(VERSION, state.clone(), true);
        assert_eq!(en.update_state_line(), en_line, "{state:?}");
        let ar = window(Lang::Ar);
        ar.set_update_available(Some("0.4.3"), Some(50 * MIB), None);
        ar.set_update_view(VERSION, state.clone(), true);
        assert_eq!(ar.update_state_line(), ar_line, "{state:?}");
        assert!(!has_arabic_indic_digit(&ar.update_state_line()));
    }
}

#[test]
fn ac_b1_check_and_update_now_are_disabled_while_downloading_or_installing() {
    let _g = init();
    let w = window(Lang::En);
    w.set_update_available(Some("0.4.3"), Some(50 * MIB), None);
    for state in [UpdateState::Downloading(10), UpdateState::Installing] {
        w.set_update_view(VERSION, state.clone(), true);
        assert!(!w.update_check_enabled(), "{state:?}: check");
        assert!(!w.update_apply_enabled(), "{state:?}: apply");
    }
    w.set_update_view(VERSION, UpdateState::Ready, true);
    assert!(w.update_apply_enabled(), "ready still enables Update now");
}

// ------------------------------------------------------------ B2

#[test]
fn ac_b2_installed_line_always_and_available_line_in_three_states_only() {
    let _g = init();
    let all = [
        UpdateState::UpToDate,
        UpdateState::Checking,
        UpdateState::Downloading(5),
        UpdateState::Ready,
        UpdateState::Installing,
        UpdateState::Failed("offline".into()),
        UpdateState::Unavailable,
        UpdateState::Updated("0.4.3".into()),
    ];
    for lang in [Lang::En, Lang::Ar] {
        let w = window(lang);
        w.set_update_available(Some("0.4.3"), Some(50 * MIB), None);
        for state in &all {
            w.set_update_view(VERSION, state.clone(), true);
            let installed = w.update_version_text();
            assert!(installed.contains(VERSION), "{lang:?} {state:?}");
            if lang == Lang::En {
                assert_eq!(installed, format!("Installed version: {VERSION}"));
            } else {
                assert!(has_arabic(&installed), "{installed}");
            }
            let shown = matches!(
                state,
                UpdateState::Downloading(_) | UpdateState::Ready | UpdateState::Installing
            );
            match (w.update_available_text(), shown) {
                (Some(t), true) => {
                    assert!(t.contains("0.4.3"), "{t}");
                    if lang == Lang::En {
                        assert_eq!(t, "Available version: 0.4.3");
                    } else {
                        assert!(has_arabic(&t), "{t}");
                    }
                }
                (None, false) => {}
                (got, _) => panic!("{lang:?} {state:?}: available line {got:?}, want {shown}"),
            }
        }
    }
}

// ------------------------------------------------------------ B3

#[test]
fn ac_b3_three_polls_one_second_apart_while_downloading_and_none_after_ready() {
    let _g = init();
    let w = window(Lang::En);
    let n = Rc::new(Cell::new(0u32));
    let n2 = n.clone();
    w.on_update_check(move || n2.set(n2.get() + 1));
    w.set_update_available(Some("0.4.3"), Some(50 * MIB), None);
    w.set_update_view(VERSION, UpdateState::Downloading(1), true);
    for t in (0..=3000).step_by(250) {
        w.poll_tick(t);
    }
    assert_eq!(n.get(), 3, "polls at 1000, 2000 and 3000 ms");
    // Installing polls too.
    w.set_update_view(VERSION, UpdateState::Installing, true);
    let before = n.get();
    for t in (3000..=5000).step_by(250) {
        w.poll_tick(t);
    }
    assert!(n.get() > before, "installing keeps polling");
    // Ready stops it.
    w.set_update_view(VERSION, UpdateState::Ready, true);
    let before = n.get();
    for t in (5000..=9000).step_by(250) {
        w.poll_tick(t);
    }
    assert_eq!(n.get(), before, "no poll after ready");
}

#[test]
fn ac_b3_no_poll_in_the_other_states() {
    let _g = init();
    let w = window(Lang::En);
    let n = Rc::new(Cell::new(0u32));
    let n2 = n.clone();
    w.on_update_check(move || n2.set(n2.get() + 1));
    for state in [
        UpdateState::UpToDate,
        UpdateState::Failed("x".into()),
        UpdateState::Unavailable,
        UpdateState::Updated("0.4.3".into()),
    ] {
        w.set_update_view(VERSION, state, true);
        for t in (0..=4000).step_by(500) {
            w.poll_tick(t);
        }
    }
    assert_eq!(n.get(), 0);
}

#[test]
fn ac_b3_no_poll_after_the_window_closes() {
    let _g = init();
    let w = window(Lang::En);
    let n = Rc::new(Cell::new(0u32));
    let n2 = n.clone();
    w.on_update_check(move || n2.set(n2.get() + 1));
    w.set_update_view(VERSION, UpdateState::Downloading(1), true);
    w.poll_tick(0);
    w.poll_tick(1000);
    assert_eq!(n.get(), 1);
    w.press_cancel();
    assert!(!w.is_visible());
    for t in (1250..=6000).step_by(250) {
        w.poll_tick(t);
    }
    assert_eq!(n.get(), 1, "a closed window sends nothing");
}

// ------------------------------------------------------------ B4

#[test]
fn ac_b4_offline_the_installed_version_card_shows_the_bundled_bullets() {
    let _g = init();
    for lang in [Lang::En, Lang::Ar] {
        let w = window(lang);
        w.set_update_view(VERSION, UpdateState::Unavailable, true);
        let cards = w.update_cards();
        assert_eq!(cards.len(), 1, "{lang:?}: only the installed card offline");
        let (title, bullets) = &cards[0];
        let want_title = match lang {
            Lang::En => format!("What is new in {VERSION}"),
            Lang::Ar => format!("الجديد في {VERSION}"),
        };
        assert_eq!(title, &want_title);
        assert_eq!(bullets, &whats_new::installed(lang));
        assert!((3..=5).contains(&bullets.len()), "{bullets:?}");
        assert!(bullets.iter().all(|b| b.chars().count() <= 80));
    }
}

#[test]
fn ac_b4_a_known_update_adds_a_second_card_from_the_notes() {
    let _g = init();
    let n = good_notes(4);
    for lang in [Lang::En, Lang::Ar] {
        let w = window(lang);
        w.set_update_available(Some("0.4.3"), Some(50 * MIB), Some(&n));
        w.set_update_view(VERSION, UpdateState::Ready, true);
        let cards = w.update_cards();
        assert_eq!(cards.len(), 2, "{lang:?}");
        let (title, bullets) = &cards[1];
        let want_title = match lang {
            Lang::En => "What is new in 0.4.3",
            Lang::Ar => "الجديد في 0.4.3",
        };
        assert_eq!(title, want_title);
        let want = if lang == Lang::En { &n.en } else { &n.ar };
        assert_eq!(bullets, want);
        assert_eq!(cards[0].1, whats_new::installed(lang));
    }
}

#[test]
fn ac_b4_empty_or_rule_breaking_notes_omit_the_second_card() {
    let _g = init();
    let too_long = "x".repeat(81);
    let bad = [
        Notes::default(),
        good_notes(2),
        good_notes(6),
        Notes {
            en: vec![too_long.clone(), "b".into(), "c".into()],
            ar: vec!["أ".into(), "ب".into(), "ج".into()],
        },
    ];
    let w = window(Lang::En);
    w.set_update_view(VERSION, UpdateState::Ready, true);
    w.set_update_available(Some("0.4.3"), Some(50 * MIB), None);
    assert_eq!(w.update_cards().len(), 1, "no notes");
    for n in &bad {
        w.set_update_available(Some("0.4.3"), Some(50 * MIB), Some(n));
        let cards = w.update_cards();
        assert_eq!(cards.len(), 1, "{n:?}");
        assert!(
            cards
                .iter()
                .all(|(_, b)| b.iter().all(|l| l.chars().count() <= 80)),
            "no text over the A4 limit is drawn"
        );
    }
}

#[test]
fn ac_b4_every_card_has_an_accessible_label() {
    let _g = init();
    for lang in [Lang::En, Lang::Ar] {
        let w = window(lang);
        // S-099 ruling: the Updates page scrolls at 880x640, so the page is
        // made tall enough to bring every card into view before measuring.
        w.set_window_size(880.0, 1400.0);
        w.set_update_available(Some("0.4.3"), Some(50 * MIB), Some(&good_notes(3)));
        w.set_update_view(VERSION, UpdateState::Ready, true);
        let labels = w.update_card_labels();
        assert_eq!(labels.len(), w.update_cards().len());
        assert_eq!(labels.len(), 2);
        for (l, (title, _)) in labels.iter().zip(w.update_cards()) {
            assert!(!l.trim().is_empty());
            assert!(
                l.contains(&title) || l.contains("0.4.3") || l.contains(VERSION),
                "{l}"
            );
        }
        assert_ne!(labels[0], labels[1]);
    }
}

#[test]
fn ac_b4_starting_a_check_moves_no_row_and_card_height_is_reserved() {
    let _g = init();
    let w = window(Lang::En);
    // S-099 ruling: the Updates page scrolls at 880x640, so the page is
    // made tall enough to bring every card into view before measuring.
    w.set_window_size(880.0, 1400.0);
    w.set_update_view(VERSION, UpdateState::UpToDate, true);
    let before = w.update_row_ys();
    assert!(!before.is_empty(), "rows below the cards exist");
    w.press_update_check();
    w.set_update_view(VERSION, UpdateState::Checking, true);
    assert_eq!(w.update_row_ys(), before, "a check moves rows by 0 px");
    w.set_update_view(VERSION, UpdateState::Failed("offline".into()), true);
    assert_eq!(w.update_row_ys(), before, "a failure moves rows by 0 px");
    // The card height is the same for 3 and 5 bullets.
    w.set_update_available(Some("0.4.3"), Some(50 * MIB), Some(&good_notes(3)));
    w.set_update_view(VERSION, UpdateState::Ready, true);
    let (h3, ys3) = (w.update_card_rect(1).h, w.update_row_ys());
    w.set_update_available(Some("0.4.3"), Some(50 * MIB), Some(&good_notes(5)));
    let (h5, ys5) = (w.update_card_rect(1).h, w.update_row_ys());
    assert_eq!(h3, h5, "reserved height of title plus 5 lines");
    assert_eq!(ys3, ys5);
    assert_eq!(w.update_card_rect(0).h, h3, "both cards reserve the same");
}

#[test]
fn ac_b4_cards_need_no_scrollbar_beyond_the_page() {
    let _g = init();
    let w = window(Lang::En);
    // S-099 ruling: the Updates page scrolls at 880x640, so the page is
    // made tall enough to bring every card into view before measuring.
    w.set_window_size(880.0, 1400.0);
    w.set_update_available(Some("0.4.3"), Some(50 * MIB), Some(&good_notes(5)));
    w.set_update_view(VERSION, UpdateState::Ready, true);
    for i in 0..2 {
        let r: Rect = w.update_card_rect(i);
        assert!(r.w > 0.0 && r.h > 0.0, "card {i}: {r:?}");
    }
    let c0 = w.update_card_rect(0);
    let c1 = w.update_card_rect(1);
    assert!(c1.y >= c0.y + c0.h, "cards do not overlap: {c0:?} {c1:?}");
}

// ------------------------------------------------------------ B5 (window side)

#[test]
fn ac_b5_updated_shows_the_banner_with_the_new_notes_and_clears_on_check_or_close() {
    let _g = init();
    for lang in [Lang::En, Lang::Ar] {
        let w = window(lang);
        let line = |v: &str| match lang {
            Lang::En => format!("Updated to {v}"),
            Lang::Ar => format!("تم التحديث إلى {v}"),
        };
        w.set_update_view(VERSION, UpdateState::Updated(VERSION.into()), true);
        assert_eq!(w.update_banner(), Some(line(VERSION)));
        assert_eq!(w.update_state_line(), line(VERSION));
        let cards = w.update_cards();
        assert_eq!(cards[0].1, whats_new::installed(lang), "new version notes");
        // The next check clears it.
        w.press_update_check();
        assert_eq!(w.update_banner(), None, "{lang:?}: check clears the banner");
        // Close clears it too.
        w.set_update_view(VERSION, UpdateState::Updated(VERSION.into()), true);
        assert!(w.update_banner().is_some());
        w.press_cancel();
        w.open(&Config::default()).unwrap();
        assert_eq!(w.update_banner(), None, "{lang:?}: close clears the banner");
    }
}

#[test]
fn ac_b5_banner_is_absent_in_every_other_state() {
    let _g = init();
    let w = window(Lang::En);
    for s in [
        UpdateState::UpToDate,
        UpdateState::Ready,
        UpdateState::Installing,
        UpdateState::Downloading(3),
    ] {
        w.set_update_view(VERSION, s.clone(), true);
        assert_eq!(w.update_banner(), None, "{s:?}");
    }
}

// ------------------------------------------------------------ B8

#[test]
fn ac_b8_new_strings_exist_and_differ_per_language() {
    let _g = init();
    let states = [
        UpdateState::Downloading(42),
        UpdateState::Ready,
        UpdateState::Installing,
        UpdateState::Updated("0.4.3".into()),
    ];
    let en = window(Lang::En);
    let ar = window(Lang::Ar);
    for w in [&en, &ar] {
        w.set_update_available(Some("0.4.3"), Some(50 * MIB), Some(&good_notes(3)));
    }
    for s in states {
        en.set_update_view(VERSION, s.clone(), true);
        ar.set_update_view(VERSION, s.clone(), true);
        let (e, a) = (en.update_state_line(), ar.update_state_line());
        assert!(!e.is_empty() && !a.is_empty());
        assert_ne!(e, a, "{s:?}");
        assert!(has_arabic(&a) && !has_arabic(&e), "{s:?}: {e} / {a}");
        for t in [&e, &a] {
            assert!(!t.contains('\u{2014}') && !t.contains('\u{2013}'));
        }
    }
    let (ce, ca) = (en.update_cards(), ar.update_cards());
    assert_ne!(ce[0].0, ca[0].0);
}

#[test]
fn ac_b8_arabic_cards_mirror_the_english_ones() {
    let _g = init();
    let n = good_notes(3);
    let mut gaps = Vec::new();
    for lang in [Lang::En, Lang::Ar] {
        let w = window(lang);
        w.set_update_available(Some("0.4.3"), Some(50 * MIB), Some(&n));
        w.set_update_view(VERSION, UpdateState::Ready, true);
        let card = w.update_card_rect(0);
        let title = w.update_card_title_rect(0);
        let left = title.x - card.x;
        let right = (card.x + card.w) - (title.x + title.w);
        gaps.push((left, right));
    }
    let (en, ar) = (gaps[0], gaps[1]);
    assert!(en.0 < en.1, "English title hugs the left edge: {en:?}");
    assert!(ar.1 < ar.0, "Arabic title hugs the right edge: {ar:?}");
}

#[test]
fn ac_b8_slint_source_has_the_cards_labels_and_no_long_dashes() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let src = std::fs::read_to_string(root.join("crates/ui/ui/settings.slint")).unwrap()
        + &std::fs::read_to_string(root.join("crates/ui/ui/shell.slint")).unwrap();
    assert!(!src.contains('\u{2014}') && !src.contains('\u{2013}'));
    assert!(
        src.to_lowercase().contains("whats-new") || src.to_lowercase().contains("what-is-new"),
        "the Updates page carries a What is new card"
    );
}
