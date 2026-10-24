//! The service log file (S-045 AC7). Stands alone: no `crate::` items, so a
//! test can include it by path.

use std::fs::{File, OpenOptions};
use std::io;
use std::path::Path;

/// Opens `path` for appending, creating it and any missing parent folder.
pub fn open_append(path: &Path) -> io::Result<File> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    OpenOptions::new().create(true).append(true).open(path)
}

/// Points the process's stderr at `file`, so every `eprintln!` lands in the
/// log. The service has no console on Windows. The file stays open for the
/// life of the process.
#[cfg(windows)]
pub fn redirect_stderr(file: File) -> io::Result<()> {
    use std::os::windows::io::IntoRawHandle;
    use windows_sys::Win32::System::Console::{STD_ERROR_HANDLE, SetStdHandle};
    let handle = file.into_raw_handle();
    // SAFETY: `handle` is a valid, open file handle that is never closed.
    if unsafe { SetStdHandle(STD_ERROR_HANDLE, handle) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
