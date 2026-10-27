//! The global shortcut string, for example `Ctrl+Alt+H` (S-048 AC1): parsed
//! into the modifier mask and virtual key code that `RegisterHotKey` takes.
//! Pure, so it is tested on every platform; the Windows shell uses it.

/// `MOD_ALT` of `RegisterHotKey`.
pub const MOD_ALT: u32 = 1;
/// `MOD_CONTROL` of `RegisterHotKey`.
pub const MOD_CONTROL: u32 = 2;
/// `MOD_SHIFT` of `RegisterHotKey`.
pub const MOD_SHIFT: u32 = 4;
/// `MOD_WIN` of `RegisterHotKey`.
pub const MOD_WIN: u32 = 8;

// The values above are Win32's; the build for Windows checks them.
#[cfg(windows)]
#[rustfmt::skip]
mod pinned {
    use super::*;
    const _: () = assert!(MOD_ALT == windows_sys::Win32::UI::Input::KeyboardAndMouse::MOD_ALT);
    const _: () = assert!(MOD_CONTROL == windows_sys::Win32::UI::Input::KeyboardAndMouse::MOD_CONTROL);
    const _: () = assert!(MOD_SHIFT == windows_sys::Win32::UI::Input::KeyboardAndMouse::MOD_SHIFT);
    const _: () = assert!(MOD_WIN == windows_sys::Win32::UI::Input::KeyboardAndMouse::MOD_WIN);
}

/// A parsed shortcut: `mods` is an OR of the `MOD_*` values, `vk` a Windows
/// virtual key code.
#[derive(Debug, PartialEq)]
pub struct Hotkey {
    pub mods: u32,
    pub vk: u32,
}

/// Parses `Mod+Mod+Key`, case-insensitive. Modifiers are `Ctrl`, `Alt`,
/// `Shift` and `Win`; the key is `A` to `Z`, `0` to `9` or `F1` to `F24`.
/// At least one modifier is required.
pub fn parse(s: &str) -> Result<Hotkey, String> {
    let (key, mods_part) = match s.rsplit_once('+') {
        Some((m, k)) => (k, m),
        None => {
            return Err(format!(
                "`{s}` has no modifier (use for example Ctrl+Alt+H)"
            ));
        }
    };
    let mut mods = 0;
    for m in mods_part.split('+') {
        mods |= match m.to_ascii_lowercase().as_str() {
            "alt" => MOD_ALT,
            "ctrl" => MOD_CONTROL,
            "shift" => MOD_SHIFT,
            "win" => MOD_WIN,
            _ => return Err(format!("`{m}` is not a modifier (Ctrl, Alt, Shift, Win)")),
        };
    }
    let vk =
        key_code(key).ok_or_else(|| format!("`{key}` is not a key (A to Z, 0 to 9, F1 to F24)"))?;
    Ok(Hotkey { mods, vk })
}

pub(crate) fn key_code(key: &str) -> Option<u32> {
    let upper = key.to_ascii_uppercase();
    let mut chars = upper.chars();
    match (chars.next()?, chars.as_str()) {
        (c @ ('A'..='Z' | '0'..='9'), "") => Some(u32::from(c)),
        ('F', n) => match n.parse::<u32>() {
            Ok(n @ 1..=24) => Some(0x6F + n),
            _ => None,
        },
        _ => None,
    }
}
