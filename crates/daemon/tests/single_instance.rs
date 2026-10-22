//! S-022a AC6 (backlog/S-022-windows-ipc.md): a second `sbarhived` on the
//! same runtime dir finds the first one answering `ping`, prints
//! `sbarhived: already running` and exits 0, as before the transport.
//! The first service here is a fake on `sbarhive_core::transport`, so no
//! window opens; the child gets no display variables either.

use sbarhive_core::ipc::{Request, Response};
use sbarhive_core::transport::{Bound, Endpoint, Listener};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const BIN: &str = env!("CARGO_BIN_EXE_sbarhived");

#[test]
fn ac6_second_daemon_on_the_same_runtime_dir_says_already_running_and_exits_0() {
    let dir = tempfile::tempdir().unwrap();
    let e = Endpoint::from_path(dir.path().join("sbarhive.sock"));
    let l = match Listener::bind(&e).unwrap() {
        Bound::Listening(l) => l,
        Bound::AlreadyRunning => panic!("the temp dir is empty"),
    };
    std::thread::spawn(move || {
        while let Ok(mut s) = l.accept() {
            let mut line = String::new();
            let Ok(clone) = s.try_clone() else { continue };
            if BufReader::new(clone).read_line(&mut line).is_err() {
                continue;
            }
            if serde_json::from_str::<Request>(&line).ok() == Some(Request::Ping) {
                let ok = serde_json::to_string(&Response::Ok).unwrap();
                let _ = writeln!(s, "{ok}");
            }
        }
    });

    let mut child = Command::new(BIN)
        .env_clear()
        .env("HOME", dir.path())
        .env("XDG_RUNTIME_DIR", dir.path())
        .env("XDG_CONFIG_HOME", dir.path().join("config"))
        .env("PATH", "/usr/bin:/bin")
        .env("LANG", "en_US.UTF-8")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let start = Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait().unwrap() {
            break s;
        }
        if start.elapsed() > Duration::from_secs(5) {
            let _ = child.kill();
            let out = child.wait_with_output().unwrap();
            panic!(
                "sbarhived did not exit within 5 s; stderr:\n{}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let out = child.wait_with_output().unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(status.code(), Some(0), "{text}");
    assert!(text.contains("sbarhived: already running"), "{text}");
    assert!(
        dir.path().join("sbarhive.sock").exists(),
        "the first service keeps its socket"
    );
}
