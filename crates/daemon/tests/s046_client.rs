//! S-046 AC6 (backlog/S-046-windows-hive-window.md): `sbarhived open <file>`
//! acts as a client when a daemon answers. The fake listener comes from
//! `core::transport`; the child runs with an empty display environment so no
//! window can appear on the live desktop. The "no daemon" branch needs a
//! display and is checked by the live run (S-052).

#![cfg(unix)]

use sbarhive_core::transport::{Bound, Endpoint, Listener};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn run(dir: &std::path::Path, args: &[&str]) -> (Option<i32>, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_sbarhived"))
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
        .expect("spawn sbarhived");
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
fn ac6_open_sends_exactly_one_line_to_a_running_daemon_and_exits_0() {
    let dir = tempfile::tempdir().unwrap();
    let endpoint = Endpoint::from_path(dir.path().join("sbarhive.sock"));
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

    let (code, err) = run(dir.path(), &["open", "a.png"]);
    assert_eq!(code, Some(0), "stderr: {err}");
    assert_eq!(
        *lines.lock().unwrap(),
        vec![r#"{"cmd":"open","files":["a.png"],"token":null}"#.to_string()]
    );
}

#[test]
fn ac5_open_without_a_file_exits_nonzero_and_says_why() {
    let dir = tempfile::tempdir().unwrap();
    let (code, err) = run(dir.path(), &["open"]);
    assert!(matches!(code, Some(c) if c != 0), "code {code:?}");
    assert!(err.contains("open needs at least one file"), "{err}");
}

#[test]
fn ac6_open_prints_the_error_of_the_daemon_and_exits_1() {
    let dir = tempfile::tempdir().unwrap();
    let endpoint = Endpoint::from_path(dir.path().join("sbarhive.sock"));
    let Bound::Listening(listener) = Listener::bind(&endpoint).unwrap() else {
        panic!("fresh socket must bind");
    };
    std::thread::spawn(move || {
        while let Ok(stream) = listener.accept() {
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            let _ = reader.read_line(&mut line);
            let mut stream = stream;
            let _ = stream.write_all(b"{\"status\":\"error\",\"message\":\"no fitting action\"}\n");
        }
    });

    let (code, err) = run(dir.path(), &["open", "a.xyz"]);
    assert_eq!(code, Some(1), "stderr: {err}");
    assert!(err.contains("sbarhived: no fitting action"), "{err}");
}
