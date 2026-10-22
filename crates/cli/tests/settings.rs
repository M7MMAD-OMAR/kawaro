//! `sbarhive settings`, the command the app launcher entry runs. The daemon
//! side is a fake socket server, so no window opens.
#![cfg(unix)]

use sbarhive_core::ipc::{Request, Response};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::process::Command;
use std::sync::{Arc, Mutex};

#[test]
fn settings_asks_the_running_daemon_to_open_its_window() {
    let dir = tempfile::tempdir().unwrap();
    let listener = UnixListener::bind(dir.path().join("sbarhive.sock")).unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = seen.clone();
    std::thread::spawn(move || {
        for conn in listener.incoming() {
            let Ok(mut conn) = conn else { return };
            let mut line = String::new();
            BufReader::new(&conn).read_line(&mut line).unwrap();
            let req: Request = serde_json::from_str(&line).unwrap();
            if req != Request::Ping {
                log.lock().unwrap().push(req);
            }
            let ok = serde_json::to_string(&Response::Ok).unwrap();
            let _ = writeln!(conn, "{ok}");
        }
    });
    let o = Command::new(env!("CARGO_BIN_EXE_sbarhive"))
        .arg("settings")
        .env_clear()
        .env("HOME", dir.path())
        .env("XDG_RUNTIME_DIR", dir.path())
        .env("PATH", dir.path())
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(seen.lock().unwrap().as_slice(), [Request::Settings]);
}

#[test]
fn settings_without_a_daemon_to_start_fails_clearly() {
    let dir = tempfile::tempdir().unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_sbarhive"))
        .arg("settings")
        .env_clear()
        .env("HOME", dir.path())
        .env("XDG_RUNTIME_DIR", dir.path())
        .env("PATH", dir.path())
        .output()
        .unwrap();
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("sbarhived"));
}
