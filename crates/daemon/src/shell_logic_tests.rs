//! S-048 AC2, AC3, AC4 and AC7 (backlog/S-048-windows-tray-shortcut.md): the
//! pure logic of the Windows shell thread.
//!
//! Contract (tester, from the acceptance criteria; do not edit). Module
//! `crate::shell_logic` (`mod shell_logic;` in main.rs, with
//! `#[cfg_attr(windows, allow(dead_code))]`-style gating as needed):
//! - `tooltip(active: &[(String, f32)], lang: Lang) -> String`. Idle gives
//!   `text::idle(lang)`. Otherwise one line per job, `"{label}: {pct}%"`,
//!   joined by `"\n"` (no trailing newline); `pct` is the fraction times 100
//!   rounded to nearest, clamped to 0..=100. If the text is more than 127
//!   UTF-16 units it is cut to its first 126 units, backed off to a scalar
//!   boundary (never inside a surrogate pair), and `"…"` is appended.
//! - `#[derive(Debug, PartialEq, Clone)] pub enum Command { CancelAll,
//!   Settings, Quit }` and `#[derive(Debug, PartialEq, Clone)] pub enum Item
//!   { Action { command: Command, label: String, enabled: bool }, Separator }`.
//! - `menu(lang: Lang, active_count: usize, update_ready: bool) -> Vec<Item>`;
//!   `menu_right_to_left(lang) -> bool`. CHANGED BY S-054b: `Command` gains
//!   `UpdateNow` and `menu` gains a first item `UpdateNow` (label
//!   `text::update_now(lang)`: "Update now" / «حدّث الآن»), enabled only when
//!   `update_ready`, then the four S-048 entries unchanged. The Windows menu
//!   has no other way to apply an update in `ask` mode.
//! - `pub const POLL_MS: u64 = 16;` `#[derive(Default)] pub struct HoldPoll`
//!   with `press(&mut self)` and `tick(&mut self, key_down: bool) -> Poll`,
//!   `#[derive(Debug, PartialEq, Clone, Copy)] pub enum Poll { Held, Released, Idle }`.
//!   A tick before any press gives `Idle`; a new `press()` re-arms it.
//! - `shortcut_error_line(s: &str, ok: bool) -> Option<String>`.

use crate::shell_logic::{
    Command, HoldPoll, Item, POLL_MS, Poll, menu, menu_right_to_left, shortcut_error_line, tooltip,
};
use crate::text;
use sbarhive_core::i18n::Lang;

fn jobs(v: &[(&str, f32)]) -> Vec<(String, f32)> {
    v.iter().map(|(a, b)| (a.to_string(), *b)).collect()
}

fn units(s: &str) -> usize {
    s.encode_utf16().count()
}

// AC2

#[test]
fn ac2_idle_is_the_idle_text_in_both_languages() {
    assert_eq!(tooltip(&[], Lang::En), text::idle(Lang::En));
    assert_eq!(tooltip(&[], Lang::Ar), text::idle(Lang::Ar));
    assert_ne!(text::idle(Lang::En), text::idle(Lang::Ar));
}

#[test]
fn ac2_two_jobs_give_two_lines() {
    let t = tooltip(&jobs(&[("a.png", 0.5), ("b.pdf", 0.25)]), Lang::En);
    assert_eq!(t, "a.png: 50%\nb.pdf: 25%");
    let t = tooltip(&jobs(&[("a.png", 0.5), ("b.pdf", 0.25)]), Lang::Ar);
    assert_eq!(t, "a.png: 50%\nb.pdf: 25%");
}

#[test]
fn ac2_one_job_has_no_newline() {
    assert_eq!(tooltip(&jobs(&[("a.png", 0.0)]), Lang::En), "a.png: 0%");
    assert_eq!(tooltip(&jobs(&[("a.png", 1.0)]), Lang::En), "a.png: 100%");
}

#[test]
fn ac2_percent_is_rounded_and_clamped() {
    assert_eq!(tooltip(&jobs(&[("a", 0.126)]), Lang::En), "a: 13%");
    assert_eq!(tooltip(&jobs(&[("a", 0.124)]), Lang::En), "a: 12%");
    assert_eq!(tooltip(&jobs(&[("a", 1.5)]), Lang::En), "a: 100%");
    assert_eq!(tooltip(&jobs(&[("a", -0.2)]), Lang::En), "a: 0%");
}

