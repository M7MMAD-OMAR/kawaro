//! The IPC transport behind one interface (S-022, ADR 0005 section 2): a
//! Unix socket at [`crate::ipc::socket_path`] on Unix, a named pipe per user
//! and session on Windows. The protocol on top is unchanged, see
//! [`crate::ipc`].
//!
//! The Windows decisions (S-022b) are pure functions here, tested and
//! mutated on Linux; `transport/windows.rs` only calls the system with them.

use std::io::{self, ErrorKind};
use std::time::Duration;

pub mod deadline;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::{Endpoint, Listener, Stream, endpoint};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::{Endpoint, Listener, Stream, endpoint};

/// The result of [`Listener::bind`].
pub enum Bound {
    /// This process owns the endpoint and listens on it.
    Listening(Listener),
    /// Another service already answers on the endpoint.
    AlreadyRunning,
}

/// The pipe name for one user (`sid`) in one session, unique per user and
/// session and at most 256 characters for the longest SID.
pub fn pipe_name(sid: &str, session: u32) -> String {
    format!(r"\\.\pipe\io.github.m7mmad_omar.SbarHive.{sid}.{session}")
}

/// `ERROR_FILE_NOT_FOUND`: no pipe by that name, so no service.
/// `connect_plan` passes it on as a raw code, which std reads as
/// `ErrorKind::NotFound` on Windows (and on Linux, where 2 is `ENOENT`).
#[cfg_attr(not(windows), allow(dead_code))]
const ERROR_FILE_NOT_FOUND: u32 = 2;
/// `ERROR_ACCESS_DENIED`: the first instance exists, a service runs.
const ERROR_ACCESS_DENIED: u32 = 5;
/// `ERROR_PIPE_BUSY`: every instance is taken, wait for one.
const ERROR_PIPE_BUSY: u32 = 231;
/// `PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE`.
const OPEN_MODE_FIRST: u32 = 0x0008_0003;
/// `PIPE_ACCESS_DUPLEX`.
const OPEN_MODE_LATER: u32 = 0x3;
/// `PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT` (all 0) with
/// `PIPE_REJECT_REMOTE_CLIENTS`.
const PIPE_MODE: u32 = 0x8;
/// How long a client waits for a busy pipe before it gives up.
const CONNECT_WAIT: Duration = Duration::from_millis(2000);

/// The pipe's DACL: protected, one ACE granting `sid` full access, so no
/// other user, group or anonymous logon can open it.
pub fn dacl_sddl(sid: &str) -> String {
    format!("D:P(A;;GA;;;{sid})")
}

/// What the first `CreateNamedPipeW` of a service means.
#[derive(Debug)]
pub enum BindOutcome {
    Listen,
    AlreadyRunning,
    Failed(io::Error),
}

/// Reads the error of the first pipe instance (`FILE_FLAG_FIRST_PIPE_INSTANCE`):
/// none listens, `ERROR_ACCESS_DENIED` means a service already owns the name.
pub fn bind_result(first_instance_error: Option<u32>) -> BindOutcome {
    match first_instance_error {
        None => BindOutcome::Listen,
        Some(ERROR_ACCESS_DENIED) => BindOutcome::AlreadyRunning,
        Some(code) => BindOutcome::Failed(os_error(code)),
    }
}

/// One step of the client's connect loop.
#[derive(Debug)]
pub enum ConnectStep {
    Connected,
    /// Wait this long with `WaitNamedPipeW`, then try again.
    Wait(Duration),
    Fail(io::Error),
}

/// Decides the next step from the last open's error and the time spent
/// since the first try: a busy pipe is waited for until 2000 ms in all.
pub fn connect_plan(open_error: Option<u32>, elapsed: Duration) -> ConnectStep {
    match open_error {
        None => ConnectStep::Connected,
        Some(ERROR_PIPE_BUSY) if elapsed < CONNECT_WAIT => {
            ConnectStep::Wait(CONNECT_WAIT - elapsed)
        }
        Some(ERROR_PIPE_BUSY) => ConnectStep::Fail(io::Error::new(
            ErrorKind::TimedOut,
            "the service pipe stayed busy",
        )),
        Some(code) => ConnectStep::Fail(os_error(code)),
    }
}

/// Hands `stream` back only when the pipe server runs as the client's own
/// user; otherwise drops it unwritten, so a process that took the name
/// first under another account gets nothing.
pub fn verify_owner<S>(server_sid: &str, client_sid: &str, stream: S) -> io::Result<S> {
    if server_sid == client_sid {
        Ok(stream)
    } else {
        drop(stream);
        Err(io::Error::new(
            ErrorKind::PermissionDenied,
            "the service pipe belongs to another user",
        ))
    }
}

/// The `CreateNamedPipeW` open mode: only the first instance claims the
/// name, so later instances do not fail with `ERROR_ACCESS_DENIED`.
pub fn open_mode(first: bool) -> u32 {
    if first {
        OPEN_MODE_FIRST
    } else {
        OPEN_MODE_LATER
    }
}

/// The `CreateNamedPipeW` pipe mode: bytes, blocking, local clients only.
pub fn pipe_mode() -> u32 {
    PIPE_MODE
}

fn os_error(code: u32) -> io::Error {
    io::Error::from_raw_os_error(code as i32)
}
