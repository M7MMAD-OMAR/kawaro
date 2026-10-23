//! S-040a AC1 to AC4 and AC6 (backlog/S-040-windows-ipc.md): the IPC
//! transport behind one interface, tested on Linux in a temporary dir.
//!
//! Contract (tester, from AC1 and AC2; do not edit). In
//! `sbarhive_core::transport`:
//! - `pub fn endpoint() -> Endpoint`; on Unix it displays exactly as
//!   `ipc::socket_path().display()`.
//! - `Endpoint: Clone + Debug + Display`, and on Unix
//!   `Endpoint::from_path(impl Into<PathBuf>) -> Endpoint`, so tests and
//!   callers can name a socket without touching `XDG_RUNTIME_DIR`.
//! - `pub enum Bound { Listening(Listener), AlreadyRunning }`.
//! - `Listener::bind(&Endpoint) -> io::Result<Bound>`, and
//!   `Listener::accept(&self) -> io::Result<Stream>`. `Listener: Send`.
//! - `Stream::connect(&Endpoint) -> io::Result<Stream>`,
//!   `Stream::set_read_timeout(&self, Option<Duration>) -> io::Result<()>`,
//!   `Stream::try_clone(&self) -> io::Result<Stream>`,
//!   `Stream::server_pid(&self) -> Option<u32>` (`None` on Unix), and
//!   `Stream: Read + Write + Send`.
//! - `pub fn pipe_name(sid: &str, session: u32) -> String`, on every
//!   platform.
//! - A read past the timeout fails with `ErrorKind::TimedOut`, never
//!   `WouldBlock`.
//! - `bind` answers `AlreadyRunning` when a server on the endpoint answers
//!   `{"cmd":"ping"}` with `{"status":"ok"}` within 500 ms; a stale socket
//!   file nobody listens on is removed and bound.

#[cfg_attr(not(unix), allow(unused_imports))]
use sbarhive_core::ipc::{self, Request, Response};
#[cfg_attr(not(unix), allow(unused_imports))]
use sbarhive_core::transport::{self, Bound, Endpoint, Listener, Stream};
#[cfg_attr(not(unix), allow(unused_imports))]
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
#[cfg(unix)]
use std::path::Path;
#[cfg(unix)]
use std::thread;
#[cfg(unix)]
use std::time::{Duration, Instant};

#[cfg(unix)]
fn ep(dir: &Path) -> Endpoint {
    Endpoint::from_path(dir.join("sbarhive.sock"))
}

#[cfg(unix)]
fn listen(e: &Endpoint) -> Listener {
    match Listener::bind(e).expect("bind") {
        Bound::Listening(l) => l,
        Bound::AlreadyRunning => panic!("nothing listens yet, bind must listen"),
    }
}

#[cfg(unix)]
fn read_line(s: &Stream) -> std::io::Result<String> {
    let mut line = String::new();
    BufReader::new(s.try_clone()?).read_line(&mut line)?;
    Ok(line)
}

fn need_stream<T: Read + Write + Send + 'static>() {}
fn need_send<T: Send + 'static>() {}

#[test]
fn ac1_stream_is_read_write_send_and_listener_is_send() {
    need_stream::<Stream>();
    need_send::<Listener>();
}

#[test]
fn ac1_socket_name_and_socket_path_keep_their_value() {
    assert_eq!(ipc::SOCKET_NAME, "sbarhive.sock");
    let p: std::path::PathBuf = ipc::socket_path();
    assert_eq!(p.file_name().unwrap(), "sbarhive.sock");
}

#[cfg(unix)]
#[test]
fn ac1_endpoint_on_unix_is_the_socket_path() {
    assert_eq!(
        transport::endpoint().to_string(),
        ipc::socket_path().display().to_string()
    );
}

#[cfg(unix)]
#[test]
fn ac1_endpoint_from_path_displays_the_path() {
    let e = Endpoint::from_path("/run/user/1000/sbarhive.sock");
    assert_eq!(e.to_string(), "/run/user/1000/sbarhive.sock");
    assert_eq!(e.clone().to_string(), e.to_string());
}

#[cfg(unix)]
#[test]
fn ac1_ac3_server_pid_is_none_on_unix() {
    let dir = tempfile::tempdir().unwrap();
    let e = ep(dir.path());
    let l = listen(&e);
    let server = thread::spawn(move || {
        let s = l.accept().unwrap();
        assert_eq!(s.server_pid(), None);
    });
    let c = Stream::connect(&e).unwrap();
    assert_eq!(c.server_pid(), None);
    server.join().unwrap();
}

