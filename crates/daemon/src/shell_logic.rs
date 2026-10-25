//! The pure part of the Windows shell thread (S-048): the tray tooltip, the
//! tray menu, the poll of a held shortcut and the log line for a shortcut
//! that is taken. The Win32 side is `shell.rs` and `tray/windows.rs`.

use crate::text;
use kawaro_core::i18n::Lang;

/// The longest tooltip `NOTIFYICONDATAW` holds, in UTF-16 units.
const TIP_UNITS: usize = 127;

/// One line per running job, `label: percent%`, cut to what the tray tooltip
/// can show: more than 127 UTF-16 units keep the first 126 (never half a
/// surrogate pair) and an ellipsis.
pub fn tooltip(active: &[(String, f32)], lang: Lang) -> String {
    if active.is_empty() {
        return text::idle(lang).to_string();
    }
    let full = active
        .iter()
        .map(|(label, f)| format!("{label}: {}%", (f * 100.0).round().clamp(0.0, 100.0) as u32))
        .collect::<Vec<_>>()
        .join("\n");
    fit_tip(full)
}

/// Cuts a tooltip to what the tray can show: more than 127 UTF-16 units
/// keep the first 126 and an ellipsis.
pub fn fit_tip(full: String) -> String {
    if full.encode_utf16().count() <= TIP_UNITS {
        return full;
    }
    let mut cut = String::new();
    let mut units = 0;
    for c in full.chars() {
        units += c.len_utf16();
        if units > TIP_UNITS - 1 {
            break;
        }
        cut.push(c);
    }
    cut.push('…');
    cut
}

/// What a tray menu item does.
#[derive(Debug, PartialEq, Clone)]
pub enum Command {
    UpdateNow,
    CancelAll,
    Settings,
    Quit,
}

/// A tray menu entry.
#[derive(Debug, PartialEq, Clone)]
pub enum Item {
    Action {
        command: Command,
        label: String,
        enabled: bool,
    },
    Separator,
}

/// The tray menu, as the Linux tray has it: Update now (enabled only when a
/// verified update is ready), Cancel all (disabled with no running job),
/// Settings, a separator, Quit.
pub fn menu(lang: Lang, active_count: usize, update_ready: bool) -> Vec<Item> {
    let action = |command, label: &str, enabled| Item::Action {
        command,
        label: label.to_string(),
        enabled,
    };
    vec![
        action(Command::UpdateNow, text::update_now(lang), update_ready),
        action(Command::CancelAll, text::cancel_all(lang), active_count > 0),
        action(Command::Settings, text::settings(lang), true),
        Item::Separator,
        action(Command::Quit, text::quit(lang), true),
    ]
}

/// Arabic menus open right to left (`MFT_RIGHTORDER`).
pub fn menu_right_to_left(lang: Lang) -> bool {
    lang.rtl()
}

/// How often a held shortcut's key is polled, in milliseconds.
pub const POLL_MS: u64 = 16;

/// What one poll of the held key found.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Poll {
    /// Still down.
    Held,
    /// Went up since the last tick; reported once.
    Released,
    /// Nothing to report: no press, or the release was already reported.
    Idle,
}

/// Follows one press of the shortcut's key until it goes up.
#[derive(Default)]
pub struct HoldPoll {
    armed: bool,
}

impl HoldPoll {
    /// The shortcut was pressed: start following the key.
    pub fn press(&mut self) {
        self.armed = true;
    }

    /// One poll, with whether the key is down now.
    pub fn tick(&mut self, key_down: bool) -> Poll {
        match (self.armed, key_down) {
            (false, _) => Poll::Idle,
            (true, true) => Poll::Held,
            (true, false) => {
                self.armed = false;
                Poll::Released
            }
        }
    }
}

/// The log line for a shortcut `RegisterHotKey` refused (`ok` is false).
pub fn shortcut_error_line(s: &str, ok: bool) -> Option<String> {
    (!ok).then(|| format!("kawarod: shortcut {s} unavailable"))
}
