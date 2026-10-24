//! The registry under `HKEY_LOCAL_MACHINE` (S-043b AC9). Windows only, so
//! `.cargo/mutants.toml` leaves this file out; the lookup order that uses it
//! is mutated in `locate.rs`.

use super::Registry;
use sbarhive_core::win::wide;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{ERROR_SUCCESS, FILETIME};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_LOCAL_MACHINE, KEY_READ, RRF_RT_REG_SZ, RegCloseKey, RegEnumKeyExW, RegGetValueW,
    RegOpenKeyExW,
};

/// The live registry.
pub(super) struct SystemRegistry;

/// The text before the first nul.
fn until_nul(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}

impl Registry for SystemRegistry {
    fn value(&self, key: &str, name: &str) -> Option<String> {
        let key = wide(key);
        let name = wide(name);
        let mut bytes: u32 = 0;
        // SAFETY: both strings are nul terminated and outlive the call; a
        // null data pointer asks only for the size, written to `bytes`.
        let status = unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                key.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_SZ,
                null_mut(),
                null_mut(),
                &mut bytes,
            )
        };
        if status != ERROR_SUCCESS {
            return None;
        }
        let mut buf = vec![0u16; (bytes as usize).div_ceil(2)];
        let mut bytes = (buf.len() * 2) as u32;
        // SAFETY: `buf` holds `bytes` bytes and outlives the call.
        let status = unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                key.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_SZ,
                null_mut(),
                buf.as_mut_ptr().cast(),
                &mut bytes,
            )
        };
        (status == ERROR_SUCCESS).then(|| until_nul(&buf))
    }

    fn subkeys(&self, key: &str) -> Vec<String> {
        let key = wide(key);
        let mut hkey: HKEY = null_mut();
        // SAFETY: `key` is nul terminated; on success `hkey` is an open key
        // that is closed below.
        if unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, key.as_ptr(), 0, KEY_READ, &mut hkey) }
            != ERROR_SUCCESS
        {
            return Vec::new();
        }
        let mut names = Vec::new();
        // A key name has at most 255 characters.
        let mut buf = [0u16; 256];
        for index in 0.. {
            let mut len = buf.len() as u32;
            let mut written = FILETIME {
                dwLowDateTime: 0,
                dwHighDateTime: 0,
            };
            // SAFETY: `hkey` is open, `buf` holds `len` characters, and the
            // optional class outputs are null.
            let status = unsafe {
                RegEnumKeyExW(
                    hkey,
                    index,
                    buf.as_mut_ptr(),
                    &mut len,
                    null(),
                    null_mut(),
                    null_mut(),
                    &mut written,
                )
            };
            if status != ERROR_SUCCESS {
                break;
            }
            names.push(String::from_utf16_lossy(&buf[..len as usize]));
        }
        // SAFETY: `hkey` was opened above and is not used afterwards.
        unsafe {
            RegCloseKey(hkey);
        }
        names
    }
}
