//! S-101 (backlog/S-101-update-ui-defects.md): the Updates page shows what
//! really happens. Written from the acceptance criteria A2, A3 (config part),
//! A4, B1 (line), B3 (banners) and B4 (probe) only.
//!
//! Surface these tests pin down (the implementer builds it):
//! - `poll_tick(now_ms)` also fires `on_update_check` once per 1000 ms in
//!   `UpdateState::Checking` (S-099b polled only Downloading and Installing).
//!   S-099b's "no poll in other states" test drops Checking from its list.
//! - `kawaro_ui::state_from_status("downloading", Some(v))` gives
//!   `Downloading(0)` style states, so a poll reply moves Checking on.
//! - `SettingsWindow::update_auto_notice() -> String` (empty in ask mode),
//!   `press_update_check()`, `update_auto_notice_rect() -> Rect`,
//!   `update_state_line_rect() -> Rect`, `update_banner_rect() -> Rect`.
//! - `kawaro_ui::UpdateResult { Updated(String), Failed(String) }` and
//!   `SettingsWindow::show_update_result(Option<UpdateResult>)`; the banner
//!   text is `update_banner() -> Option<String>`. The failed text uses the
//!   installed version given to `set_update_view`.
//! - The notice and banners reserve their row height, so rows never move.

use kawaro_core::config::{Config, UpdateMode};
use kawaro_ui::{Lang, SettingsWindow, UpdateResult, UpdateState, state_from_status};
use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Mutex, MutexGuard};

const MIB: u64 = 1024 * 1024;
const OLD: &str = "0.4.2";
/// The bottom of the page area at 880x640 (header 64 plus 504).
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

fn polls(w: &SettingsWindow) -> Rc<Cell<u32>> {
    let n = Rc::new(Cell::new(0u32));
    let n2 = n.clone();
    w.on_update_check(move || n2.set(n2.get() + 1));
    n
}

// ---------------------------------------------------------------- A2

#[test]
fn ac_a2_the_window_polls_every_1000_ms_in_checking_too() {
    let _g = init();
    let w = window(Lang::En);
    let n = polls(&w);
    w.set_update_view(OLD, UpdateState::Checking, true);
    for t in (0..=3000).step_by(250) {
        w.poll_tick(t);
    }
    assert_eq!(n.get(), 3, "polls at 1000, 2000 and 3000 ms while checking");
}

#[test]
fn ac_a2_no_poll_in_ready_uptodate_failed_or_after_close() {
    let _g = init();
    let w = window(Lang::En);
    let n = polls(&w);
    for s in [
        UpdateState::Ready,
        UpdateState::UpToDate,
        UpdateState::Failed(String::new()),
    ] {
        w.set_update_view(OLD, s, true);
        for t in (0..=4000).step_by(250) {
            w.poll_tick(t);
        }
    }
    assert_eq!(n.get(), 0);
    w.set_update_view(OLD, UpdateState::Checking, true);
    w.press_cancel();
    for t in (0..=4000).step_by(250) {
        w.poll_tick(t);
    }
    assert_eq!(n.get(), 0, "closed window sends nothing");
}

#[test]
fn ac_a2_a_downloading_reply_moves_checking_to_downloading() {
    let _g = init();
    assert_eq!(
        state_from_status("downloading", Some("0.4.3")),
        UpdateState::Downloading(0)
    );
}

#[test]
fn ac_a2_the_line_follows_the_percent_and_never_returns_to_checking() {
    let _g = init();
    let w = window(Lang::En);
    w.set_update_available(Some("0.4.3"), Some(50 * MIB), None);
    for (pct, want) in [
        (0u8, "Downloading 0.4.3: 0% (0.0 of 50.0 MB)"),
        (42, "Downloading 0.4.3: 42% (21.0 of 50.0 MB)"),
        (100, "Downloading 0.4.3: 100% (50.0 of 50.0 MB)"),
    ] {
        w.set_update_view(OLD, UpdateState::Downloading(pct), true);
        assert_eq!(w.update_state_line(), want);
        assert_ne!(w.update_state_line(), "Checking for updates.");
    }
    w.set_update_view(OLD, UpdateState::Ready, true);
    assert_eq!(
        w.update_state_line(),
        "Download finished. Ready to install 0.4.3."
    );
}

