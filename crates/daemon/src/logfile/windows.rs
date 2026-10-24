//! Windows only: the stderr redirect for the service log (S-045 AC7).

use std::fs::File;
use std::io;

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
