//! S-067 (backlog/S-067-review-fixes-update-drop-notify.md), the real
//! `kawaro` binary in a temp HOME: AC7 (a broken `config.toml` is reported,
//! not swallowed) and AC8 (`--install-privileged=FILE` reaches the
//! privileged step). No network: `--status` reads local files only, and the
//! privileged step refuses a normal user before it looks at the file. The
//! AC8 test is skipped as root, where the command would really install.
//!
//! Contract (tester): for a `config.toml` that does not parse, stderr is
//! exactly one line that names `config.toml`; stdout stays `status none`.
#![cfg(unix)]

use std::path::Path;
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_kawaro");
const SHA: &str = "bff5b3ca49e18ea61c9f21a39f95922bcf75f2e0f8ffedf46e07c7cfd70d5d64";

fn kawaro(home: &Path, args: &[&str]) -> Output {
    Command::new(BIN)
        .args(args)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_CACHE_HOME", home.join("cache"))
        .env("LANG", "C")
        .env_remove("LC_ALL")
        .env_remove("LC_MESSAGES")
        .env_remove("LANGUAGE")
        .output()
        .unwrap()
}

fn euid() -> u32 {
    let out = Command::new("id").arg("-u").output().unwrap();
    String::from_utf8_lossy(&out.stdout).trim().parse().unwrap()
}

#[test]
fn ac7_status_with_a_broken_config_warns_once_and_still_answers_none() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("config").join("kawaro");
    std::fs::create_dir_all(&dir).unwrap();
    // Not TOML, and a toml error spans several lines: the warning must not.
    std::fs::write(dir.join("config.toml"), "lang = [unclosed\nquality = \n").unwrap();
    let out = kawaro(tmp.path(), &["update", "--status"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(String::from_utf8_lossy(&out.stdout), "status none\n");
    assert_eq!(out.status.code(), Some(0), "stderr {stderr:?}");
    assert!(
        stderr.ends_with('\n') && stderr.matches('\n').count() == 1,
        "the config warning is one stderr line, got {stderr:?}"
    );
    assert!(stderr.contains("config.toml"), "names the file: {stderr:?}");
}

#[test]
fn ac8_install_privileged_equals_file_is_routed_to_the_privileged_step() {
    if euid() == 0 {
        eprintln!("skipped: running as root");
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("kawaro-0.4.0-1.x86_64.rpm");
    std::fs::write(&file, b"first-asset-bytes").unwrap();
    let flag = format!("--install-privileged={}", file.display());
    let out = kawaro(
        tmp.path(),
        &["update", &flag, "--sha256", SHA, "--kind", "rpm"],
    );
    assert_eq!(
        out.status.code(),
        Some(77),
        "a normal user gets the root refusal, not a usage error: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(std::fs::read(&file).unwrap(), b"first-asset-bytes");
}
