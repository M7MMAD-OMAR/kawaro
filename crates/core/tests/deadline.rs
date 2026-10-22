//! S-022b AC10 (backlog/S-022-windows-ipc.md): a read with a deadline by a
//! watcher thread that cancels the blocked read, as `CancelIoEx` does for a
//! pipe on Windows. Tested on Linux with a fake read.
//!
//! Contract (tester, from AC10; do not edit). In
//! `sbarhive_core::transport::deadline` (`pub mod`, every platform):
//! `pub fn read_with_deadline<T, R, C>(timeout: Duration, read: R, cancel: C)
//! -> std::io::Result<T>` where `R: FnOnce() -> std::io::Result<T>` runs on
//! the calling thread and `C: FnOnce() + Send + 'static` is owned by the
//! watcher.
//! - If `read` has not returned when `timeout` passes, the watcher calls
//!   `cancel` once; whatever the cancelled `read` then returns, the result
//!   is `ErrorKind::TimedOut`.
//! - If `read` returns first, its result is returned as is and `cancel` is
//!   never called, not even later.
//! - On return no watcher is alive: it has ended and dropped `cancel`.

use sbarhive_core::transport::deadline::read_with_deadline;
use std::io::{self, ErrorKind};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// A cancel callback that counts its calls, signals the fake read, and
/// holds `alive` until the watcher drops it (the live watcher counter).
fn cancel_probe(
    calls: &Arc<AtomicUsize>,
    alive: &Arc<()>,
    tx: mpsc::Sender<()>,
) -> impl FnOnce() + Send + 'static {
    let calls = calls.clone();
    let alive = alive.clone();
    move || {
        let _held = &alive;
        calls.fetch_add(1, Ordering::SeqCst);
        let _ = tx.send(());
    }
}

#[test]
fn ac10_blocked_read_times_out_between_180_and_1000_ms_with_one_cancel() {
    let calls = Arc::new(AtomicUsize::new(0));
    let alive = Arc::new(());
    let (tx, rx) = mpsc::channel();
    let cancel = cancel_probe(&calls, &alive, tx);
    // Blocks until cancelled, like ReadFile on a silent pipe; bounded at 3 s
    // so a watcher that never cancels fails the test instead of hanging it.
    let read = move || -> io::Result<Vec<u8>> {
        match rx.recv_timeout(Duration::from_secs(3)) {
            Ok(()) => Err(io::Error::other("operation aborted")),
            Err(_) => Ok(b"never cancelled\n".to_vec()),
        }
    };

    let start = Instant::now();
    let res = read_with_deadline(Duration::from_millis(200), read, cancel);
    let took = start.elapsed();

    let err = res.expect_err("the read never finished, it must time out");
    assert_eq!(err.kind(), ErrorKind::TimedOut, "got {err:?}");
    assert!(
        took >= Duration::from_millis(180) && took <= Duration::from_millis(1000),
        "timed out after {took:?}"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1, "cancel exactly once");
    assert_eq!(
        Arc::strong_count(&alive),
        1,
        "no watcher may be alive after the return"
    );
    thread::sleep(Duration::from_millis(300));
    assert_eq!(calls.load(Ordering::SeqCst), 1, "still exactly once");
}

#[test]
fn ac10_read_done_within_50_ms_returns_its_data_and_never_cancels() {
    let calls = Arc::new(AtomicUsize::new(0));
    let alive = Arc::new(());
    let (tx, _rx) = mpsc::channel();
    let cancel = cancel_probe(&calls, &alive, tx);
    let read = || -> io::Result<Vec<u8>> {
        thread::sleep(Duration::from_millis(20));
        Ok(b"{\"status\":\"ok\"}\n".to_vec())
    };

    let start = Instant::now();
    let res = read_with_deadline(Duration::from_millis(200), read, cancel);
    let took = start.elapsed();

    assert_eq!(
        res.expect("the read finished in time"),
        b"{\"status\":\"ok\"}\n"
    );
    assert!(
        took < Duration::from_millis(150),
        "a finished read must not wait for the deadline, took {took:?}"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        Arc::strong_count(&alive),
        1,
        "no watcher may be alive after the return"
    );
    // Past the deadline the watcher must still not fire.
    thread::sleep(Duration::from_millis(400));
    assert_eq!(calls.load(Ordering::SeqCst), 0, "cancel 0 times");
}

#[test]
fn ac10_a_read_error_before_the_deadline_is_returned_as_is() {
    let calls = Arc::new(AtomicUsize::new(0));
    let alive = Arc::new(());
    let (tx, _rx) = mpsc::channel();
    let cancel = cancel_probe(&calls, &alive, tx);
    let read = || -> io::Result<Vec<u8>> { Err(io::Error::from(ErrorKind::BrokenPipe)) };

    let err =
        read_with_deadline(Duration::from_millis(200), read, cancel).expect_err("the read failed");
    assert_eq!(err.kind(), ErrorKind::BrokenPipe);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(Arc::strong_count(&alive), 1);
    thread::sleep(Duration::from_millis(300));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
