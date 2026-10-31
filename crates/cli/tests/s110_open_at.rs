//! S-110 (backlog/S-110-open-at-position.md) AC1, AC2, AC8 (help): `kawaro
//! open --at X,Y`, the same parser and meaning as `kawaro drop --at`. The
//! daemon side is a fake socket in this process, so no window opens.
#![cfg(unix)]

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

const BIN: &str = env!("CARGO_BIN_EXE_kawaro");

struct Env {
    dir: tempfile::TempDir,
}

impl Env {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("bin")).unwrap();
        Env { dir }
    }

    fn cmd(&self, args: &[&str]) -> Command {
        let mut c = Command::new(BIN);
        c.args(args)
            .env_clear()
            .env("HOME", self.dir.path())
            .env("XDG_RUNTIME_DIR", self.dir.path())
            .env("PATH", self.dir.path().join("bin"))
            .env("LANG", "en_US.UTF-8");
        c
    }
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

type Seen = Arc<Mutex<Vec<serde_json::Value>>>;

/// A fake daemon that records every non-ping request as raw JSON.
fn daemon(env: &Env) -> Seen {
    let listener = UnixListener::bind(env.dir.path().join("kawaro.sock")).unwrap();
    let seen: Seen = Arc::new(Mutex::new(Vec::new()));
    let log = seen.clone();
    std::thread::spawn(move || {
        for conn in listener.incoming() {
            let Ok(mut conn) = conn else { return };
            let mut line = String::new();
            BufReader::new(&conn).read_line(&mut line).unwrap();
            let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else {
                continue;
            };
            if v["cmd"] != "ping" {
                log.lock().unwrap().push(v);
            }
            let _ = writeln!(conn, r#"{{"status":"ok"}}"#);
        }
    });
    seen
}

fn open_request(args: &[&str]) -> serde_json::Value {
    let env = Env::new();
    let seen = daemon(&env);
    let mut all = vec!["open"];
    all.extend_from_slice(args);
    let o = env.cmd(&all).output().unwrap();
    assert!(o.status.success(), "open failed: {}", stderr(&o));
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 1, "requests: {seen:?}");
    seen[0].clone()
}

fn at(v: &serde_json::Value) -> Option<(f64, f64)> {
    let a = v.get("at")?.as_array()?;
    assert_eq!(a.len(), 2, "{v}");
    Some((a[0].as_f64()?, a[1].as_f64()?))
}

#[test]
fn ac1_open_at_parses_the_three_good_inputs_like_drop() {
    for (input, want) in [
        ("400,500", (400.0, 500.0)),
        ("1198, 1800", (1198.0, 1800.0)),
        ("-1280.5,300.25", (-1280.5, 300.25)),
    ] {
        let v = open_request(&["--at", input, "/a.png"]);
        assert_eq!(at(&v), Some(want), "--at {input:?}: {v}");
    }
}

#[test]
fn ac1_open_at_rejects_the_bad_inputs_with_code_2_and_no_request() {
    // The option exists and takes a good value, so the rejections below are
    // about the value and not an unknown option.
    let v = open_request(&["--at", "1,2", "/a.png"]);
    assert_eq!(at(&v), Some((1.0, 2.0)), "{v}");
    for bad in ["abc", "1", "1,2,3", "nan,1", "inf,1", ""] {
        let env = Env::new();
        let seen = daemon(&env);
        let o = env.cmd(&["open", "--at", bad, "/a.png"]).output().unwrap();
        assert_eq!(o.status.code(), Some(2), "--at {bad:?}: {}", stderr(&o));
        assert!(
            stderr(&o).contains("--at"),
            "--at {bad:?}: the error names the option: {}",
            stderr(&o)
        );
        assert!(
            seen.lock().unwrap().is_empty(),
            "--at {bad:?}: nothing sent"
        );
    }
}

#[test]
fn ac2_open_at_sends_one_open_with_file_token_and_at() {
    let v = open_request(&["--at", "400,500", "--token", "T", "/a.png"]);
    assert_eq!(v["cmd"], "open", "{v}");
    assert_eq!(v["files"], serde_json::json!(["/a.png"]), "{v}");
    assert_eq!(v["token"], "T", "{v}");
    assert_eq!(v["at"], serde_json::json!([400.0, 500.0]), "{v}");
}

#[test]
fn ac2_open_without_at_sends_none() {
    // The with-`--at` request of the same command carries the pair.
    let v = open_request(&["--at", "3,4", "--token", "T", "/a.png"]);
    assert_eq!(at(&v), Some((3.0, 4.0)), "{v}");
    let v = open_request(&["--token", "T", "/a.png"]);
    assert_eq!(v["cmd"], "open", "{v}");
    assert_eq!(v["files"], serde_json::json!(["/a.png"]), "{v}");
    assert!(v.get("at").is_none_or(|a| a.is_null()), "{v}");
}

#[test]
fn ac2_open_at_keeps_negative_and_fractional_values() {
    let v = open_request(&["--at=-1280.5,300.25", "/a.png"]);
    assert_eq!(v["at"], serde_json::json!([-1280.5, 300.25]), "{v}");
}

/// The words of the `--at` option in a help text: its line and the indented
/// continuation lines up to the next option, whitespace normalised, with the
/// option name and value placeholder dropped.
fn at_words(help: &str) -> String {
    let mut out = Vec::new();
    let mut on = false;
    for l in help.lines() {
        let t = l.trim();
        if t.starts_with("--at") {
            on = true;
            out.push(t.split_once("  ").map_or("", |x| x.1).trim().to_string());
        } else if on {
            if t.is_empty() || t.starts_with('-') {
                break;
            }
            out.push(t.to_string());
        }
    }
    out.join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn ac8_open_help_documents_at_in_the_same_words_as_drop() {
    let env = Env::new();
    let help = |sub: &str| {
        let o = env.cmd(&[sub, "--help"]).output().unwrap();
        String::from_utf8_lossy(&o.stdout).into_owned()
    };
    let drop_words = at_words(&help("drop"));
    assert!(!drop_words.is_empty(), "drop documents --at");
    let open_help = help("open");
    assert!(open_help.contains("--at"), "{open_help}");
    assert_eq!(at_words(&open_help), drop_words, "{open_help}");
}
