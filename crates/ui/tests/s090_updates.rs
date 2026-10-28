//! S-090 (backlog/S-090-settings-updates-about.md): the "Updates" section of
//! the settings window. Written from the acceptance criteria only.
//!
//! The public surface these tests pin down (the implementer builds it):
//! - `kawaro_ui::UpdateState` with `UpToDate`, `Checking`, `Downloading(u8)`,
//!   `Ready`, `Failed(String)`, `Unavailable`.
//! - `SettingsWindow::set_update_view(version: &str, state: UpdateState,
//!   service_running: bool)`, `on_update_check(Fn())`, `on_update_apply(Fn())`
//!   and `show_update_deferred()` (the S-054b AC7 answer to Update now).
//! - Hidden test hooks on `SettingsWindow`: `update_version_text()`,
//!   `update_state_line()`, `update_check_enabled()`, `update_apply_enabled()`,
//!   `press_update_check()`, `press_update_apply()`, `press_save()`,
//!   `press_cancel()`, `set_update_check_auto(bool)`,
//!   `set_update_channel(Channel)`, `set_update_mode(UpdateMode)`.
//! - `kawaro_ui::about_line(Lang) -> String`: the one version string shared by
//!   the welcome footer and the tray About item.

use kawaro_core::config::{Config, UpdateMode};
use kawaro_core::update::Channel;
use kawaro_ui::{Lang, SettingsWindow, UpdateState};
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn window(lang: Lang) -> SettingsWindow {
    thread_local! {
        static INIT: std::sync::Once = const { std::sync::Once::new() };
    }
    INIT.with(|once| once.call_once(i_slint_backend_testing::init_no_event_loop));
    let w = SettingsWindow::new(lang).expect("window");
    w.open(&Config::default()).expect("open");
    w
}

fn states() -> Vec<UpdateState> {
    vec![
        UpdateState::UpToDate,
        UpdateState::Checking,
        UpdateState::Downloading(42),
        UpdateState::Ready,
        UpdateState::Failed("offline".into()),
        UpdateState::Unavailable,
    ]
}

fn has_arabic(s: &str) -> bool {
    s.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c))
}

// ------------------------------------------------------------ AC1

#[test]
fn ac1_section_shows_the_build_version_and_six_distinct_states_in_both_languages() {
    for lang in [Lang::En, Lang::Ar] {
        let w = window(lang);
        let mut lines = Vec::new();
        for st in states() {
            w.set_update_view(VERSION, st.clone(), true);
            assert!(
                w.update_version_text().contains(VERSION),
                "{lang:?}: version missing"
            );
            let line = w.update_state_line();
            assert!(!line.trim().is_empty(), "{lang:?} {st:?}: empty state line");
            lines.push(line);
        }
        let mut uniq = lines.clone();
        uniq.sort();
        uniq.dedup();
        assert_eq!(uniq.len(), 6, "{lang:?}: states must read differently");
        assert!(lines[2].contains("42"), "downloading shows the percent");
        assert!(lines[4].contains("offline"), "failed shows the reason");
    }
}

// ------------------------------------------------------------ AC2

#[test]
fn ac2_check_sends_one_request_disables_the_button_and_opens_no_window() {
    let w = window(Lang::En);
    let n = Rc::new(Cell::new(0u32));
    let n2 = n.clone();
    w.on_update_check(move || n2.set(n2.get() + 1));
    w.set_update_view(VERSION, UpdateState::UpToDate, true);
    assert!(w.update_check_enabled());
    w.press_update_check();
    assert_eq!(n.get(), 1, "exactly one update-check request");
    assert!(!w.update_check_enabled(), "disabled while the check runs");
    assert!(w.is_visible(), "the settings window stays the only window");
    // A second click while it runs sends nothing.
    w.press_update_check();
    assert_eq!(n.get(), 1);
    w.set_update_view(VERSION, UpdateState::UpToDate, true);
    assert!(w.update_check_enabled(), "enabled again after the response");
}

// ------------------------------------------------------------ AC3

#[test]
fn ac3_update_now_is_enabled_only_in_ready_and_sends_one_apply() {
    let w = window(Lang::En);
    let n = Rc::new(Cell::new(0u32));
    let n2 = n.clone();
    w.on_update_apply(move || n2.set(n2.get() + 1));
    for st in states() {
        w.set_update_view(VERSION, st.clone(), true);
        assert_eq!(w.update_apply_enabled(), st == UpdateState::Ready, "{st:?}");
        w.press_update_apply();
    }
    assert_eq!(n.get(), 1, "only the Ready state sends update-apply");
}

#[test]
fn ac3_deferred_answer_shows_the_waiting_text_not_an_error() {
    let w = window(Lang::En);
    w.set_update_view(VERSION, UpdateState::Ready, true);
    let ready = w.update_state_line();
    w.show_update_deferred();
    let line = w.update_state_line();
    assert!(!line.trim().is_empty());
    assert_ne!(line, ready);
    assert!(
        !line.to_lowercase().contains("fail") && !line.to_lowercase().contains("error"),
        "deferred is not an error: {line}"
    );
}

