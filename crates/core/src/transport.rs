//! The IPC transport behind one interface (S-022, ADR 0005 section 2): a
//! Unix socket at [`crate::ipc::socket_path`] on Unix, a named pipe per user
//! and session on Windows. The protocol on top is unchanged, see
//! [`crate::ipc`].

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
