//! S-054b (backlog/S-054b-update-daemon-tray.md), the core side: the IPC
//! messages (AC1) and the review rulings carried into the story. Written by
//! the tester from the acceptance criteria and the review log.
//!
//! Contract (the implementer follows it; the tests are not edited). Where the
//! story leaves a gap the choice is marked DECISION.
//!
//! `ipc` (`crates/core/src/ipc.rs`):
//! - `Request::UpdateCheck` and `Request::UpdateApply` (unit variants).
//! - `Response::Update { status: String, current: String, latest: Option<String> }`.
//!   DECISION (deviation from the AC text): `Response` is internally tagged
//!   with `"status"`, so a field also called `status` would write the key
//!   twice and could never be read back. The Rust field keeps the name
//!   `status` (as the AC says); its JSON key is `state`
//!   (`#[serde(rename = "state")]`). Exact line:
//!   `{"status":"update","state":"ready","current":"0.3.3","latest":"0.4.0"}`.
//!
//! Beta default channel (manager ruling, S-053a). DECISION: the least
//! invasive way that keeps `s053a_update.rs` and `s053b_update_fetch.rs`
//! passing is to leave `Config.update_channel: Channel` alone (making it an
//! `Option<Channel>` breaks the struct literals and the `== Channel::Stable`
//! asserts there) and to read key presence from the file text:
//! - `Config::explicit_update_channel(text: &str) -> Option<Channel>`: the
//!   value of `update_channel` in a config file text; `None` when the key is
//!   absent, the value is not `stable` or `beta`, or the text is not TOML.
//! - `update::effective_channel(configured: Option<Channel>, installed: &str)
//!   -> Channel`: the configured channel when there is one, otherwise
//!   `default_channel(installed)`, and `Stable` when `installed` is not a
//!   version.
//!   Known limit, for the manager: `Config::save_to` writes every field, so
//!   after the first save from the settings window the key is present and the
//!   channel counts as explicit.
//!
//! Curl environment (manager ruling, S-053b):
//! - `fetch::curl_env(os: Os, env: EnvFn) -> Vec<(String, String)>`: the only
//!   variables `CurlFetcher::get` passes (after `env_clear`). Linux:
//!   `[("LC_ALL", "C")]`. Windows: `[("SystemRoot", <value>)]` when
//!   `SystemRoot` is set and not empty, otherwise empty.
//!
//! Privileged install (manager ruling, S-054a):
//! - `apply::PKEXEC: &str = "/usr/bin/pkexec"`; `privileged_argv` starts with it.
//! - `apply::exe_is_trusted(owner_uid: u32, mode: u32) -> bool`: owner is root
//!   and the mode has no group or other write bit.
//! - `apply::privileged_argv_checked(exe, file, sha256, kind) ->
//!   Result<Vec<String>, UpdateError>` (unix): `Err(UpdateError::Io)` unless
//!   the whole path of `exe` is trusted (`apply::path_is_trusted`: an
//!   absolute name of a plain file under real directories that all pass
//!   `exe_is_trusted`; a symlink anywhere refuses), otherwise
//!   `privileged_argv`'s result.
//!   `apply_now` stays as it is (its tests use fake exe paths).
//!
//! Windows quit (manager ruling, S-054a): `apply/windows.rs` sends
//! `Request::Quit` over IPC after it starts the helper copy. Source check
//! only, the file builds for Windows only.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use kawaro_core::config::Config;
use kawaro_core::ipc::{Request, Response};
use kawaro_core::paths::Os;
use kawaro_core::update::apply::{PKEXEC, PkgKind, exe_is_trusted, privileged_argv};
use kawaro_core::update::fetch::curl_env;
use kawaro_core::update::{Channel, effective_channel};

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

// ------------------------------------------------------------ AC1

#[test]
fn ac1_update_requests_serialise_to_the_exact_lines() {
    assert_eq!(
        serde_json::to_string(&Request::UpdateCheck).unwrap(),
        r#"{"cmd":"update-check"}"#
    );
    assert_eq!(
        serde_json::to_string(&Request::UpdateApply).unwrap(),
        r#"{"cmd":"update-apply"}"#
    );
}