#[test]
fn ac2_pipe_name_is_the_exact_literal() {
    assert_eq!(
        transport::pipe_name("S-1-5-21-1-2-3-1001", 1),
        r"\\.\pipe\io.github.m7mmad_omar.SbarHive.S-1-5-21-1-2-3-1001.1"
    );
}

#[test]
fn ac2_pipe_name_differs_by_sid_and_by_session() {
    let base = transport::pipe_name("S-1-5-21-1-2-3-1001", 1);
    assert_ne!(transport::pipe_name("S-1-5-21-1-2-3-1002", 1), base);
    assert_ne!(transport::pipe_name("S-1-5-21-1-2-3-1001", 2), base);
    assert_eq!(
        transport::pipe_name("S-1-5-21-1-2-3-1001", 2),
        r"\\.\pipe\io.github.m7mmad_omar.SbarHive.S-1-5-21-1-2-3-1001.2"
    );
}

#[test]
fn ac2_longest_sid_and_session_fit_in_256_chars() {
    let sid = "S-1-5-21-4294967295-4294967295-4294967295-4294967295";
    let name = transport::pipe_name(sid, u32::MAX);
    assert_eq!(
        name,
        format!(r"\\.\pipe\io.github.m7mmad_omar.SbarHive.{sid}.4294967295")
    );
    assert!(
        name.chars().count() <= 256,
        "{} chars",
        name.chars().count()
    );
}

#[cfg(unix)]
#[test]
fn ac3_ping_line_reaches_the_server_and_the_reply_reaches_the_client() {
    let dir = tempfile::tempdir().unwrap();
    let e = ep(dir.path());
    let l = listen(&e);
    let server = thread::spawn(move || {
        let mut s = l.accept().unwrap();
        let line = read_line(&s).unwrap();
        s.write_all(b"{\"status\":\"ok\"}\n").unwrap();
        s.flush().unwrap();
        line
    });
    let mut c = Stream::connect(&e).unwrap();
    c.write_all(b"{\"cmd\":\"ping\"}\n").unwrap();
    c.flush().unwrap();
    let reply = read_line(&c).unwrap();
    assert_eq!(server.join().unwrap(), "{\"cmd\":\"ping\"}\n");
    assert_eq!(reply, "{\"status\":\"ok\"}\n");
    assert_eq!(
        serde_json::from_str::<Response>(&reply).unwrap(),
        Response::Ok
    );
}

#[cfg(unix)]
#[test]
fn ac3_lines_from_a_stream_and_its_try_clone_arrive_in_order() {
    let dir = tempfile::tempdir().unwrap();
    let e = ep(dir.path());
    let l = listen(&e);
    let server = thread::spawn(move || {
        let s = l.accept().unwrap();
        let mut r = BufReader::new(s);
        let (mut a, mut b) = (String::new(), String::new());
        r.read_line(&mut a).unwrap();
        r.read_line(&mut b).unwrap();
        (a, b)
    });
    let mut c = Stream::connect(&e).unwrap();
    let mut clone = c.try_clone().unwrap();
    c.write_all(b"first\n").unwrap();
    c.flush().unwrap();
    clone.write_all(b"second\n").unwrap();
    clone.flush().unwrap();
    let (a, b) = server.join().unwrap();
    assert_eq!(a, "first\n");
    assert_eq!(b, "second\n");
}

#[cfg(unix)]
#[test]
fn ac4_read_timeout_is_timed_out_between_180_and_1000_ms() {
    let dir = tempfile::tempdir().unwrap();
    let e = ep(dir.path());
    let l = listen(&e);
    let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
    let server = thread::spawn(move || {
        let _s = l.accept().unwrap();
        // Accepts and never writes, until the client is done.
        let _ = done_rx.recv_timeout(Duration::from_secs(5));
    });
    let c = Stream::connect(&e).unwrap();
    c.set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();
    let start = Instant::now();
    let mut line = String::new();
    let err = BufReader::new(c.try_clone().unwrap())
        .read_line(&mut line)
        .expect_err("nothing was written, the read must time out");
    let took = start.elapsed();
    done_tx.send(()).unwrap();
    server.join().unwrap();
    assert_eq!(err.kind(), ErrorKind::TimedOut, "got {err:?}");
    assert!(
        took >= Duration::from_millis(180) && took <= Duration::from_millis(1000),
        "timed out after {took:?}"
    );
}