#[test]
fn ac_a2_a_fake_1000_byte_download_reads_zero_megabytes() {
    let _g = init();
    let w = window(Lang::En);
    w.set_update_available(Some("0.4.3"), Some(1000), None);
    w.set_update_view(OLD, UpdateState::Downloading(0), true);
    assert_eq!(
        w.update_state_line(),
        "Downloading 0.4.3: 0% (0.0 of 0.0 MB)"
    );
}

// ---------------------------------------------------------------- A3 (config)

#[test]
fn ac_a3_saving_ask_or_auto_reads_back_the_same_in_en_and_ar() {
    let _g = init();
    for lang in [Lang::En, Lang::Ar] {
        for mode in [UpdateMode::Ask, UpdateMode::Auto] {
            let w = window(lang);
            let saved: std::rc::Rc<std::cell::RefCell<Option<Config>>> = Default::default();
            let s2 = saved.clone();
            w.on_save(move |c| *s2.borrow_mut() = Some(c));
            w.set_update_mode(mode);
            w.press_save();
            let out = saved.borrow().clone().expect("saved");
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("config.toml");
            out.save_to(&path).unwrap();
            let text = std::fs::read_to_string(&path).unwrap();
            let want = if mode == UpdateMode::Ask {
                "ask"
            } else {
                "auto"
            };
            assert!(
                text.contains(&format!("update_mode = \"{want}\"")),
                "{lang:?} {mode:?}: {text}"
            );
            let (back, _) = Config::load_from(&path);
            assert_eq!(back.update_mode, mode, "{lang:?} {mode:?}");
        }
    }
}

// ---------------------------------------------------------------- A4

const NOTICE_EN: &str =
    "Automatic install is on. A found update installs by itself when no job runs.";
const NOTICE_AR: &str = "التثبيت التلقائي مفعّل. يُثبَّت أي تحديث تلقائياً عند عدم وجود مهام جارية.";

#[test]
fn ac_a4_auto_mode_states_itself_before_any_check_and_ask_is_empty() {
    let _g = init();
    for (lang, want) in [(Lang::En, NOTICE_EN), (Lang::Ar, NOTICE_AR)] {
        let w = window(lang);
        w.set_update_mode(UpdateMode::Ask);
        assert_eq!(w.update_auto_notice(), "");
        w.set_update_mode(UpdateMode::Auto);
        assert_eq!(w.update_auto_notice(), want, "{lang:?}");
    }
}

#[test]
fn ac_a4_starting_a_check_moves_no_row() {
    let _g = init();
    for mode in [UpdateMode::Ask, UpdateMode::Auto] {
        let w = window(Lang::En);
        w.set_window_size(880.0, 1600.0);
        w.set_update_mode(mode);
        w.set_update_view(OLD, UpdateState::UpToDate, true);
        let before = w.update_row_ys();
        w.press_update_check();
        w.set_update_view(OLD, UpdateState::Checking, true);
        assert_eq!(w.update_row_ys(), before, "{mode:?}: rows moved by a check");
    }
}

// ---------------------------------------------------------------- B1

#[test]
fn ac_b1_the_installing_line_is_exact_in_en_and_arabic() {
    let _g = init();
    let w = window(Lang::En);
    w.set_update_available(Some("0.4.3"), Some(50 * MIB), None);
    w.set_update_view(OLD, UpdateState::Installing, true);
    assert_eq!(
        w.update_state_line(),
        "Installing 0.4.3. Kawaro will restart by itself."
    );
    let w = window(Lang::Ar);
    w.set_update_available(Some("0.4.3"), Some(50 * MIB), None);
    w.set_update_view(OLD, UpdateState::Installing, true);
    let line = w.update_state_line();
    assert!(line.contains("0.4.3") && line.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c)));
}

// ---------------------------------------------------------------- B3

