//! The Windows side of starting the service and handing it the foreground
//! right (S-024, ADR 0005 sections 3 and 5). Builds for Windows only; the
//! decisions it uses stay mutated in `spawn.rs`.

use super::{CREATE_BREAKAWAY_FROM_JOB, CREATE_NEW_PROCESS_GROUP, DETACHED_PROCESS, start_with};
use std::io;
use std::os::windows::process::CommandExt;
use std::process::Command;
use windows_sys::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow;

const _: () = assert!(DETACHED_PROCESS == windows_sys::Win32::System::Threading::DETACHED_PROCESS);
const _: () = assert!(
    CREATE_NEW_PROCESS_GROUP == windows_sys::Win32::System::Threading::CREATE_NEW_PROCESS_GROUP
);
const _: () = assert!(
    CREATE_BREAKAWAY_FROM_JOB == windows_sys::Win32::System::Threading::CREATE_BREAKAWAY_FROM_JOB
);

/// Spawns `cmd` with no console, in its own process group, outside the
/// terminal's job when the job allows it, so closing the terminal leaves
/// the service running.
pub fn start_detached(cmd: &mut Command) -> io::Result<()> {
    start_with(|flags| cmd.creation_flags(flags).spawn().map(drop))
}

/// Lets the service take the foreground, so the hive it shows gets focus.
pub fn allow_foreground(pid: u32) {
    // SAFETY: takes a plain process id and touches no memory. A failure only
    // means the hive may open without focus, so the result is ignored.
    unsafe {
        AllowSetForegroundWindow(pid);
    }
}
