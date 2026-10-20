//! Runs the `sbarhive` binary with a private HOME, runtime dir and PATH. The
//! daemon side is a fake socket server in this process, so no window opens.

use sbarhive_core::ipc::{Event, Request, Response};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

const BIN: &str = env!("CARGO_BIN_EXE_sbarhive");

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/images")
        .join(name)
}

struct Env {
    dir: tempfile::TempDir,
    path: String,
}

impl Env {
    /// `path` is the child's PATH; `None` gives it an empty private bin dir.
    fn new(path: Option<&str>) -> Self {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("bin")).unwrap();
        let bin = dir.path().join("bin").display().to_string();
        let path = path.map_or(bin.clone(), |p| format!("{bin}:{p}"));
        Env { dir, path }
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn file(&self, name: &str, from: &Path) -> PathBuf {
        let p = self.root().join(name);
        std::fs::copy(from, &p).unwrap();
        p
    }

    fn tool(&self, name: &str, script: &str) {
        let p = self.root().join("bin").join(name);
        std::fs::write(&p, format!("#!/bin/sh\n{script}\n")).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn cmd(&self, args: &[&str]) -> Command {
        let mut c = Command::new(BIN);
        c.args(args)
            .env_clear()
            .env("HOME", self.root())
            .env("XDG_RUNTIME_DIR", self.root())
            .env("PATH", &self.path)
            .env("LANG", "en_US.UTF-8");
        c
    }

    fn run(&self, args: &[&str]) -> Output {
        self.cmd(args).output().unwrap()
    }
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

/// Answers `Ping` with `Ok`, records every other request, and answers it with
/// the replies `reply` gives, one JSON line each.
fn fake_daemon(
    env: &Env,
    reply: impl Fn(&Request) -> Vec<Response> + Send + 'static,
) -> Arc<Mutex<Vec<Request>>> {
    let listener = UnixListener::bind(env.root().join("sbarhive.sock")).unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = seen.clone();
    std::thread::spawn(move || {
        for conn in listener.incoming() {
            let Ok(mut conn) = conn else { return };
            let mut line = String::new();
            BufReader::new(&conn).read_line(&mut line).unwrap();
            let req: Request = serde_json::from_str(&line).unwrap();
            let replies = if req == Request::Ping {
                vec![Response::Ok]
            } else {
                log.lock().unwrap().push(req.clone());
                reply(&req)
            };
            for r in replies {
                let _ = writeln!(conn, "{}", serde_json::to_string(&r).unwrap());
            }
        }
    });
    seen
}

fn event(event: Event) -> Response {
    Response::Event { event }
}

#[test]
fn convert_writes_the_requested_format_beside_the_original() {
    let env = Env::new(None);
    let png = env.file("a.png", &fixture("alpha.png"));
    let o = env.run(&["convert", "--to", "jpg", png.to_str().unwrap()]);
    assert!(o.status.success(), "{}", stderr(&o));
    let out = env.root().join("a.jpg");
    assert_eq!(stdout(&o).trim(), out.display().to_string());
    assert!(out.is_file());
    assert!(!env.root().join("a.webp").exists());
}

#[test]
fn convert_quality_must_be_between_1_and_100() {
    let env = Env::new(None);
    let png = env.file("a.png", &fixture("alpha.png"));
    let p = png.to_str().unwrap();
    let o = env.run(&["convert", "--to", "webp", "--quality", "0", p]);
    assert!(!o.status.success());
    assert!(stderr(&o).contains("quality must be between 1 and 100"));
    let o = env.run(&["convert", "--to", ".WEBP", "--quality", "50", p]);
    assert!(o.status.success(), "{}", stderr(&o));
    assert!(env.root().join("a.webp").is_file());
}

#[test]
fn convert_to_an_unknown_format_lists_the_plain_conversions() {
    let env = Env::new(None);
    let png = env.file("a.png", &fixture("alpha.png"));
    // A conversion with a suffix is not a plain one and stays out of the list.
    let dir = env.root().join("cfg/sbarhive/actions");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("small.toml"),
        r#"[[action]]
id = "mine.small-webp"
engine = "image"
op = "convert"
ring = 2
accepts = ["image"]
output = { mode = "per-file", ext = "webp", suffix = "-small" }
code = { ar = "S", en = "S" }
label = { ar = "صغير", en = "Small" }
"#,
    )
    .unwrap();
    let o = env
        .cmd(&["convert", "--to", "zzz", png.to_str().unwrap()])
        .env("XDG_CONFIG_HOME", env.root().join("cfg"))
        .output()
        .unwrap();
    assert!(!o.status.success());
    assert!(
        stderr(&o).contains("available: webp, jpg, avif, pdf, ico, gif, tiff, bmp\n"),
        "{}",
        stderr(&o)
    );
}

#[test]
fn convert_rejects_a_missing_file() {
    let env = Env::new(None);
    let missing = env.root().join("nope.png");
    let o = env.run(&["convert", "--to", "webp", missing.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(1));
    assert!(stderr(&o).contains("not a file"), "{}", stderr(&o));
}

#[test]
fn a_failed_file_fails_the_command_and_is_counted() {
    let env = Env::new(None);
    let bad = env.root().join("bad.png");
    std::fs::write(&bad, "junk").unwrap();
    let good = env.file("a.png", &fixture("alpha.png"));
    let o = env.run(&[
        "convert",
        "--to",
        "webp",
        bad.to_str().unwrap(),
        good.to_str().unwrap(),
    ]);
    assert_eq!(o.status.code(), Some(1));
    assert!(stderr(&o).contains("1 file(s) failed"), "{}", stderr(&o));
    assert!(env.root().join("a.webp").is_file());
}

#[test]
fn run_without_a_daemon_uses_the_engine_the_action_names() {
    let env = Env::new(None);
    let png = env.file("a.png", &fixture("alpha.png"));
    let o = env.run(&["convert", "--to", "pdf", png.to_str().unwrap()]);
    assert!(o.status.success(), "{}", stderr(&o));
    let pdf = env.root().join("a.pdf");
    // pdf.rotate belongs to the pure Rust `pdf` engine, not the first one.
    let o = env.run(&["run", "pdf.rotate", pdf.to_str().unwrap()]);
    assert!(o.status.success(), "{}", stderr(&o));
    let out = PathBuf::from(stdout(&o).trim());
    assert!(out.is_file() && out != pdf, "{}", stdout(&o));
}

#[test]
fn run_rejects_an_unknown_action() {
    let env = Env::new(None);
    let png = env.file("a.png", &fixture("alpha.png"));
    let o = env.run(&["run", "no.such", png.to_str().unwrap()]);
    assert!(!o.status.success());
    assert!(stderr(&o).contains("unknown action `no.such`"));
}

#[test]
fn run_wait_prints_outputs_and_failures_until_the_job_finishes() {
    let env = Env::new(None);
    let png = env.file("a.png", &fixture("alpha.png"));
    let seen = fake_daemon(&env, |_| {
        vec![
            event(Event::Started {
                job: 1,
                action_id: "image.to-webp".into(),
                tasks: 2,
            }),
            event(Event::TaskDone {
                job: 1,
                output: "/out/a.webp".into(),
            }),
            event(Event::TaskFailed {
                job: 1,
                input: "/in/b.png".into(),
                error: "boom".into(),
            }),
            event(Event::Finished {
                job: 1,
                ok: 1,
                failed: 1,
                cancelled: false,
            }),
            event(Event::TaskDone {
                job: 1,
                output: "/after/finished".into(),
            }),
        ]
    });
    let o = env.run(&["run", "--wait", "image.to-webp", png.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(1));
    assert_eq!(stdout(&o), "/out/a.webp\n");
    assert!(
        stderr(&o).contains("failed: /in/b.png: boom"),
        "{}",
        stderr(&o)
    );
    assert!(stderr(&o).contains("1 file(s) failed"));
    let seen = seen.lock().unwrap();
    assert_eq!(
        seen.as_slice(),
        [Request::Run {
            action: "image.to-webp".into(),
            files: vec![png],
            wait: true
        }]
    );
}

#[test]
fn run_wait_succeeds_when_nothing_failed() {
    let env = Env::new(None);
    let png = env.file("a.png", &fixture("alpha.png"));
    fake_daemon(&env, |_| {
        vec![
            event(Event::TaskDone {
                job: 1,
                output: "/out/a.webp".into(),
            }),
            event(Event::Finished {
                job: 1,
                ok: 1,
                failed: 0,
                cancelled: false,
            }),
        ]
    });
    let o = env.run(&["run", "--wait", "image.to-webp", png.to_str().unwrap()]);
    assert!(o.status.success(), "{}", stderr(&o));
    assert_eq!(stdout(&o), "/out/a.webp\n");
}

#[test]
fn run_without_wait_returns_after_the_first_reply() {
    let env = Env::new(None);
    let png = env.file("a.png", &fixture("alpha.png"));
    let seen = fake_daemon(&env, |_| {
        vec![
            Response::Ok,
            event(Event::TaskDone {
                job: 1,
                output: "/x".into(),
            }),
        ]
    });
    let o = env.run(&["run", "image.to-webp", png.to_str().unwrap()]);
    assert!(o.status.success(), "{}", stderr(&o));
    assert_eq!(stdout(&o), "");
    assert!(matches!(
        seen.lock().unwrap()[0],
        Request::Run { wait: false, .. }
    ));
}

#[test]
fn open_from_the_clipboard_sends_the_copied_files_and_the_token() {
    let env = Env::new(None);
    let png = env.file("a b.png", &fixture("alpha.png"));
    env.tool(
        "wl-paste",
        &format!(
            "printf '%s\\r\\n' 'file://{}'",
            env.root().join("a%20b.png").display()
        ),
    );
    let seen = fake_daemon(&env, |_| vec![Response::Ok]);
    let o = env
        .cmd(&["open", "--from-clipboard"])
        .env("WAYLAND_DISPLAY", "wayland-test")
        .env("XDG_ACTIVATION_TOKEN", "tok")
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", stderr(&o));
    assert_eq!(
        seen.lock().unwrap().as_slice(),
        [Request::Open {
            files: vec![png],
            token: Some("tok".into())
        }]
    );
}

#[test]
fn actions_hide_engines_that_are_not_installed() {
    let env = Env::new(None);
    let clip = env.root().join("clip.mp4");
    std::fs::write(&clip, "").unwrap();
    let o = env.run(&["actions", clip.to_str().unwrap()]);
    assert!(o.status.success());
    assert_eq!(stdout(&o), "", "no ffmpeg on this PATH");
}

#[test]
fn actions_include_user_actions_from_the_config_dir() {
    let env = Env::new(None);
    let png = env.file("a.png", &fixture("alpha.png"));
    let dir = env.root().join("cfg/sbarhive/actions");
    std::fs::create_dir_all(&dir).unwrap();
    let mine = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../actions/images.toml"),
    )
    .unwrap()
    .replace("image.", "mine.");
    std::fs::write(dir.join("mine.toml"), mine).unwrap();
    let o = env
        .cmd(&["actions", "--json", png.to_str().unwrap()])
        .env("XDG_CONFIG_HOME", env.root().join("cfg"))
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", stderr(&o));
    let list: Vec<serde_json::Value> = serde_json::from_slice(&o.stdout).unwrap();
    assert!(list.iter().any(|a| a["id"] == "mine.to-webp"));
    assert!(list.iter().any(|a| a["id"] == "image.to-webp"));
    assert_eq!(list[0]["code"], "WEBP");
}

#[test]
fn doctor_reports_tools_found_on_path() {
    let env = Env::new(None);
    env.tool("gs", "exit 0");
    let o = env.run(&["doctor"]);
    assert!(o.status.success());
    let text = stdout(&o);
    assert!(text.contains("ok      gs "), "{text}");
    assert!(text.contains("missing ffmpeg "), "{text}");
    assert!(text.contains("ok      image"), "{text}");
    assert!(text.contains("missing sbarhived answering"), "{text}");
}

/// Like `fake_daemon`, but records raw JSON, so a request the protocol does
/// not know yet still reaches the assertion instead of a parse panic.
fn raw_daemon(env: &Env) -> Arc<Mutex<Vec<serde_json::Value>>> {
    let listener = UnixListener::bind(env.root().join("sbarhive.sock")).unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
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
            let ok = serde_json::to_string(&Response::Ok).unwrap();
            let _ = writeln!(conn, "{ok}");
        }
    });
    seen
}

