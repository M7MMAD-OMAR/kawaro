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

// S-019d, manager decision 1 (backlog/S-019-hive-at-pointer.md):
// `sbarhive drop --at X,Y` passes the pointer position in global logical
// coordinates, as `hyprctl cursorpos` prints it ("1198, 1800"), so a still
// hand needs no wait. The request carries it as `"at": [X, Y]`
// (crates/core/tests/ipc_drop.rs).

fn at(v: &serde_json::Value) -> Option<(f64, f64)> {
    let a = v.get("at")?.as_array()?;
    assert_eq!(a.len(), 2, "{v}");
    Some((a[0].as_f64()?, a[1].as_f64()?))
}

#[test]
fn ac8_s019_decision1_drop_at_sends_the_position_and_plain_drop_none() {
    let v = drop_request(&["--at", "400,500"], &[]);
    assert_eq!(at(&v), Some((400.0, 500.0)), "{v}");
    let v = drop_request(&[], &[]);
    assert!(v.get("at").is_none_or(|a| a.is_null()), "{v}");
}

#[test]
fn ac8_s019_decision1_drop_at_takes_hyprctl_cursorpos_output_as_printed() {
    // `hyprctl cursorpos` prints "X, Y" with a space after the comma.
    let v = drop_request(&["--at", "1198, 1800"], &[]);
    assert_eq!(at(&v), Some((1198.0, 1800.0)), "{v}");
}

#[test]
fn ac8_s019_decision1_drop_at_keeps_fractions_and_negative_coordinates() {
    let v = drop_request(&["--at=-1280.5,300.25"], &[]);
    assert_eq!(at(&v), Some((-1280.5, 300.25)), "{v}");
}

#[test]
fn ac8_s019_decision1_drop_at_goes_with_the_token() {
    let v = drop_request(
        &["--token", "flag", "--at", "10,20"],
        &[("XDG_ACTIVATION_TOKEN", "xdg")],
    );
    assert_eq!(token(&v), Some("flag"));
    assert_eq!(at(&v), Some((10.0, 20.0)), "{v}");
}

