//! UTF-16 helpers for the wide Windows API (S-029 AC1), shared by core and
//! the engines. Windows only, so `.cargo/mutants.toml` leaves this file out.

use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::{OsStrExt, OsStringExt};

/// `s` as a NUL terminated UTF-16 string.
pub fn wide(s: impl AsRef<OsStr>) -> Vec<u16> {
    s.as_ref().encode_wide().chain(Some(0)).collect()
}

/// The NUL terminated UTF-16 string at `p`, copied.
///
/// # Safety
///
/// `p` must be non-null and point to a readable UTF-16 string that ends
/// with a NUL, valid for the whole call.
pub unsafe fn from_wide_ptr(p: *const u16) -> OsString {
    let mut len = 0;
    // SAFETY: the caller guarantees a NUL ends the string, so every cell up
    // to and including it is readable.
    while unsafe { *p.add(len) } != 0 {
        len += 1;
    }
    // SAFETY: the `len` cells before the NUL were read above.
    OsString::from_wide(unsafe { std::slice::from_raw_parts(p, len) })
}
