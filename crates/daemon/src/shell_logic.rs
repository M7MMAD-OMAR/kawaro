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
    GetStarted,
    About,
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
/// verified update is ready), Get started (S-087, always enabled), About
/// (S-099, opens the Updates page), Cancel
/// all (disabled with no running job), Settings, a separator, Quit.
pub fn menu(lang: Lang, active_count: usize, update_ready: bool) -> Vec<Item> {
    let action = |command, label: &str, enabled| Item::Action {
        command,
        label: label.to_string(),
        enabled,
    };
    vec![
        action(Command::UpdateNow, text::update_now(lang), update_ready),
        action(Command::GetStarted, text::get_started(lang), true),
        action(Command::About, text::about(lang), true),
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

/// The virtual key of the primary mouse button: `VK_LBUTTON`, or
/// `VK_RBUTTON` when the user swapped the buttons (`SM_SWAPBUTTON`).
pub fn primary_button_vk(swapped: bool) -> i32 {
    if swapped { 0x02 } else { 0x01 }
}

/// The virtual key of the S-084 drag trigger (the manager decision D3): the
/// secondary button, whose drag shows the shell's own "Copy here / Move here
/// / Create shortcuts here" menu, `VK_RBUTTON`, or `VK_LBUTTON` when the
/// user swapped the buttons (`SM_SWAPBUTTON`). The mapping stays in the
/// caller, as the primary mapping does.
pub fn drag_button_vk(swapped: bool) -> i32 {
    if swapped { 0x01 } else { 0x02 }
}

/// What one poll of the S-084 drag watch found (the manager decision D8).
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum DragPoll {
    /// Nothing to report: no button down, or a drag already reported.
    Idle,
    /// The button went up before the drag threshold: a press, never a drag.
    PressOnly,
    /// The pointer reached the drag threshold with the button held: the drag
    /// has started. Reported once per drag.
    Started,
    /// The button went up after the drag had started: the drag ended.
    Ended,
}

/// Follows the S-084 drag trigger (the manager decisions D3 and D8): the
/// right-button drag. The trigger moment is the drag start, that is the
/// button held and the pointer moved at least the system drag threshold
/// (`SM_CXDRAG`, `SM_CYDRAG`) from the press point; a momentary press never
/// shows the hive, and there is no hold threshold at all: `tick` takes no
/// clock. Pure; the Win32 polling is `shell.rs`.
#[derive(Default)]
pub struct DragWatch {
    /// The press point of the button that is down, and whether its drag has
    /// started.
    press: Option<((i32, i32), bool)>,
}

impl DragWatch {
    /// One poll: the button state, the pointer position in global pixels and
    /// the system drag threshold `(SM_CXDRAG, SM_CYDRAG)`.
    pub fn tick(&mut self, down: bool, at: (i32, i32), threshold: (i32, i32)) -> DragPoll {
        match (self.press, down) {
            (None, false) => DragPoll::Idle,
            (None, true) => {
                self.press = Some((at, false));
                DragPoll::Idle
            }
            (Some(_), false) => {
                let started = self.press.take().is_some_and(|(_, s)| s);
                if started {
                    DragPoll::Ended
                } else {
                    DragPoll::PressOnly
                }
            }
            (Some((from, started)), true) => {
                if started {
                    DragPoll::Idle
                } else if (at.0 - from.0).abs() >= threshold.0
                    || (at.1 - from.1).abs() >= threshold.1
                {
                    self.press = Some((from, true));
                    DragPoll::Started
                } else {
                    DragPoll::Idle
                }
            }
        }
    }
}
