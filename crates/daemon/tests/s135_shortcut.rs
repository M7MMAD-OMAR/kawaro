//! S-135 AC5: the shortcut handler with a fake clipboard.
//! Contract pinned here (tester): crates/daemon/src/shortcut_mode.rs with
//! ShortcutEvent { Activated, Deactivated }, Shortcut { ClipboardHive,
//! HoldShow, CloseHold }, shortcut_action(mode, event, clipboard) and
//! shortcut_log(mode, clipboard) returning at most one "clipboard: <reason>"
//! line, plus clipboard_log(reason).

#[allow(dead_code)]
mod shortcut_mode {
    include!("../src/shortcut_mode.rs");
}

use kawaro_core::config::DropMode;
use shortcut_mode::{Shortcut, ShortcutEvent, clipboard_log, shortcut_action, shortcut_log};
use std::path::PathBuf;

fn files() -> Vec<PathBuf> {
    vec![PathBuf::from("/home/u/a.png")]
}

#[test]
fn ac5_clipboard_files_open_the_normal_hive_without_a_drop() {
    let action = shortcut_action(DropMode::DropFirst, ShortcutEvent::Activated, Ok(files()));
    assert_eq!(action, Shortcut::ClipboardHive);
    assert_eq!(
        shortcut_log(DropMode::DropFirst, &Ok(files())),
        None,
        "files need no fallback line"
    );
}

#[test]
fn ac5_an_empty_or_unreadable_clipboard_falls_back_to_hold_with_one_line() {
    for clipboard in [
        Ok(Vec::new()),
        Err("the clipboard holds no files".to_string()),
    ] {
        let action = shortcut_action(
            DropMode::DropFirst,
            ShortcutEvent::Activated,
            clipboard.clone(),
        );
        assert_eq!(action, Shortcut::HoldShow);
        let line = shortcut_log(DropMode::DropFirst, &clipboard).expect("one fallback line");
        assert!(line.starts_with("clipboard: "), "{line}");
        assert_eq!(line.lines().count(), 1);
    }
}

#[test]
fn ac5_choose_during_drag_keeps_the_hold_shortcut() {
    let action = shortcut_action(
        DropMode::ChooseDuringDrag,
        ShortcutEvent::Activated,
        Ok(files()),
    );
    assert_eq!(action, Shortcut::HoldShow);
    assert_eq!(shortcut_log(DropMode::ChooseDuringDrag, &Ok(files())), None);
}

#[test]
fn ac5_deactivated_closes_only_a_hold_hive() {
    assert_eq!(
        shortcut_action(DropMode::DropFirst, ShortcutEvent::Deactivated, Ok(files())),
        Shortcut::CloseHold
    );
    assert_eq!(
        shortcut_action(
            DropMode::DropFirst,
            ShortcutEvent::Deactivated,
            Ok(Vec::new())
        ),
        Shortcut::CloseHold
    );
}

#[test]
fn ac5_one_clipboard_line_format() {
    assert_eq!(
        clipboard_log("the clipboard holds no files"),
        "clipboard: the clipboard holds no files"
    );
}
