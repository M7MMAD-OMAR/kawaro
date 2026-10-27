//! S-086 AC1 (backlog/S-086-icon-opens-window.md): `kawarod settings` acts
//! as a client when a daemon answers, through the same `Request::Settings`
//! line `kawaro settings` sends, and exits 0. The fake listener comes from
//! `core::transport`; the child runs with an empty display environment so no
//! window can appear on the live desktop. The window it opens is observed
//! live in the owner checklist W01; here the "no daemon" branch is proven to
//! start the service (it binds the socket and reaches the window path)
//! instead of exiting silently like the old daemon mode did.
//!
//! Test contracts the acceptance criteria leave open, fixed here:
//! - a daemon error reply on `settings` prints `kawarod: <message>` and
//!   exits 1, exactly like the `open` client path of S-046 AC6;
//! - an unknown argument still exits 2 with the message of today.

#![cfg(unix)]

use kawaro_core::transport::{Bound, Endpoint, Listener};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn run(dir: &std::path::Path, args: &[&str]) -> (Option<i32>, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_kawarod"))
        .args(args)
        .env("XDG_RUNTIME_DIR", dir)
        .env("HOME", dir)
        .env("XDG_CONFIG_HOME", dir)
        .env("XDG_DATA_HOME", dir)
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("DISPLAY")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn kawarod");
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            let mut err = String::new();
            std::io::Read::read_to_string(&mut child.stderr.take().unwrap(), &mut err).ok();
            return (status.code(), err);
        }
        if start.elapsed() > Duration::from_secs(10) {
            child.kill().ok();
            child.wait().ok();
            return (None, "timed out".into());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn s086_ac1_settings_sends_one_settings_line_and_exits_0() {
    let dir = tempfile::tempdir().unwrap();
    let endpoint = Endpoint::from_path(dir.path().join("kawaro.sock"));
    let Bound::Listening(listener) = Listener::bind(&endpoint).unwrap() else {
        panic!("fresh socket must bind");
    };
    let lines: Arc<Mutex<Vec<String>>> = Arc::default();
    let seen = lines.clone();
    std::thread::spawn(move || {
        while let Ok(stream) = listener.accept() {
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap_or(0) > 0 {
                seen.lock().unwrap().push(line.trim_end().to_string());
            }
            let mut stream = stream;
            let _ = stream.write_all(b"{\"status\":\"ok\"}\n");
        }
    });

    let (code, err) = run(dir.path(), &["settings"]);
    assert_eq!(code, Some(0), "stderr: {err}");
    assert_eq!(
        *lines.lock().unwrap(),
        vec![r#"{"cmd":"settings"}"#.to_string()],
        "the same Request::Settings line as `kawaro settings` (AC1)"
    );
}

#[test]
fn s086_ac1_settings_reports_the_daemons_error_and_exits_1() {
    let dir = tempfile::tempdir().unwrap();
    let endpoint = Endpoint::from_path(dir.path().join("kawaro.sock"));
    let Bound::Listening(listener) = Listener::bind(&endpoint).unwrap() else {
        panic!("fresh socket must bind");
    };
    std::thread::spawn(move || {
        while let Ok(stream) = listener.accept() {
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            let _ = reader.read_line(&mut line);
            let mut stream = stream;
            let _ = stream.write_all(b"{\"status\":\"error\",\"message\":\"settings refused\"}\n");
        }
    });

    let (code, err) = run(dir.path(), &["settings"]);
    assert_eq!(code, Some(1), "stderr: {err}");
    assert!(err.contains("kawarod: settings refused"), "{err}");
}

#[test]
fn s086_ac1_settings_without_a_daemon_starts_the_service_instead_of_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let (code, err) = run(dir.path(), &["settings"]);
    // No display in the sandbox: the fresh service binds its socket, builds
    // the windows and stops at `kawaro_ui::init`. That is the old silent
    // exit turned into the service start branch of AC1; the window itself
    // needs a real desktop and is checked live in W01.
    assert_eq!(code, Some(1), "stderr: {err}");
    assert!(err.contains("kawarod: no display"), "{err}");
    assert!(!err.contains("already running"), "{err}");
}

#[test]
fn s086_ac1_an_unknown_argument_still_exits_2_with_the_message_of_today() {
    let dir = tempfile::tempdir().unwrap();
    let (code, err) = run(dir.path(), &["bogus"]);
    assert_eq!(code, Some(2), "stderr: {err}");
    assert!(err.contains("kawarod: unknown argument `bogus`"), "{err}");
}