#[test]
fn ac1_update_requests_round_trip_and_ping_is_unchanged() {
    for r in [Request::UpdateCheck, Request::UpdateApply, Request::Ping] {
        let line = serde_json::to_string(&r).unwrap();
        assert!(!line.contains('\n'));
        assert_eq!(serde_json::from_str::<Request>(&line).unwrap(), r);
    }
    assert_eq!(
        serde_json::to_string(&Request::Ping).unwrap(),
        r#"{"cmd":"ping"}"#
    );
    assert_eq!(
        serde_json::from_str::<Request>(r#"{"cmd":"update-check"}"#).unwrap(),
        Request::UpdateCheck
    );
    assert_eq!(
        serde_json::from_str::<Request>(r#"{"cmd":"update-apply"}"#).unwrap(),
        Request::UpdateApply
    );
}

#[test]
fn ac1_update_response_serialises_with_status_update_and_round_trips() {
    let full = Response::Update {
        status: "ready".to_string(),
        current: "0.3.3".to_string(),
        latest: Some("0.4.0".to_string()),
        percent: None,
        size: None,
        notes: None,
    };
    let line = serde_json::to_string(&full).unwrap();
    assert_eq!(
        line,
        r#"{"status":"update","state":"ready","current":"0.3.3","latest":"0.4.0"}"#
    );
    assert_eq!(serde_json::from_str::<Response>(&line).unwrap(), full);

    let none = Response::Update {
        status: "offline".to_string(),
        current: "0.3.3".to_string(),
        latest: None,
        percent: None,
        size: None,
        notes: None,
    };
    let line = serde_json::to_string(&none).unwrap();
    assert_eq!(
        line,
        r#"{"status":"update","state":"offline","current":"0.3.3","latest":null}"#
    );
    assert_eq!(serde_json::from_str::<Response>(&line).unwrap(), none);
}

#[test]
fn ac1_the_other_responses_are_unchanged() {
    assert_eq!(
        serde_json::to_string(&Response::Ok).unwrap(),
        r#"{"status":"ok"}"#
    );
    assert_eq!(
        serde_json::to_string(&Response::Error {
            message: "no verified update".to_string()
        })
        .unwrap(),
        r#"{"status":"error","message":"no verified update"}"#
    );
}

// ------------------------------------------------------------ channel ruling

#[test]
fn explicit_channel_is_read_from_the_key_presence() {
    assert_eq!(Config::explicit_update_channel(""), None);
    assert_eq!(Config::explicit_update_channel("hold_ms = 500\n"), None);
    assert_eq!(
        Config::explicit_update_channel("update_channel = \"beta\"\n"),
        Some(Channel::Beta)
    );
    assert_eq!(
        Config::explicit_update_channel("hold_ms = 500\nupdate_channel = \"stable\"\n"),
        Some(Channel::Stable)
    );
    assert_eq!(
        Config::explicit_update_channel("update_channel = \"nightly\"\n"),
        None
    );
    assert_eq!(
        Config::explicit_update_channel("update_channel = 3\n"),
        None
    );
    assert_eq!(Config::explicit_update_channel("this is [not toml"), None);
}

#[test]
fn effective_channel_defaults_to_beta_only_for_a_prerelease_without_a_setting() {
    assert_eq!(effective_channel(None, "0.4.0-beta.1"), Channel::Beta);
    assert_eq!(effective_channel(None, "1.0.0-rc.1"), Channel::Beta);
    assert_eq!(effective_channel(None, "0.3.3"), Channel::Stable);
    assert_eq!(effective_channel(None, "0.4.0"), Channel::Stable);
    assert_eq!(
        effective_channel(Some(Channel::Stable), "0.4.0-beta.1"),
        Channel::Stable
    );
    assert_eq!(
        effective_channel(Some(Channel::Beta), "0.3.3"),
        Channel::Beta
    );
    assert_eq!(effective_channel(None, "not a version"), Channel::Stable);
}

// ------------------------------------------------------------ curl environment

fn env_with_root(k: &str) -> Option<OsString> {
    (k == "SystemRoot").then(|| OsString::from("C:\\Windows"))
}

#[test]
fn curl_env_is_minimal_on_each_platform() {
    let none = |_: &str| None;
    assert_eq!(curl_env(Os::Linux, &none), s_pairs(&[("LC_ALL", "C")]));
    // The caller's variables never leak in, even when they are set.
    let proxies = |k: &str| {
        matches!(
            k,
            "https_proxy" | "HTTPS_PROXY" | "CURL_CA_BUNDLE" | "SSL_CERT_FILE" | "HOME" | "PATH"
        )
        .then(|| OsString::from("x"))
    };
    assert_eq!(curl_env(Os::Linux, &proxies), s_pairs(&[("LC_ALL", "C")]));
    assert_eq!(
        curl_env(Os::Windows, &env_with_root),
        s_pairs(&[("SystemRoot", "C:\\Windows")])
    );
    assert_eq!(curl_env(Os::Windows, &none), Vec::new());
    let empty = |k: &str| (k == "SystemRoot").then(OsString::new);
    assert_eq!(curl_env(Os::Windows, &empty), Vec::new());
}

fn s_pairs(v: &[(&str, &str)]) -> Vec<(String, String)> {
    v.iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}

// ------------------------------------------------------------ pkexec ruling

#[test]
fn privileged_argv_uses_the_absolute_pkexec() {
    assert_eq!(PKEXEC, "/usr/bin/pkexec");
    let argv = privileged_argv(
        Path::new("/usr/bin/kawaro"),
        Path::new("/c/kawaro.rpm"),
        "ab",
        PkgKind::Rpm,
    );
    assert_eq!(
        argv,
        s(&[
            "/usr/bin/pkexec",
            "/usr/bin/kawaro",
            "update",
            "--install-privileged",
            "/c/kawaro.rpm",
            "--sha256",
            "ab",
            "--kind",
            "rpm"
        ])
    );
}

#[test]
fn the_exe_must_be_root_owned_and_not_writable_by_others() {
    assert!(exe_is_trusted(0, 0o755));
    assert!(exe_is_trusted(0, 0o100755));
    assert!(exe_is_trusted(0, 0o4755));
    assert!(exe_is_trusted(0, 0o500));
    assert!(!exe_is_trusted(0, 0o775));
    assert!(!exe_is_trusted(0, 0o757));
    assert!(!exe_is_trusted(0, 0o777));
    assert!(!exe_is_trusted(0, 0o722));
    assert!(!exe_is_trusted(1000, 0o755));
    assert!(!exe_is_trusted(1000, 0o700));
    assert!(!exe_is_trusted(1, 0o755));
}

// ------------------------------------------------------------ Windows quit

fn code_lines(rel: &str) -> String {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} must exist: {e}", path.display()))
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn apply_msi_asks_the_daemon_to_quit_by_ipc_after_starting_the_helper() {
    let src = code_lines("src/update/apply/windows.rs");
    let spawn = src.find(".spawn()").expect("apply_msi starts the helper");
    let quit = src
        .find("Request::Quit")
        .expect("apply_msi must send Request::Quit over IPC");
    assert!(
        spawn < quit,
        "the helper starts before the daemon is asked to quit"
    );
    assert!(src.contains("Stream::connect"));
}