#[test]
fn ac_b3_the_updated_and_failed_banners_have_the_exact_text() {
    let _g = init();
    let w = window(Lang::En);
    w.set_update_view(OLD, UpdateState::UpToDate, true);
    w.show_update_result(Some(UpdateResult::Updated("0.4.3".into())));
    assert_eq!(w.update_banner().as_deref(), Some("Updated to 0.4.3"));
    w.show_update_result(Some(UpdateResult::Failed("0.4.3".into())));
    assert_eq!(
        w.update_banner().as_deref(),
        Some("Update to 0.4.3 did not install. Kawaro is still 0.4.2.")
    );
    let w = window(Lang::Ar);
    w.set_update_view(OLD, UpdateState::UpToDate, true);
    w.show_update_result(Some(UpdateResult::Failed("0.4.3".into())));
    assert_eq!(
        w.update_banner().as_deref(),
        Some("لم يُثبَّت التحديث 0.4.3. ما زال كوارو 0.4.2.")
    );
}

#[test]
fn ac_b3_failed_keeps_check_enabled_and_banners_clear_on_check_or_close() {
    let _g = init();
    let w = window(Lang::En);
    w.set_update_view(OLD, UpdateState::UpToDate, true);
    w.show_update_result(Some(UpdateResult::Failed("0.4.3".into())));
    assert!(
        w.update_check_enabled(),
        "check stays enabled after a failure"
    );
    assert!(w.update_version_text().contains(OLD));
    w.press_update_check();
    assert_eq!(w.update_banner(), None, "cleared by the next check");
    w.show_update_result(Some(UpdateResult::Updated("0.4.3".into())));
    w.press_cancel();
    w.open(&Config::default()).expect("reopen");
    assert_eq!(w.update_banner(), None, "cleared by closing the window");
}

// ---------------------------------------------------------------- B4

fn shots() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/screenshots/s101")
}

#[test]
fn ac_b4_every_banner_and_line_starts_above_the_page_bottom_less_100() {
    let _g = init();
    let limit = PAGE_BOTTOM - 100.0;
    for lang in [Lang::En, Lang::Ar] {
        let w = window(lang);
        w.set_update_mode(UpdateMode::Auto);
        let r = w.update_auto_notice_rect();
        assert!(r.h > 0.0 && r.y < limit, "{lang:?} notice {r:?}");

        w.set_update_available(Some("0.4.3"), Some(50 * MIB), None);
        for s in [UpdateState::Downloading(42), UpdateState::Installing] {
            w.set_update_view(OLD, s.clone(), true);
            let r = w.update_state_line_rect();
            assert!(r.h > 0.0 && r.y < limit, "{lang:?} {s:?} {r:?}");
        }
        w.set_update_view(OLD, UpdateState::UpToDate, true);
        for res in [
            UpdateResult::Updated("0.4.3".into()),
            UpdateResult::Failed("0.4.3".into()),
        ] {
            w.show_update_result(Some(res.clone()));
            let r = w.update_banner_rect();
            assert!(r.h > 0.0 && r.y < limit, "{lang:?} {res:?} {r:?}");
        }
    }
}

#[test]
fn ac_b4_the_twenty_screenshots_exist_at_880x640() {
    let dir = shots();
    let mut missing = vec![];
    for lang in ["en", "ar"] {
        for scheme in ["light", "dark"] {
            for state in [
                "auto-notice",
                "downloading",
                "installing",
                "updated",
                "failed",
            ] {
                let f = dir.join(format!("updates-{lang}-{scheme}-{state}.png"));
                match std::fs::read(&f) {
                    Ok(b) if b.len() > 24 && &b[1..4] == b"PNG" => {
                        let w = u32::from_be_bytes(b[16..20].try_into().unwrap());
                        let h = u32::from_be_bytes(b[20..24].try_into().unwrap());
                        if (w, h) != (880, 640) {
                            missing.push(format!("{} is {w}x{h}", f.display()));
                        }
                    }
                    _ => missing.push(format!("{} missing", f.display())),
                }
            }
        }
    }
    assert!(missing.is_empty(), "{missing:#?}");
}