#[test]
fn ac8_s019_decision1_drop_at_rejects_a_malformed_position_without_a_request() {
    // The option exists and takes a good position (so the rejections below
    // are about the value, not an unknown option).
    let v = drop_request(&["--at", "1,2"], &[]);
    assert_eq!(at(&v), Some((1.0, 2.0)), "{v}");
    for bad in ["400", "400,500,600", "x,500", "400;500", ""] {
        let env = Env::new(None);
        let seen = raw_daemon(&env);
        let o = env.cmd(&["drop", "--at", bad]).output().unwrap();
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

// S-019e (backlog/S-019-hive-at-pointer.md, AC11 and AC19, amended
// 2026-10-21): `sbarhive drop --hold` is the press of a held key and
// `sbarhive drop --release` its release. The threshold timer is in the
// daemon (AC17): the client sends one request and returns at once. The JSON
// forms are fixed in crates/core/tests/ipc_drop.rs: `"hold": true` on a
// drop, and `{"cmd":"drop-release"}`.

/// Runs `sbarhive drop ARGS` against a fake daemon and returns the requests
/// it saw, the output and the wall time of the command.
fn drop_timed(env: &Env, args: &[&str]) -> (Vec<serde_json::Value>, Output, std::time::Duration) {
    let seen = raw_daemon(env);
    let mut all = vec!["drop"];
    all.extend_from_slice(args);
    let t0 = std::time::Instant::now();
    let o = env.run(&all);
    let took = t0.elapsed();
    let seen = seen.lock().unwrap().clone();
    (seen, o, took)
}

fn hold_flag(v: &serde_json::Value) -> bool {
    v.get("hold").and_then(|h| h.as_bool()).unwrap_or(false)
}

#[test]
fn ac11_s019e_drop_hold_sends_one_press_and_returns_within_200_ms() {
    let env = Env::new(None);
    // A long threshold in the private config: the client must not wait for
    // it (the timer is the daemon's).
    std::fs::create_dir_all(env.root().join(".config/sbarhive")).unwrap();
    std::fs::write(
        env.root().join(".config/sbarhive/config.toml"),
        "hold_ms = 2000\n",
    )
    .unwrap();
    let (seen, o, took) = drop_timed(&env, &["--hold"]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert!(
        took < std::time::Duration::from_millis(200),
        "drop --hold returned after {took:?}"
    );
    assert_eq!(seen.len(), 1, "one request: {seen:?}");
    assert_eq!(seen[0]["cmd"], "drop", "{}", seen[0]);
    assert!(hold_flag(&seen[0]), "\"hold\": true: {}", seen[0]);
}

#[test]
fn ac11_s019e_plain_drop_stays_an_immediate_show() {
    // `sbarhive drop` without --hold keeps the S-014 request (no hold), for
    // a binding that runs it on a plain key press; --hold is the other one.
    let v = drop_request(&[], &[]);
    assert!(!hold_flag(&v), "no hold on a plain drop: {v}");
    let v = drop_request(&["--hold"], &[]);
    assert!(hold_flag(&v), "hold with --hold: {v}");
}

#[test]
fn ac11_s019e_drop_hold_goes_with_at_and_the_token() {
    let v = drop_request(&["--hold", "--at", "1198, 1800", "--token", "T"], &[]);
    assert!(hold_flag(&v), "{v}");
    assert_eq!(at(&v), Some((1198.0, 1800.0)), "{v}");
    assert_eq!(token(&v), Some("T"), "{v}");
}

#[test]
fn ac11_s019e_drop_release_sends_drop_release_and_returns_within_200_ms() {
    let env = Env::new(None);
    let (seen, o, took) = drop_timed(&env, &["--release"]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert!(
        took < std::time::Duration::from_millis(200),
        "drop --release returned after {took:?}"
    );
    assert_eq!(seen, [serde_json::json!({"cmd": "drop-release"})]);
}

#[test]
fn ac11_s019e_drop_release_without_a_daemon_starts_none_and_exits_0() {
    // No socket in the private runtime dir: the release of a key whose
    // press never reached a daemon. The client must not start one (AC11).
    let env = Env::new(None);
    let sock = env.root().join("sbarhive.sock");
    let t0 = std::time::Instant::now();
    let o = env.run(&["drop", "--release"]);
    let took = t0.elapsed();
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert!(
        took < std::time::Duration::from_millis(200),
        "drop --release returned after {took:?}"
    );
    // A daemon started in the background would bind its socket here.
    std::thread::sleep(std::time::Duration::from_millis(1000));
    assert!(
        !sock.exists(),
        "drop --release started a daemon (socket {})",
        sock.display()
    );
}

#[test]
fn ac19_s019e_drop_release_without_a_press_exits_0() {
    // The daemon got no press before: the client still exits 0 (AC19).
    let env = Env::new(None);
    let (seen, o, _) = drop_timed(&env, &["--release"]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert_eq!(seen.len(), 1, "{seen:?}");
}

#[test]
fn ac11_s019e_hold_and_release_together_are_a_usage_error() {
    let env = Env::new(None);
    let seen = raw_daemon(&env);
    // Each option alone is accepted, so the rejections below are about the
    // pair, not an unknown option.
    for one in ["--hold", "--release"] {
        let o = env.run(&["drop", one]);
        assert_eq!(o.status.code(), Some(0), "{one}: {}", stderr(&o));
    }
    let before = seen.lock().unwrap().len();
    assert_eq!(before, 2, "{:?}", seen.lock().unwrap());
    for args in [
        ["drop", "--hold", "--release"],
        ["drop", "--release", "--hold"],
    ] {
        let o = env.run(&args);
        assert_eq!(o.status.code(), Some(2), "{args:?}: {}", stderr(&o));
    }
    assert_eq!(
        seen.lock().unwrap().len(),
        before,
        "nothing sent for the pair"
    );
}