#[cfg(unix)]
#[test]
fn ac4_no_timeout_reads_a_reply_written_after_300_ms() {
    let dir = tempfile::tempdir().unwrap();
    let e = ep(dir.path());
    let l = listen(&e);
    let server = thread::spawn(move || {
        let mut s = l.accept().unwrap();
        thread::sleep(Duration::from_millis(300));
        s.write_all(b"{\"status\":\"ok\"}\n").unwrap();
        s.flush().unwrap();
    });
    let c = Stream::connect(&e).unwrap();
    // A timeout set and then cleared must not linger.
    c.set_read_timeout(Some(Duration::from_millis(100)))
        .unwrap();
    c.set_read_timeout(None).unwrap();
    let start = Instant::now();
    let reply = read_line(&c).unwrap();
    assert_eq!(reply, "{\"status\":\"ok\"}\n");
    assert!(start.elapsed() >= Duration::from_millis(250));
    server.join().unwrap();
}

#[cfg(unix)]
#[test]
fn ac6_bind_is_already_running_when_a_server_answers_ping() {
    let dir = tempfile::tempdir().unwrap();
    let e = ep(dir.path());
    let l = listen(&e);
    let pings = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let seen = pings.clone();
    thread::spawn(move || {
        while let Ok(mut s) = l.accept() {
            let Ok(line) = read_line(&s) else { continue };
            if serde_json::from_str::<Request>(&line).ok() == Some(Request::Ping) {
                seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
            let ok = serde_json::to_string(&Response::Ok).unwrap();
            let _ = writeln!(s, "{ok}");
        }
    });
    let start = Instant::now();
    match Listener::bind(&e).unwrap() {
        Bound::AlreadyRunning => {}
        Bound::Listening(_) => panic!("a live server answers ping, bind must not listen"),
    }
    assert!(start.elapsed() < Duration::from_millis(1000));
    assert!(
        pings.load(std::sync::atomic::Ordering::SeqCst) >= 1,
        "bind never sent ping"
    );
    // The live server's socket is still there and still answers.
    let mut c = Stream::connect(&e).unwrap();
    writeln!(c, "{}", serde_json::to_string(&Request::Ping).unwrap()).unwrap();
    assert_eq!(read_line(&c).unwrap().trim(), "{\"status\":\"ok\"}");
}

#[cfg(unix)]
#[test]
fn ac6_a_stale_socket_file_is_removed_and_bound() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sbarhive.sock");
    drop(std::os::unix::net::UnixListener::bind(&path).unwrap());
    assert!(
        path.exists(),
        "the stale socket file stays after its listener"
    );
    let e = Endpoint::from_path(&path);
    let l = listen(&e);
    let server = thread::spawn(move || {
        let mut s = l.accept().unwrap();
        let line = read_line(&s).unwrap();
        s.write_all(line.as_bytes()).unwrap();
    });
    let mut c = Stream::connect(&e).unwrap();
    c.write_all(b"again\n").unwrap();
    assert_eq!(read_line(&c).unwrap(), "again\n");
    server.join().unwrap();
}

// S-028b AC7 (backlog/S-028-review-fixes.md): `Listener::cleanup(&self) ->
// transport::Cleanup`; dropping the `Cleanup` removes the socket file on
// Unix, and a second one dropped after the removal does not panic.

#[cfg(unix)]
#[test]
fn s028_ac7_dropping_cleanup_removes_the_socket_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sbarhive.sock");
    let l = listen(&Endpoint::from_path(&path));
    assert!(path.exists(), "bind leaves the socket file");
    let cleanup: transport::Cleanup = l.cleanup();
    assert!(path.exists(), "taking the cleanup removes nothing yet");
    drop(cleanup);
    assert!(
        !path.exists(),
        "dropping the cleanup removes the socket file"
    );
}

#[cfg(unix)]
#[test]
fn s028_ac7_a_second_cleanup_dropped_after_removal_does_not_panic() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sbarhive.sock");
    let l = listen(&Endpoint::from_path(&path));
    let first = l.cleanup();
    let second = l.cleanup();
    drop(first);
    assert!(!path.exists());
    drop(second);
    assert!(!path.exists());
}
