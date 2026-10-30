//! The Windows app identity (S-085): the one idempotent sync that gives the
//! Start shortcut and the notification identity the bilingual name.
//!
//! wixl cannot carry a non-ASCII string, so the MSI installs the ASCII name
//! and this sync applies the bilingual one after install (D1, D3). The daemon
//! runs it at start and `kawaro integration install` runs it after its
//! `reg.exe` calls. The plan and the bytes are pure, so they are tested on
//! any OS; the Windows applier is `identity/windows.rs`.

use std::path::{Path, PathBuf};

#[cfg(windows)]
mod windows;

/// The fixed brand string, the Latin part first in every language (D8).
pub const DISPLAY_NAME: &str = "Kawaro | كوارو";

/// The Start shortcut the MSI installs (ASCII, wixl).
pub const MSI_SHORTCUT_FILE: &str = "Kawaro.lnk";

/// The bilingual Start shortcut. `|` is reserved in Win32 file names, so the
/// file name uses U+FF5C FULLWIDTH VERTICAL LINE instead (D10).
pub const SHORTCUT_FILE: &str = "Kawaro \u{FF5C} كوارو.lnk";

/// The HKCU subkey of the notification identity (S-049, S-050), the key
/// `registry::install_commands` writes under `HKCU\`; it ends with `APP_ID`.
pub const AUMID_KEY: &str = r"Software\Classes\AppUserModelId\io.github.m7mmad_omar.Kawaro";

/// The properties set on both Start shortcuts (S-106 D3), as (property
/// name, value). The AUMID ties the shortcut to the registered app name, so
/// the notification header shows `DISPLAY_NAME` and not the id.
pub fn shortcut_properties() -> &'static [(&'static str, &'static str)] {
    &[("System.AppUserModel.ID", crate::appid::APP_ID)]
}

/// The rename of the MSI shortcut to the bilingual one inside the Start Menu
/// `programs` folder, as (from, to). `None` when there is nothing to do: the
/// MSI shortcut is gone (already renamed, or a portable install) or the
/// bilingual one exists already, which is never overwritten.
pub fn shortcut_rename(
    programs: &Path,
    exists: impl Fn(&Path) -> bool,
) -> Option<(PathBuf, PathBuf)> {
    let from = programs.join(MSI_SHORTCUT_FILE);
    if !exists(&from) {
        return None;
    }
    let to = programs.join(SHORTCUT_FILE);
    if exists(&to) {
        return None;
    }
    Some((from, to))
}

/// The REG_SZ data of `value`: UTF-16LE with one trailing NUL.
pub fn reg_sz(value: &str) -> Vec<u8> {
    value
        .encode_utf16()
        .chain(Some(0))
        .flat_map(u16::to_le_bytes)
        .collect()
}

/// Applies the identity: renames the Start shortcut and writes the AUMID
/// DisplayName. Best effort and idempotent; it never fails the caller.
#[cfg(windows)]
pub use windows::sync;

/// Off Windows there is no Start menu or AUMID to name.
#[cfg(not(windows))]
pub fn sync() {}
