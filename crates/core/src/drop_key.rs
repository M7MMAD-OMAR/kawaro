//! The drag key of the settings picker (S-076) with the S-084 Windows rules
//! (backlog/S-084-windows-drag-trigger.md, manager decisions D1, D3 and D6):
//! the per-platform default of a missing `drop_key`, the read-time
//! re-resolution of a stale Alt, the capture refusals and their bilingual
//! texts. Pure, so it is tested on Linux; the S-076b picker consumes exactly
//! this logic and never invents its own.

use crate::i18n::Lang;

/// What opens the hive while a document is dragged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DropTrigger {
    /// The Windows default of the S-084 manager decision D3: the
    /// right-button drag, the secondary button drag whose own menu is the
    /// shell folder's "Copy here / Move here / Create shortcuts here"
    /// behavior. It holds no Ctrl, Shift or Alt while the drag is in
    /// progress, so it never changes what Explorer does with the drag.
    RightDrag,
    /// A held key or chord (`drop_key`), the S-019e long press path.
    Key(String),
}

/// The default of a missing `drop_key` where Alt was measured, and the
/// re-resolution rule of D1: on Windows a missing key and a stored Alt both
/// resolve to [`DropTrigger::RightDrag`], so a stale value written before
/// S-084 never resurrects the bug. The read is pure and writes nothing.
pub fn resolve(stored: Option<&str>, windows: bool) -> DropTrigger {
    let stored = stored.map(str::trim).filter(|s| !s.is_empty());
    match stored {
        None => default(windows),
        Some(s) if windows && s.eq_ignore_ascii_case("alt") => DropTrigger::RightDrag,
        Some(s) => DropTrigger::Key(s.to_string()),
    }
}

/// The measured default of each platform (D3): the right-button drag on
/// Windows, Alt where it was measured.
pub fn default(windows: bool) -> DropTrigger {
    if windows {
        DropTrigger::RightDrag
    } else {
        DropTrigger::Key("Alt".into())
    }
}

/// The preset options the S-076 picker offers (D6 and D7): the measured
/// default of the platform is the only preset; a captured chord is an option
/// only when [`check_capture`] accepts it, and no option holds Ctrl, Shift or
/// Alt while the drag is in progress (AC2a).
pub fn preset_options(windows: bool) -> Vec<DropTrigger> {
    vec![default(windows)]
}

/// Why the picker refuses a captured key (D1 and S-076 decision 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureRefusal {
    /// A plain key with no modifier, alone or in a chord.
    PlainKey,
    /// A key that would be held during the drag and is an Explorer drag
    /// modifier on Windows: Alt copies, Shift moves, Ctrl copies and Alt
    /// with any of them creates a shortcut, so the drag badge flips and a
    /// drop outside the hive changes meaning.
    DragModifier,
    /// Empty, unknown or malformed input.
    Unsupported,
}

/// Checks a captured chord and returns it normalised (modifier names as
/// `Alt`, `Ctrl`, `Shift`, `Super`, keys upper case, the order kept).
///
/// S-076 decision 5 accepts a modifier alone or a combination with at least
/// one modifier, and refuses a plain key alone and unsupported input on
/// every platform. D1 adds the Windows rules: bare Alt, Ctrl or Shift and
/// every chord holding one of them is refused, because the whole chord is
/// held during the drag and each of those changes what Explorer does with
/// it (AC2a: the invariant covers every shipped trigger).
pub fn check_capture(chord: &str, windows: bool) -> Result<String, CaptureRefusal> {
    let tokens: Vec<&str> = chord.split('+').map(str::trim).collect();
    if tokens.iter().any(|t| t.is_empty()) {
        return Err(CaptureRefusal::Unsupported);
    }
    let mut modifiers = 0;
    let mut keys = 0;
    for t in &tokens {
        match t.to_ascii_lowercase().as_str() {
            "alt" => modifiers |= MOD_ALT,
            "ctrl" => modifiers |= MOD_CTRL,
            "shift" => modifiers |= MOD_SHIFT,
            "super" | "win" => modifiers |= MOD_SUPER,
            _ if is_key(t) => keys += 1,
            _ => return Err(CaptureRefusal::Unsupported),
        }
    }
    if modifiers == 0 {
        return Err(CaptureRefusal::PlainKey);
    }
    if keys > 1 {
        return Err(CaptureRefusal::Unsupported);
    }
    if windows && modifiers & DRAG_MODIFIERS != 0 {
        return Err(CaptureRefusal::DragModifier);
    }
    Ok(tokens
        .iter()
        .map(|t| normalise(t))
        .collect::<Vec<_>>()
        .join("+"))
}

/// The refusal text in both languages (D1: the Windows refusals name the
/// Windows reason).
pub fn refusal_text(reason: CaptureRefusal, lang: Lang) -> String {
    match (reason, lang) {
        (CaptureRefusal::PlainKey, Lang::En) => {
            "Use one key with a modifier, for example Super+Z.".to_string()
        }
        (CaptureRefusal::PlainKey, Lang::Ar) => {
            "استخدم مفتاحاً واحداً مع مُعدِّل، مثل Super+Z.".to_string()
        }
        (CaptureRefusal::DragModifier, Lang::En) => {
            "Alt, Ctrl and Shift change what Explorer does with a drag on \
             Windows, so they cannot be the drag key."
                .to_string()
        }
        (CaptureRefusal::DragModifier, Lang::Ar) => {
            "في ويندوز تُغيّر مفاتيح Alt وCtrl وShift ما يفعله Explorer أثناء \
             السحب، فلا يمكن استخدامها مفتاحاً للسحب."
                .to_string()
        }
        (CaptureRefusal::Unsupported, Lang::En) => "That key cannot be the drag key.".to_string(),
        (CaptureRefusal::Unsupported, Lang::Ar) => {
            "لا يمكن استخدام هذا المفتاح مفتاحاً للسحب.".to_string()
        }
    }
}

/// The modifier tokens of a captured chord, as internal bits.
const MOD_ALT: u32 = 1;
const MOD_CTRL: u32 = 2;
const MOD_SHIFT: u32 = 4;
const MOD_SUPER: u32 = 8;
/// The Explorer drag modifiers (the S-084 research): each changes what
/// Explorer does with a drag while it is held.
const DRAG_MODIFIERS: u32 = MOD_ALT | MOD_CTRL | MOD_SHIFT;

/// One key token: `A` to `Z`, `0` to `9` or `F1` to `F24`, like
/// [`crate::hotkey::parse`] accepts.
fn is_key(token: &str) -> bool {
    crate::hotkey::key_code(token).is_some()
}

/// One token in the canonical spelling of [`check_capture`].
fn normalise(token: &str) -> String {
    match token.to_ascii_lowercase().as_str() {
        "alt" => "Alt".to_string(),
        "ctrl" => "Ctrl".to_string(),
        "shift" => "Shift".to_string(),
        "super" | "win" => "Super".to_string(),
        _ => token.to_ascii_uppercase(),
    }
}