/// Runs `sbarhive drop` with the given args and environment, and returns the
/// one request the daemon saw.
fn drop_request(args: &[&str], vars: &[(&str, &str)]) -> serde_json::Value {
    let env = Env::new(None);
    let seen = raw_daemon(&env);
    let mut all = vec!["drop"];
    all.extend_from_slice(args);
    let mut c = env.cmd(&all);
    for (k, v) in vars {
        c.env(k, v);
    }
    let o = c.output().unwrap();
    assert!(o.status.success(), "drop failed: {}", stderr(&o));
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 1, "requests: {seen:?}");
    let v = seen[0].clone();
    assert_eq!(v["cmd"], "drop", "{v}");
    assert!(v.get("files").is_none(), "drop sends no files: {v}");
    v
}

fn token(v: &serde_json::Value) -> Option<&str> {
    v.get("token").and_then(|t| t.as_str())
}

#[test]
fn ac4_drop_token_flag_wins_over_both_environment_variables() {
    let v = drop_request(
        &["--token", "flag"],
        &[
            ("XDG_ACTIVATION_TOKEN", "xdg"),
            ("DESKTOP_STARTUP_ID", "x11"),
        ],
    );
    assert_eq!(token(&v), Some("flag"));
}

#[test]
fn ac4_drop_takes_xdg_activation_token_before_desktop_startup_id() {
    let v = drop_request(
        &[],
        &[
            ("XDG_ACTIVATION_TOKEN", "xdg"),
            ("DESKTOP_STARTUP_ID", "x11"),
        ],
    );
    assert_eq!(token(&v), Some("xdg"));
}

#[test]
fn ac4_drop_falls_back_to_desktop_startup_id() {
    let v = drop_request(&[], &[("DESKTOP_STARTUP_ID", "x11")]);
    assert_eq!(token(&v), Some("x11"));
}

#[test]
fn ac4_drop_without_any_token_sends_none() {
    let v = drop_request(&[], &[]);
    assert_eq!(token(&v), None, "{v}");
    assert!(v.get("token").is_none_or(|t| t.is_null()), "{v}");
}

#[test]
fn ac4_drop_skips_an_empty_environment_token_like_open() {
    let v = drop_request(
        &[],
        &[("XDG_ACTIVATION_TOKEN", ""), ("DESKTOP_STARTUP_ID", "x11")],
    );
    assert_eq!(token(&v), Some("x11"));
}
