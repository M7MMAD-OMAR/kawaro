//! S-102 (backlog/S-102-alt-doctor-and-docs.md), AC1 and AC2 at the CLI.
//!
//! Contract (tester, from the acceptance criteria): with no daemon, no
//! Hyprland bind and no hold ever recorded, `kawaro doctor` prints exactly
//! one line `drop key: <key> bind missing, daemon down, last hold: none`
//! (the format of `kawaro_core::hold::doctor_line`). `kawaro drop --hold`
//! with no daemon that can be reached or started prints the line
//! `hold: daemon-down` on stderr and exits non zero. The `no-action-file`
//! emission and the states with a running daemon need a status source the
//! story leaves to the implementer; their tests are added with it.
#![cfg(unix)]

use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_kawaro");

fn run(args: &[&str]) -> std::process::Output {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("bin")).unwrap();
    Command::new(BIN)
        .args(args)
        .env_clear()
        .env("HOME", dir.path())
        .env("XDG_RUNTIME_DIR", dir.path())
        .env("PATH", dir.path().join("bin"))
        .env("LANG", "en_US.UTF-8")
        .output()
        .unwrap()
}

#[test]
fn ac1_doctor_prints_one_drop_key_line_for_the_no_daemon_state() {
    let o = run(&["doctor"]);
    let text = String::from_utf8_lossy(&o.stdout).into_owned();
    let lines: Vec<&str> = text
        .lines()
        .filter(|l| l.starts_with("drop key: "))
        .collect();
    assert_eq!(lines.len(), 1, "one drop key line expected in:\n{text}");
    assert!(
        lines[0].ends_with(" bind missing, daemon down, last hold: none"),
        "{}",
        lines[0]
    );
}

#[test]
fn ac2_drop_hold_without_a_daemon_logs_hold_daemon_down() {
    let o = run(&["drop", "--hold"]);
    let err = String::from_utf8_lossy(&o.stderr).into_owned();
    assert!(!o.status.success());
    assert!(
        err.lines().any(|l| l.trim() == "hold: daemon-down"),
        "{err}"
    );
}
