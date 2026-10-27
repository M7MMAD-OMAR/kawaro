//! The Windows known folders (S-041 AC5, ADR 0005 section 4). The choice
//! between the variable and the known folder is pure and tested in
//! `paths.rs`; this file only asks the system, so it is left out of the
//! mutants run (`.cargo/mutants.toml`) and checked live in S-052.

use super::{Os, log_file_with};
use crate::win::from_wide_ptr;
use std::path::PathBuf;
use std::ptr::null_mut;
use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::UI::Shell::{
    FOLDERID_LocalAppData, FOLDERID_Programs, FOLDERID_RoamingAppData, KF_FLAG_DEFAULT,
    SHGetKnownFolderPath,
};
use windows_sys::core::{GUID, PWSTR};

/// The path of one known folder, or `None` when the system has none.
fn known_folder(id: &GUID) -> Option<PathBuf> {
    let mut raw: PWSTR = null_mut();
    // SAFETY: `id` is a valid GUID, a null token means the current user,
    // and `raw` receives a buffer the system allocates.
    let hr = unsafe { SHGetKnownFolderPath(id, KF_FLAG_DEFAULT as u32, null_mut(), &mut raw) };
    let path = if hr >= 0 && !raw.is_null() {
        // SAFETY: on success `raw` is a NUL terminated wide string.
        Some(PathBuf::from(unsafe { from_wide_ptr(raw) }))
    } else {
        None
    };
    // SAFETY: the buffer is freed on every path, success or failure, as the
    // documentation of SHGetKnownFolderPath asks; a null pointer is allowed.
    unsafe { CoTaskMemFree(raw.cast()) };
    path.filter(|p| !p.as_os_str().is_empty())
}

/// `FOLDERID_RoamingAppData`, the fallback for `%APPDATA%`.
pub(super) fn roaming_app_data() -> Option<PathBuf> {
    known_folder(&FOLDERID_RoamingAppData)
}

/// `FOLDERID_LocalAppData`, the fallback for `%LOCALAPPDATA%`.
fn local_app_data() -> Option<PathBuf> {
    known_folder(&FOLDERID_LocalAppData)
}

/// `FOLDERID_Programs`, the user's Start Menu Programs folder, where the
/// per-user MSI puts its shortcut (S-085).
pub(crate) fn start_menu_programs() -> Option<PathBuf> {
    known_folder(&FOLDERID_Programs)
}

/// The service log file, `%LOCALAPPDATA%\Kawaro\logs\kawarod.log`.
pub fn log_file() -> Option<PathBuf> {
    log_file_with(Os::Windows, &|k| std::env::var_os(k), &local_app_data)
}
