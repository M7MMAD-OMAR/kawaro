//! S-148 AC3 and AC5 (backlog/S-148-job-control-without-tray.md): the CLI
//! side of the daemon instance.
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

    /// The fake daemon: it answers `Version` with the given answer, every
    /// other request with `reply`, and it records what it saw.
    fn daemon(
        &self,
        version: Response,
        reply: impl Fn(&Request) -> Response + Send + 'static,
    ) -> Arc<Mutex<Vec<Request>>> {
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
                    Request::Version => version.clone(),
                    other => reply(other),
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

/// The answer of a daemon that knows the results commands, with the
/// instance it sends or without one.
fn version(instance: Option<&str>) -> Response {
    Response::Version {
        version: String::from("0.5.0"),
        run_params: true,
        job_results: true,
        instance: instance.map(String::from),
    }
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

/// The cancel request the daemon saw, if it saw one.
fn cancel_request(seen: &Arc<Mutex<Vec<Request>>>) -> Option<(u64, Option<String>)> {
    let requests = seen.lock().unwrap();
    for request in requests.iter() {
        if let Request::CancelJob { job, instance } = request {
            return Some((*job, instance.clone()));
        }
    }
    None
}

#[test]
fn s148_the_jobs_header_names_the_instance_of_the_daemon() {
    let env = Env::new();
    let jobs = Response::Jobs {
        jobs: Vec::new(),
        instance: Some(String::from("k-1a2b-3c4d")),
    };
    env.daemon(version(Some("k-1a2b-3c4d")), move |_| jobs.clone());
    let out = env.run(&["jobs"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("k-1a2b-3c4d"), "{text}");
    // The instance is named once, in the header.
    assert_eq!(text.matches("k-1a2b-3c4d").count(), 1, "{text}");
}

#[test]
fn s148_the_jobs_json_carries_the_instance() {
    let env = Env::new();
    let jobs = Response::Jobs {
        jobs: Vec::new(),
        instance: Some(String::from("k-1a2b-3c4d")),
    };
    env.daemon(version(Some("k-1a2b-3c4d")), move |_| jobs.clone());
    let out = env.run(&["jobs", "--json"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["instance"].as_str(), Some("k-1a2b-3c4d"), "{text}");
    assert!(value["jobs"].is_array(), "{text}");
}

#[test]
fn s148_a_checked_reference_carries_the_instance() {
    let env = Env::new();
    let seen = env.daemon(version(Some("k-1a2b-3c4d")), |_| Response::Ok);
    let out = env.run(&["cancel", "k-1a2b-3c4d:7"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let request = cancel_request(&seen).expect("the daemon saw the cancel");
    assert_eq!(request.0, 7);
    assert_eq!(request.1.as_deref(), Some("k-1a2b-3c4d"));
}

#[test]
fn s148_a_plain_job_number_names_the_current_session() {
    let env = Env::new();
    let seen = env.daemon(version(Some("k-1a2b-3c4d")), |_| Response::Ok);
    let out = env.run(&["cancel", "7"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let request = cancel_request(&seen).expect("the daemon saw the cancel");
    assert_eq!(request.0, 7);
    assert!(request.1.is_none(), "no instance is sent");
}

#[test]
fn s148_a_daemon_without_an_instance_refuses_a_checked_reference() {
    let env = Env::new();
    let seen = env.daemon(version(None), |_| Response::Ok);
    let out = env.run(&["cancel", "k-1a2b-3c4d:7"]);
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("support checked job references"), "{err}");
    assert!(
        cancel_request(&seen).is_none(),
        "the request is never sent to a daemon that cannot check it"
    );
}

#[test]
fn s148_an_old_daemon_keeps_the_old_refusal() {
    let env = Env::new();
    env.daemon(
        Response::Version {
            version: String::from("0.4.0"),
            run_params: true,
            job_results: false,
            instance: None,
        },
        |_| Response::Ok,
    );
    let out = env.run(&["jobs"]);
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("has no job results"), "{err}");
}

#[test]
fn s148_a_text_that_is_no_job_is_refused_before_the_daemon() {
    let env = Env::new();
    let seen = env.daemon(version(Some("k-1a2b-3c4d")), |_| Response::Ok);
    let out = env.run(&["cancel", "seven"]);
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("JOB"), "{err}");
    let sent = cancel_request(&seen);
    assert!(sent.is_none(), "no request reaches the daemon");
}
