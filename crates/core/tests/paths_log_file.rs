//! S-045 AC6 (backlog/S-045-windows-cfg-gates.md): `paths::log_file_with` is
//! pure and equals `log_dir_with(...)` joined with `kawarod.log`.

use kawaro_core::paths::{Os, log_dir_with, log_file_with};
use std::ffi::OsString;
use std::path::PathBuf;

const LOCAL: &str = r"C:\Users\u\AppData\Local";

fn env_local(name: &str) -> Option<OsString> {
    (name == "LOCALAPPDATA").then(|| OsString::from(LOCAL))
}

fn env_none(_: &str) -> Option<OsString> {
    None
}

fn known_ok() -> Option<PathBuf> {
    Some(PathBuf::from(r"D:\Known\Local"))
}

fn known_fail() -> Option<PathBuf> {
    None
}

#[test]
fn ac6_windows_with_localappdata_gives_the_logs_file() {
    let got = log_file_with(Os::Windows, &env_local, &known_fail).unwrap();
    let want = PathBuf::from(LOCAL)
        .join("Kawaro")
        .join("logs")
        .join("kawarod.log");
    assert_eq!(got, want);
    assert_eq!(got.file_name().unwrap(), "kawarod.log");
}

#[test]
fn ac6_log_file_is_log_dir_joined_with_the_name() {
    let dir = log_dir_with(Os::Windows, &env_local, &known_fail).unwrap();
    let file = log_file_with(Os::Windows, &env_local, &known_fail).unwrap();
    assert_eq!(file, dir.join("kawarod.log"));
}

#[test]
fn ac6_missing_variable_falls_back_to_known() {
    let got = log_file_with(Os::Windows, &env_none, &known_ok).unwrap();
    let want = PathBuf::from(r"D:\Known\Local")
        .join("Kawaro")
        .join("logs")
        .join("kawarod.log");
    assert_eq!(got, want);
}

#[test]
fn ac6_none_when_variable_and_known_both_fail() {
    assert_eq!(log_file_with(Os::Windows, &env_none, &known_fail), None);
}

#[test]
fn ac6_linux_has_no_log_file_like_log_dir() {
    assert_eq!(
        log_file_with(Os::Linux, &env_local, &known_ok),
        log_dir_with(Os::Linux, &env_local, &known_ok).map(|d| d.join("kawarod.log"))
    );
}
