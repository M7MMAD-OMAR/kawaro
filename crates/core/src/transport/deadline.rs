//! A read with a deadline (S-040b AC10). A named pipe has no read timeout,
//! so a watcher thread cancels the blocked read when the deadline passes
//! (`CancelIoEx` on Windows). Pure, so it is tested and mutated on Linux.

use std::io::{self, ErrorKind};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::Duration;

/// Runs `read` on this thread. If it has not returned after `timeout`, the
/// watcher calls `cancel` once and the result is `ErrorKind::TimedOut`,
/// whatever the cancelled read returns. A read that returns first keeps its
/// result and `cancel` is never called. No watcher outlives the call.
pub fn read_with_deadline<T, R, C>(timeout: Duration, read: R, cancel: C) -> io::Result<T>
where
    R: FnOnce() -> io::Result<T>,
    C: FnOnce() + Send + 'static,
{
    // Set once, by whichever comes first: the watcher firing or the read.
    let closed = Arc::new(Mutex::new(false));
    let (done, wake) = mpsc::channel::<()>();
    let watch = closed.clone();
    let watcher = thread::spawn(move || {
        if wake.recv_timeout(timeout) == Err(RecvTimeoutError::Timeout) {
            let mut closed = watch.lock().unwrap_or_else(PoisonError::into_inner);
            if !*closed {
                *closed = true;
                drop(closed);
                cancel();
            }
        }
    });

    let result = read();
    let fired = {
        let mut closed = closed.lock().unwrap_or_else(PoisonError::into_inner);
        let fired = *closed;
        *closed = true;
        fired
    };
    drop(done);
    let _ = watcher.join();
    if fired {
        Err(io::Error::new(ErrorKind::TimedOut, "read deadline passed"))
    } else {
        result
    }
}