// ------------------------------------------------------------ AC4

#[test]
fn ac4_save_writes_the_three_keys_and_cancel_leaves_the_file_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let cfg = Config::default();
    cfg.save_to(&path).unwrap();
    let before = std::fs::read(&path).unwrap();

    let w = window(Lang::En);
    w.open(&cfg).unwrap();
    let saved: Rc<std::cell::RefCell<Option<Config>>> = Rc::default();
    let s2 = saved.clone();
    w.on_save(move |c| *s2.borrow_mut() = Some(c));

    // Cancel after edits: nothing is saved, the file is untouched.
    w.set_update_check_auto(false);
    w.set_update_channel(Channel::Beta);
    w.set_update_mode(UpdateMode::Auto);
    w.press_cancel();
    assert!(saved.borrow().is_none(), "Cancel must not call on_save");
    assert_eq!(std::fs::read(&path).unwrap(), before);

    // Save after the same edits: the three keys land in config.toml.
    w.open(&cfg).unwrap();
    w.set_update_check_auto(false);
    w.set_update_channel(Channel::Beta);
    w.set_update_mode(UpdateMode::Auto);
    w.press_save();
    let out = saved.borrow().clone().expect("Save calls on_save");
    out.save_to(&path).unwrap();
    let (back, _) = Config::load_from(&path);
    assert!(!back.update_check);
    assert_eq!(back.update_channel, Channel::Beta);
    assert_eq!(back.update_mode, UpdateMode::Auto);
    assert!(
        back.update_channel_present,
        "an explicit channel stays explicit"
    );
}

#[test]
fn ac4_an_untouched_default_channel_is_not_made_explicit() {
    let w = window(Lang::En);
    let saved: Rc<std::cell::RefCell<Option<Config>>> = Rc::default();
    let s2 = saved.clone();
    w.on_save(move |c| *s2.borrow_mut() = Some(c));
    w.press_save();
    let out = saved.borrow().clone().expect("saved");
    assert!(
        !out.update_channel_present,
        "update_channel_present unchanged"
    );
    assert_eq!(out.update_channel, Channel::Stable);
}

// ------------------------------------------------------------ AC5

#[test]
fn ac5_without_the_daemon_the_version_and_a_service_off_text_show_and_save_works() {
    let w = window(Lang::En);
    let saved = Rc::new(Cell::new(false));
    let s2 = saved.clone();
    w.on_save(move |_| s2.set(true));
    w.set_update_view(VERSION, UpdateState::Unavailable, true);
    let with_service = w.update_state_line();
    w.set_update_view(VERSION, UpdateState::Unavailable, false);
    assert!(w.update_version_text().contains(VERSION));
    let off = w.update_state_line();
    assert!(!off.trim().is_empty());
    assert_ne!(off, with_service, "the service-off text is its own message");
    assert!(
        off.to_lowercase().contains("service"),
        "names the background service: {off}"
    );
    assert!(!w.update_check_enabled() || !w.update_apply_enabled());
    assert!(!w.update_apply_enabled());
    w.press_save();
    assert!(saved.get(), "Save still works");
}

// ------------------------------------------------------------ AC6

#[test]
fn ac6_one_version_line_for_welcome_footer_and_tray_about() {
    let en = kawaro_ui::about_line(Lang::En);
    let ar = kawaro_ui::about_line(Lang::Ar);
    assert!(en.contains(VERSION) && ar.contains(VERSION));
    assert!(has_arabic(&ar) && !has_arabic(&en));
    let welcome = std::fs::read_to_string(root().join("crates/ui/ui/welcome.slint")).unwrap();
    assert!(
        welcome.to_lowercase().contains("version"),
        "welcome.slint must carry a footer version text"
    );
}

// ------------------------------------------------------------ AC7

#[test]
fn ac7_arabic_and_english_strings_exist_and_differ_for_every_state() {
    let en = window(Lang::En);
    let ar = window(Lang::Ar);
    for st in states() {
        en.set_update_view(VERSION, st.clone(), true);
        ar.set_update_view(VERSION, st.clone(), true);
        let (e, a) = (en.update_state_line(), ar.update_state_line());
        assert!(!e.is_empty() && !a.is_empty(), "{st:?}");
        assert!(has_arabic(&a), "{st:?}: Arabic line has no Arabic: {a}");
        assert!(!has_arabic(&e.replace("offline", "")), "{st:?}: {e}");
    }
}

#[test]
fn ac7_slint_source_has_an_updates_section_with_labelled_buttons_and_no_long_dashes() {
    let src = std::fs::read_to_string(root().join("crates/ui/ui/settings.slint")).unwrap();
    let low = src.to_lowercase();
    assert!(low.contains("update-check") && low.contains("update-apply"));
    assert!(
        low.matches("accessible-label").count() >= 2,
        "interactive elements carry labels"
    );
    assert!(!src.contains('\u{2014}') && !src.contains('\u{2013}'));
}
