//! The Windows applier of the identity sync (S-085 D3). It only asks the
//! system, so it is left out of the mutants run (`.cargo/mutants.toml`); the
//! plan and the bytes it uses are mutated in `identity.rs`, and the live
//! check is the W01 step of docs/windows-beta-checklist.md.

use super::{
    AUMID_KEY, DISPLAY_NAME, MSI_SHORTCUT_FILE, SHORTCUT_FILE, reg_sz, shortcut_properties,
    shortcut_rename,
};
use crate::win::wide;
use std::ffi::c_void;
use std::path::Path;
use windows_sys::Win32::Foundation::PROPERTYKEY;
use windows_sys::Win32::System::Com::{
    COINIT_APARTMENTTHREADED, CoInitializeEx, CoTaskMemAlloc, CoTaskMemFree,
};
use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, REG_SZ, RegSetKeyValueW};
use windows_sys::Win32::UI::Shell::PropertiesSystem::{
    GPS_READWRITE, PSGetPropertyKeyFromName, SHGetPropertyStoreFromParsingName,
};
use windows_sys::core::{GUID, HRESULT};

/// `IID_IPropertyStore`.
#[allow(non_upper_case_globals)]
const IID_IPropertyStore: GUID = GUID {
    data1: 0x886d_8eeb,
    data2: 0x8cf2,
    data3: 0x4446,
    data4: [0x8d, 0x02, 0xcd, 0xba, 0x1d, 0xbd, 0xcf, 0x99],
};

/// `VT_LPWSTR`.
const VT_LPWSTR: u16 = 31;

/// The vtable of `IPropertyStore`; only `SetValue` and `Commit` are called.
#[repr(C)]
struct StoreVtbl {
    query_interface: usize,
    add_ref: usize,
    release: unsafe extern "system" fn(*mut Store) -> u32,
    get_count: usize,
    get_at: usize,
    get_value: usize,
    set_value:
        unsafe extern "system" fn(*mut Store, *const PROPERTYKEY, *const PropVariant) -> HRESULT,
    commit: unsafe extern "system" fn(*mut Store) -> HRESULT,
}

#[repr(C)]
struct Store {
    vtbl: *const StoreVtbl,
}

/// A `PROPVARIANT` holding a string: the 8 byte header and the 16 byte union.
#[repr(C)]
struct PropVariant {
    vt: u16,
    reserved: [u16; 3],
    value: [usize; 2],
}

/// Sets every `shortcut_properties` entry on the shortcut file `path`
/// through `IPropertyStore`. False when anything failed.
fn set_shortcut_properties(path: &Path) -> bool {
    let wpath = wide(path);
    let mut raw: *mut c_void = std::ptr::null_mut();
    // SAFETY: `wpath` is NUL terminated and the IID is a valid GUID; `raw`
    // receives an owned interface pointer on success.
    let hr = unsafe {
        SHGetPropertyStoreFromParsingName(
            wpath.as_ptr(),
            std::ptr::null_mut(),
            GPS_READWRITE,
            &IID_IPropertyStore,
            &mut raw,
        )
    };
    if hr < 0 || raw.is_null() {
        return false;
    }
    let store = raw.cast::<Store>();
    let mut ok = true;
    for (name, value) in shortcut_properties() {
        let mut key = PROPERTYKEY {
            fmtid: GUID::from_u128(0),
            pid: 0,
        };
        let wname = wide(name);
        let wvalue = wide(value);
        let bytes = wvalue.len() * 2;
        // SAFETY: `wname` is NUL terminated; the allocation holds `bytes`
        // bytes, which is the whole NUL terminated `wvalue`; SetValue copies
        // the variant, so the allocation is freed right after the call; the
        // vtable is that of IPropertyStore, which `store` implements.
        unsafe {
            let text = CoTaskMemAlloc(bytes).cast::<u16>();
            if PSGetPropertyKeyFromName(wname.as_ptr(), &mut key) < 0 || text.is_null() {
                ok = false;
                CoTaskMemFree(text.cast());
                continue;
            }
            std::ptr::copy_nonoverlapping(wvalue.as_ptr(), text, wvalue.len());
            let variant = PropVariant {
                vt: VT_LPWSTR,
                reserved: [0; 3],
                value: [text as usize, 0],
            };
            ok &= ((*(*store).vtbl).set_value)(store, &key, &variant) >= 0;
            CoTaskMemFree(text.cast());
        }
    }
    // SAFETY: `store` is a live interface pointer; it is released once.
    unsafe {
        ok &= ((*(*store).vtbl).commit)(store) >= 0;
        ((*(*store).vtbl).release)(store);
    }
    ok
}

/// Renames the Start shortcut and writes the AUMID DisplayName. A failure is
/// written to stderr (the daemon's log file) and never stops the caller.
pub fn sync() {
    if let Some(programs) = crate::paths::start_menu_programs()
        && let Some((from, to)) = shortcut_rename(&programs, |p| p.exists())
        && let Err(e) = std::fs::rename(&from, &to)
    {
        eprintln!("kawaro: cannot rename {}: {e}", from.display());
    }
    if let Some(programs) = crate::paths::start_menu_programs() {
        // SAFETY: a plain call; a thread that already has COM is fine too.
        unsafe { CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32) };
        for file in [MSI_SHORTCUT_FILE, SHORTCUT_FILE] {
            let path = programs.join(file);
            if path.exists() && !set_shortcut_properties(&path) {
                eprintln!("kawaro: cannot set the app id on {}", path.display());
            }
        }
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
