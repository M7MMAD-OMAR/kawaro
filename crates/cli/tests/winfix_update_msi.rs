//! The Windows update helper entry (the winfix batch, security audit D1):
//! the real `kawaro` binary handling `update --apply-msi <msi>`, the flag
//! `apply_msi` hands to the detached helper copy.
//!
//! Contract (tester): both `--apply-msi FILE` and `--apply-msi=FILE` reach
//! the MSI apply entry (`helper_main` on Windows), never the `update`
//! argument parser, which answers a clap usage error (exit 2). Off Windows
//! the entry refuses with one line naming Windows and exit 1, so the
//! routing is provable on Linux; on Windows the entry itself is compiled
//! and linted by `scripts/windows/check.sh` and run live in S-052.
#![cfg(unix)]

use std::path::Path;
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_kawaro");

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

#[test]
fn apply_msi_with_a_separate_value_reaches_the_msi_entry() {
    let tmp = tempfile::tempdir().unwrap();
    let msi = tmp.path().join("kawaro-0.4.0.msi");
    std::fs::write(&msi, b"msi-bytes").unwrap();
    let out = kawaro(
        tmp.path(),
        &["update", "--apply-msi", msi.to_str().unwrap()],
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_ne!(
        out.status.code(),
        Some(2),
        "not a clap usage error: {stderr}"
    );
    assert_eq!(
        out.status.code(),
        Some(1),
        "the entry is Windows only here: {stderr}"
    );
    assert!(stderr.contains("Windows only"), "{stderr}");
    assert_eq!(std::fs::read(&msi).unwrap(), b"msi-bytes");
}

#[test]
fn apply_msi_with_an_equals_value_reaches_the_msi_entry() {
    let tmp = tempfile::tempdir().unwrap();
    let msi = tmp.path().join("kawaro-0.4.0.msi");
    std::fs::write(&msi, b"msi-bytes").unwrap();
    let flag = format!("--apply-msi={}", msi.display());
    let out = kawaro(tmp.path(), &["update", &flag]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_ne!(
        out.status.code(),
        Some(2),
        "not a clap usage error: {stderr}"
    );
    assert_eq!(
        out.status.code(),
        Some(1),
        "the entry is Windows only here: {stderr}"
    );
    assert!(stderr.contains("Windows only"), "{stderr}");
}

#[test]
fn the_cli_dispatches_apply_msi_to_helper_main() {
    let src =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/main.rs")).unwrap();
    let code: String = src
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        code.contains("helper_main("),
        "main.rs must dispatch `update --apply-msi` to helper_main, the code that runs msiexec and writes the result"
    );
}
