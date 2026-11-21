//! S-145: `kawaro doctor` asks the running daemon to reload, so a tool
//! installed after the daemon started is seen without a restart.
//!
//! The daemon is a fake socket server in this process, so no window opens,
//! no job runs and nothing is written.
#![cfg(unix)]

use kawaro_core::ipc::{Request, Response};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

const BIN: &str = env!("CARGO_BIN_EXE_kawaro");

/// A private runtime dir with the fake daemon socket in it.
struct Env {
    dir: tempfile::TempDir,
}

impl Env {
    fn new() -> Self {
        Env {
            dir: tempfile::tempdir().unwrap(),
        }
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    /// The fake daemon: it answers every request with `Ok` and records what
    /// it saw.
    fn daemon(&self) -> Arc<Mutex<Vec<Request>>> {
        let path = self.root().join("kawaro.sock");
        let listener = UnixListener::bind(path).unwrap();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let log = seen.clone();
        std::thread::spawn(move || {
            for conn in listener.incoming() {
                let Ok(mut conn) = conn else { return };
                let mut line = String::new();
                BufReader::new(&mut conn).read_line(&mut line).unwrap();
                let req: Request = serde_json::from_str(&line).unwrap();
                log.lock().unwrap().push(req.clone());
                let answer = match &req {
                    Request::HoldStatus => Response::Hold {
                        last: None,
                        ago_ms: None,
                    },
                    _ => Response::Ok,
                };
                let _ = writeln!(conn, "{}", serde_json::to_string(&answer).unwrap());
            }
        });
        seen
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(BIN)
            .args(args)
            .env_clear()
            .env("HOME", self.root())
            .env("XDG_RUNTIME_DIR", self.root())
            .env("PATH", self.root())
            .env("LANG", "en_US.UTF-8")
            .output()
            .unwrap()
    }
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

/// The requests the fake daemon saw.
fn requests(seen: &Arc<Mutex<Vec<Request>>>) -> Vec<Request> {
    seen.lock().unwrap().clone()
}

#[test]
fn s145_the_doctor_asks_the_running_daemon_to_reload() {
    let env = Env::new();
    let seen_requests = env.daemon();
    let out = env.run(&["doctor"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("daemon: reloaded"), "{text}");
    assert!(
        requests(&seen_requests).contains(&Request::Reload),
        "the daemon saw the reload"
    );
}

#[test]
fn s145_the_doctor_without_a_daemon_still_works_and_starts_nothing() {
    let env = Env::new();
    let out = env.run(&["doctor"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(!text.contains("daemon: reloaded"), "{text}");
    // The report itself is still printed.
    assert!(text.contains("engines"), "{text}");
}
