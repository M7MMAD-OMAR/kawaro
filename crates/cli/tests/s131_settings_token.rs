//! S-131 AC1: kawaro settings sends the activation token from the environment.
//! Contract pinned by this test (tester): the settings command resolves the
//! token exactly like the open path, XDG_ACTIVATION_TOKEN first and then
//! DESKTOP_STARTUP_ID, and sends it on Request::Settings.
#![cfg(unix)]

use kawaro_core::ipc::{Request, Response};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::process::Command;
use std::sync::{Arc, Mutex};

fn settings_request(vars: &[(&str, &str)]) -> Request {
    let dir = tempfile::tempdir().unwrap();
    let listener = UnixListener::bind(dir.path().join("kawaro.sock")).unwrap();
    let seen = Arc::new(Mutex::new(None));
    let slot = seen.clone();
    std::thread::spawn(move || {
        for conn in listener.incoming() {
            let Ok(mut conn) = conn else { return };
            let mut line = String::new();
            if BufReader::new(&conn).read_line(&mut line).unwrap_or(0) == 0 {
                continue;
            }
            let Ok(request) = serde_json::from_str::<Request>(&line) else {
                continue;
            };
            let is_settings = request != Request::Ping;
            if is_settings {
                *slot.lock().unwrap() = Some(request);
            }
            let reply = serde_json::to_string(&Response::Ok).unwrap();
            let _ = writeln!(conn, "{reply}");
            if is_settings {
                return;
            }
        }
    });
    let mut command = Command::new(env!("CARGO_BIN_EXE_kawaro"));
    command
        .arg("settings")
        .env_clear()
        .env("HOME", dir.path())
        .env("XDG_RUNTIME_DIR", dir.path())
        .env("XDG_CONFIG_HOME", dir.path().join("config"))
        .env("XDG_CACHE_HOME", dir.path().join("cache"))
        .env("PATH", dir.path());
    for (key, value) in vars {
        command.env(key, value);
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "settings failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    seen.lock()
        .unwrap()
        .clone()
        .expect("the daemon received a settings request")
}

#[test]
fn the_wayland_token_wins_over_the_startup_id() {
    let request = settings_request(&[
        ("XDG_ACTIVATION_TOKEN", "wayland-token"),
        ("DESKTOP_STARTUP_ID", "x11-token"),
    ]);
    assert_eq!(
        request,
        Request::Settings {
            token: Some("wayland-token".to_string())
        }
    );
}

#[test]
fn the_startup_id_is_used_when_the_wayland_token_is_absent_or_empty() {
    for vars in [
        vec![("DESKTOP_STARTUP_ID", "x11-token")],
        vec![
            ("XDG_ACTIVATION_TOKEN", ""),
            ("DESKTOP_STARTUP_ID", "x11-token"),
        ],
    ] {
        let request = settings_request(&vars);
        assert_eq!(
            request,
            Request::Settings {
                token: Some("x11-token".to_string())
            },
            "{vars:?}"
        );
    }
}

#[test]
fn no_token_is_sent_when_both_variables_are_absent() {
    assert_eq!(settings_request(&[]), Request::Settings { token: None });
}
