//! The Unix socket transport.

use super::{Bound, Cleanup};
use crate::ipc::{Request, Response};
use std::fmt;
use std::io::{self, BufRead, BufReader, ErrorKind, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::time::Duration;

/// How long `bind` waits for a running service to answer `ping`.
const PING_TIMEOUT: Duration = Duration::from_millis(500);

/// Where the service listens: a socket path.
#[derive(Clone, Debug)]
pub struct Endpoint(PathBuf);

impl Endpoint {
    pub fn from_path(path: impl Into<PathBuf>) -> Endpoint {
        Endpoint(path.into())
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

/// The endpoint of this user's service.
pub fn endpoint() -> Endpoint {
    Endpoint(crate::ipc::socket_path())
}

pub struct Listener {
    listener: UnixListener,
    path: PathBuf,
}

impl Listener {
    /// Listens on `e`, unless a service there answers `ping` within 500 ms.
    /// Any other socket file there is stale: it is removed and bound.
    pub fn bind(e: &Endpoint) -> io::Result<Bound> {
        if answers_ping(e) {
            return Ok(Bound::AlreadyRunning);
        }
        let _ = std::fs::remove_file(&e.0);
        UnixListener::bind(&e.0).map(|listener| {
            Bound::Listening(Listener {
                listener,
                path: e.0.clone(),
            })
        })
    }

    pub fn accept(&self) -> io::Result<Stream> {
        self.listener.accept().map(|(s, _)| Stream(s))
    }

    /// Removes the socket file when the returned value is dropped.
    pub fn cleanup(&self) -> Cleanup {
        Cleanup(Some(self.path.clone()))
    }
}

fn answers_ping(e: &Endpoint) -> bool {
    let Ok(mut s) = UnixStream::connect(&e.0) else {
        return false;
    };
    let _ = s.set_read_timeout(Some(PING_TIMEOUT));
    let line = serde_json::to_string(&Request::Ping).unwrap_or_default();
    if writeln!(s, "{line}").is_err() {
        return false;
    }
    let mut reply = String::new();
    if BufReader::new(s).read_line(&mut reply).is_err() {
        return false;
    }
    serde_json::from_str::<Response>(&reply).ok() == Some(Response::Ok)
}

pub struct Stream(UnixStream);

impl Stream {
    pub fn connect(e: &Endpoint) -> io::Result<Stream> {
        UnixStream::connect(&e.0).map(Stream)
    }

    /// A read past `timeout` fails with `ErrorKind::TimedOut`.
    pub fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        self.0.set_read_timeout(timeout)
    }

    pub fn try_clone(&self) -> io::Result<Stream> {
        self.0.try_clone().map(Stream)
    }

    /// The service's process id, checked on Windows only.
    pub fn server_pid(&self) -> Option<u32> {
        None
    }
}

/// A socket read timeout is `WouldBlock` on Linux; callers see `TimedOut`
/// on every platform.
fn timed_out(e: io::Error) -> io::Error {
    if e.kind() == ErrorKind::WouldBlock {
        io::Error::new(ErrorKind::TimedOut, e)
    } else {
        e
    }
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf).map_err(timed_out)
    }
}

impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}