#[test]
fn ac2_exactly_127_units_is_kept_whole() {
    // ": 50%" is 5 units, so a 122 character label fills 127.
    let label = "x".repeat(122);
    let t = tooltip(&jobs(&[(&label, 0.5)]), Lang::En);
    assert_eq!(units(&t), 127);
    assert_eq!(t, format!("{label}: 50%"));
    assert!(!t.ends_with('…'));
}

#[test]
fn ac2_128_units_is_cut_to_126_plus_the_ellipsis() {
    let label = "x".repeat(123);
    let t = tooltip(&jobs(&[(&label, 0.5)]), Lang::En);
    let full = format!("{label}: 50%");
    assert_eq!(units(&full), 128);
    assert_eq!(units(&t), 127);
    assert_eq!(t, format!("{}…", &full[..126]));
}

#[test]
fn ac2_twenty_long_jobs_end_with_the_ellipsis_within_127_units() {
    let label = "y".repeat(30);
    let list: Vec<(String, f32)> = (0..20).map(|_| (label.clone(), 0.5)).collect();
    let t = tooltip(&list, Lang::En);
    assert!(units(&t) <= 127);
    assert!(t.ends_with('…'));
    assert!(t.starts_with(&format!("{label}: 50%\n{label}: 50%\n")));
    let t = tooltip(&list, Lang::Ar);
    assert!(units(&t) <= 127);
    assert!(t.ends_with('…'));
}

#[test]
fn ac2_arabic_labels_count_utf16_units_not_bytes() {
    // 100 Arabic letters are 100 units (200 bytes); the line is 106 units.
    let label = "ص".repeat(100);
    let t = tooltip(&jobs(&[(&label, 0.5)]), Lang::Ar);
    assert_eq!(t, format!("{label}: 50%"));
    let label = "ص".repeat(200);
    let t = tooltip(&jobs(&[(&label, 0.5)]), Lang::Ar);
    assert_eq!(units(&t), 127);
    assert_eq!(t, format!("{}…", "ص".repeat(126)));
}

#[test]
fn ac2_a_surrogate_pair_is_never_split_at_the_cut() {
    // The astral character fills units 125 and 126: a cut at 126 would keep
    // only its high half, so the whole character goes.
    let label = format!("{}😀{}", "a".repeat(125), "b".repeat(10));
    let t = tooltip(&jobs(&[(&label, 0.5)]), Lang::En);
    assert_eq!(t, format!("{}…", "a".repeat(125)));
    assert_eq!(units(&t), 126);
    // One unit earlier it fits whole (units 124 and 125).
    let label = format!("{}😀{}", "a".repeat(124), "b".repeat(10));
    let t = tooltip(&jobs(&[(&label, 0.5)]), Lang::En);
    assert_eq!(t, format!("{}😀…", "a".repeat(124)));
    assert_eq!(units(&t), 127);
}

// AC3

fn action(command: Command, label: &str, enabled: bool) -> Item {
    Item::Action {
        command,
        label: label.to_string(),
        enabled,
    }
}

#[test]
fn ac3_menu_order_and_labels_in_english_with_jobs() {
    assert_eq!(
        menu(Lang::En, 2, false),
        vec![
            action(Command::UpdateNow, "Update now", false),
            action(Command::CancelAll, text::cancel_all(Lang::En), true),
            action(Command::Settings, text::settings(Lang::En), true),
            Item::Separator,
            action(Command::Quit, text::quit(Lang::En), true),
        ]
    );
}

#[test]
fn ac3_cancel_all_is_disabled_without_jobs() {
    let m = menu(Lang::En, 0, false);
    assert_eq!(m.len(), 5);
    assert_eq!(m[0], action(Command::UpdateNow, "Update now", false));
    assert_eq!(
        m[1],
        action(Command::CancelAll, text::cancel_all(Lang::En), false)
    );
    assert_eq!(
        m[2],
        action(Command::Settings, text::settings(Lang::En), true)
    );
    assert_eq!(m[3], Item::Separator);
    assert_eq!(m[4], action(Command::Quit, text::quit(Lang::En), true));
}

#[test]
fn ac3_one_job_enables_cancel_all() {
    assert_eq!(
        menu(Lang::En, 1, false)[1],
        action(Command::CancelAll, text::cancel_all(Lang::En), true)
    );
}

