//! S-054c AC1 (backlog/S-054c-update-cli.md), the real binary:
//! `kawaro update --status` in an empty config dir prints `status none` and
//! exits 0. `--status` reads only local files, so no network is involved.
//! Every base folder points into a fresh temp dir.
#![cfg(unix)]

use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_kawaro");

#[test]
fn ac1_the_real_binary_status_in_an_empty_config_dir_prints_status_none() {
    let tmp = tempfile::tempdir().unwrap();
    let out = Command::new(BIN)
        .args(["update", "--status"])
        .env("HOME", tmp.path())
        .env("XDG_CONFIG_HOME", tmp.path().join("config"))
        .env("XDG_CACHE_HOME", tmp.path().join("cache"))
        .env("LANG", "C")
        .env_remove("LC_ALL")
        .env_remove("LC_MESSAGES")
        .env_remove("LANGUAGE")
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "status none\n",
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(
        std::fs::read_dir(tmp.path()).unwrap().count(),
        0,
        "--status writes nothing"
    );
}
