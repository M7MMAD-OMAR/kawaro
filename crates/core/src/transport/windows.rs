//! The named pipe transport: a seed that answers `ErrorKind::Unsupported`
//! everywhere until S-022b, so the interface builds for Windows.

use super::Bound;
use std::fmt;
use std::io::{self, ErrorKind, Read, Write};
use std::time::Duration;

fn unsupported() -> io::Error {
    io::Error::new(ErrorKind::Unsupported, "named pipe transport (S-022b)")
}

/// Where the service listens: a pipe name.
#[derive(Clone, Debug)]
pub struct Endpoint(String);

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The endpoint of this user's service. The SID and the session come in
/// S-022b; until then the name is empty and every call fails.
pub fn endpoint() -> Endpoint {
    Endpoint(String::new())
}

pub struct Listener {
    _private: (),
}

impl Listener {
    pub fn bind(_e: &Endpoint) -> io::Result<Bound> {
        Err(unsupported())
    }

    pub fn accept(&self) -> io::Result<Stream> {
        Err(unsupported())
    }
}

pub struct Stream {
    _private: (),
}

impl Stream {
    pub fn connect(_e: &Endpoint) -> io::Result<Stream> {
        Err(unsupported())
    }

    pub fn set_read_timeout(&self, _timeout: Option<Duration>) -> io::Result<()> {
        Err(unsupported())
    }

    pub fn try_clone(&self) -> io::Result<Stream> {
        Err(unsupported())
    }

    pub fn server_pid(&self) -> Option<u32> {
        None
    }
}

impl Read for Stream {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        Err(unsupported())
    }
}

impl Write for Stream {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Err(unsupported())
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(unsupported())
    }
}
