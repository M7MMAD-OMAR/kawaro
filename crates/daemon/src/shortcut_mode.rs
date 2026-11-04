// The shortcut handler of S-135, pure so it is tested without a compositor:
// in drop-first mode a press reads the clipboard and opens the normal hive
// for its files; an empty or unreadable clipboard falls back to the hold gate
// with exactly one diagnostic line. Choose-during-drag keeps the hold gate.
//
// Included by crates/daemon/tests/s135_shortcut.rs, so this file uses std and
// kawaro_core only.

use kawaro_core::config::DropMode;
use std::path::PathBuf;

/// The event the global shortcut delivers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShortcutEvent {
    Activated,
    Deactivated,
}

/// What the daemon does with one shortcut event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shortcut {
    /// Open the normal hive for the clipboard files, no drop and no hold.
    ClipboardHive,
    /// Run the existing long press gate.
    HoldShow,
    /// Close a hold hive; a clipboard hive stays as it is.
    CloseHold,
}

/// The action for one event in the mode, with the clipboard result.
pub fn shortcut_action(
    mode: DropMode,
    event: ShortcutEvent,
    clipboard: Result<Vec<PathBuf>, String>,
) -> Shortcut {
    if event == ShortcutEvent::Deactivated {
        return Shortcut::CloseHold;
    }
    if mode == DropMode::DropFirst && clipboard.is_ok_and(|files| !files.is_empty()) {
        Shortcut::ClipboardHive
    } else {
        Shortcut::HoldShow
    }
}

/// The one diagnostic line of a fallback, None when the clipboard had files.
pub fn shortcut_log(mode: DropMode, clipboard: &Result<Vec<PathBuf>, String>) -> Option<String> {
    if mode == DropMode::ChooseDuringDrag {
        return None;
    }
    match clipboard {
        Ok(files) if !files.is_empty() => None,
        Ok(_) => Some(clipboard_log("the clipboard holds no files")),
        Err(reason) => Some(clipboard_log(reason)),
    }
}

/// The one clipboard line a fallback writes.
pub fn clipboard_log(reason: &str) -> String {
    format!("clipboard: {reason}")
}
