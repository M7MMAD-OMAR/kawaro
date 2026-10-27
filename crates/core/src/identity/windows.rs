//! The Windows applier of the identity sync (S-085 D3). It only asks the
//! system, so it is left out of the mutants run (`.cargo/mutants.toml`); the
//! plan and the bytes it uses are mutated in `identity.rs`, and the live
//! check is the W01 step of docs/windows-beta-checklist.md.

use super::{AUMID_KEY, DISPLAY_NAME, reg_sz, shortcut_rename};
use crate::win::wide;
use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, REG_SZ, RegSetKeyValueW};

/// Renames the Start shortcut and writes the AUMID DisplayName. A failure is
/// written to stderr (the daemon's log file) and never stops the caller.
pub fn sync() {
    if let Some(programs) = crate::paths::start_menu_programs()
        && let Some((from, to)) = shortcut_rename(&programs, |p| p.exists())
        && let Err(e) = std::fs::rename(&from, &to)
    {
        eprintln!("kawaro: cannot rename {}: {e}", from.display());
    }
    let key = wide(AUMID_KEY);
    let name = wide("DisplayName");
    let data = reg_sz(DISPLAY_NAME);
    // SAFETY: the key and the value name are NUL terminated UTF-16 strings,
    // and `data` is `data.len()` readable bytes that end with a NUL code
    // unit, as REG_SZ asks. All three live for the whole call.
    let status = unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            name.as_ptr(),
            REG_SZ,
            data.as_ptr().cast(),
            data.len() as u32,
        )
    };
    if status != 0 {
        eprintln!("kawaro: cannot write the app name to the registry: error {status}");
    }
}