#[test]
fn ac3_arabic_labels() {
    assert_eq!(
        menu(Lang::Ar, 3, false),
        vec![
            action(Command::UpdateNow, "حدّث الآن", false),
            action(Command::CancelAll, text::cancel_all(Lang::Ar), true),
            action(Command::Settings, text::settings(Lang::Ar), true),
            Item::Separator,
            action(Command::Quit, text::quit(Lang::Ar), true),
        ]
    );
    assert_ne!(text::quit(Lang::Ar), text::quit(Lang::En));
}

// S-054b: the Windows menu's "Update now".

#[test]
fn s054b_update_now_is_first_and_enabled_only_when_an_update_is_ready() {
    for (lang, label) in [(Lang::En, "Update now"), (Lang::Ar, "حدّث الآن")] {
        for jobs in [0, 2] {
            let off = menu(lang, jobs, false);
            let on = menu(lang, jobs, true);
            assert_eq!(off[0], action(Command::UpdateNow, label, false));
            assert_eq!(on[0], action(Command::UpdateNow, label, true));
            assert_eq!(off.len(), 5);
            // Nothing else depends on the update.
            assert_eq!(off[1..], on[1..]);
        }
    }
    assert_eq!(text::update_now(Lang::En), "Update now");
    assert_eq!(text::update_now(Lang::Ar), "حدّث الآن");
}

#[test]
fn s054b_the_windows_menu_follows_the_linux_label_list() {
    for lang in [Lang::En, Lang::Ar] {
        let state = crate::update::UpdateState::new(crate::update::UpdateEnv {
            installed: String::new(),
            keys: vec![],
            platform: sbarhive_core::update::policy::Platform {
                os: String::new(),
                arch: String::new(),
                pkg: String::new(),
            },
            os: sbarhive_core::paths::Os::Windows,
            updates_dir: Default::default(),
            scratch: Default::default(),
        });
        let labels: Vec<String> = menu(lang, 0, false)
            .into_iter()
            .filter_map(|i| match i {
                Item::Action { label, .. } => Some(label),
                Item::Separator => None,
            })
            .collect();
        assert_eq!(labels, text::menu_labels(lang, &state));
    }
}

#[test]
fn ac3_right_to_left_for_arabic_only() {
    assert!(menu_right_to_left(Lang::Ar));
    assert!(!menu_right_to_left(Lang::En));
}

// AC4

#[test]
fn ac4_poll_interval_is_16_ms() {
    assert_eq!(POLL_MS, 16);
}

#[test]
fn ac4_held_until_the_first_release_then_idle() {
    let mut p = HoldPoll::default();
    p.press();
    assert_eq!(p.tick(true), Poll::Held);
    assert_eq!(p.tick(true), Poll::Held);
    assert_eq!(p.tick(true), Poll::Held);
    assert_eq!(p.tick(false), Poll::Released);
    assert_eq!(p.tick(false), Poll::Idle);
    assert_eq!(p.tick(true), Poll::Idle);
    assert_eq!(p.tick(false), Poll::Idle);
}

#[test]
fn ac4_release_on_the_first_tick_still_releases_once() {
    let mut p = HoldPoll::default();
    p.press();
    assert_eq!(p.tick(false), Poll::Released);
    assert_eq!(p.tick(false), Poll::Idle);
}

#[test]
fn ac4_ticks_before_a_press_are_idle() {
    let mut p = HoldPoll::default();
    assert_eq!(p.tick(true), Poll::Idle);
    assert_eq!(p.tick(false), Poll::Idle);
}

#[test]
fn ac4_a_new_press_rearms_the_poll() {
    let mut p = HoldPoll::default();
    p.press();
    assert_eq!(p.tick(false), Poll::Released);
    p.press();
    assert_eq!(p.tick(true), Poll::Held);
    assert_eq!(p.tick(false), Poll::Released);
}

// AC7

#[test]
fn ac7_error_line_when_the_hotkey_is_taken() {
    assert_eq!(
        shortcut_error_line("Ctrl+Alt+H", false),
        Some("sbarhived: shortcut Ctrl+Alt+H unavailable".to_string())
    );
    assert_eq!(
        shortcut_error_line("shift+f9", false),
        Some("sbarhived: shortcut shift+f9 unavailable".to_string())
    );
}

#[test]
fn ac7_no_line_when_registered() {
    assert_eq!(shortcut_error_line("Ctrl+Alt+H", true), None);
}
