//! The place function on Windows (S-041 AC7, ADR 0005 section 4):
//! `MoveFileExW` with flags 0, no replace flag, so the move fails when the
//! name is taken, even on FAT and exFAT where `hard_link` cannot run. The
//! error code map is pure and tested in `output_impl.rs`; this file only
//! calls the system, so it is left out of the mutants run and checked live
//! in S-052.

use super::{move_error_kind, win_long_path};
use crate::win::wide;
use std::io;
use std::path::Path;
use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::Storage::FileSystem::{MOVE_FILE_FLAGS, MoveFileExW};

/// No flag: never replace, never copy across volumes (the temp file sits
/// next to the output, so it is always the same volume).
const FLAGS: MOVE_FILE_FLAGS = 0;

/// Moves `from` to `to`; `AlreadyExists` when `to` is taken.
pub(super) fn place_no_replace(from: &Path, to: &Path) -> io::Result<()> {
    let (from, to) = (wide(win_long_path(from)), wide(win_long_path(to)));
    // SAFETY: both buffers are NUL terminated and outlive the call.
    if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), FLAGS) } != 0 {
        return Ok(());
    }
    // SAFETY: read right after the failed call on the same thread.
    let code = unsafe { GetLastError() };
    let os = io::Error::from_raw_os_error(code as i32);
    match move_error_kind(code) {
        io::ErrorKind::Other => Err(os),
        kind => Err(io::Error::new(kind, os)),
    }
}
