//! S-041 AC1, AC2 and AC3 (backlog/S-041-windows-paths.md): where the
//! settings, the user actions and the service log live, per platform.
//!
//! Contract fixed by the tester (the implementer may not change this file):
//!
//! - `sbarhive_core::paths` is a `pub mod` on every platform.
//! - `pub enum Os { Linux, Windows }` (derives at least `Clone, Copy`).
//! - `pub fn config_dir_with(os: Os, env: E, known: K) -> Option<PathBuf>`
//!   and `pub fn log_dir_with(os: Os, env: E, known: K) -> Option<PathBuf>`,
//!   where `env` reads one variable as `Fn(&str) -> Option<OsString>` (the
//!   shape of `std::env::var_os`) and `known` gives the known folder as
//!   `Fn() -> Option<PathBuf>`. The tests pass both by reference
//!   (`&|k| ...`), so `&dyn Fn` and generic parameters both compile.
//! - `pub fn config_dir() -> Option<PathBuf>`: `config_dir_with` with the
//!   running platform, the process environment and the real known folder.
//! - `Config::actions_dir() -> Option<PathBuf>`, next to `Config::path()`.
//!
//! The table asks for unit tests inside `paths.rs`; that file is the
//! implementer's, so the tests live here and use the public API. Expected
//! values are built with `Path::join`, never with a `\` in a string, because
//! Linux joins with `/`. These are the only tests in this binary and none
//! of them changes the process environment.

use sbarhive_core::config::Config;
use sbarhive_core::paths::{Os, config_dir, config_dir_with, log_dir_with};
use std::cell::Cell;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// A fake environment holding only `pairs`.
fn env_of(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> + use<> {
    let owned: Vec<(String, OsString)> = pairs
        .iter()
        .map(|(k, v)| (k.to_string(), OsString::from(v)))
        .collect();
    move |k: &str| {
        owned
            .iter()
            .find(|(name, _)| name == k)
            .map(|(_, v)| v.clone())
    }
}

fn known_dir() -> PathBuf {
    PathBuf::from("known").join("Roaming")
}

// AC1, Windows.

#[test]
fn ac1_windows_appdata_wins_and_known_is_not_called() {
    let calls = Cell::new(0);
    let known = || {
        calls.set(calls.get() + 1);
        Some(known_dir())
    };
    let env = env_of(&[("APPDATA", "A"), ("XDG_CONFIG_HOME", "X"), ("HOME", "H")]);
    assert_eq!(
        config_dir_with(Os::Windows, &env, &known),
        Some(Path::new("A").join("SbarHive"))
    );
    assert_eq!(
        calls.get(),
        0,
        "known must not be called when APPDATA is set"
    );
}

#[test]
fn ac1_windows_empty_appdata_falls_back_to_the_known_folder() {
    let calls = Cell::new(0);
    let known = || {
        calls.set(calls.get() + 1);
        Some(known_dir())
    };
    let env = env_of(&[("APPDATA", "")]);
    assert_eq!(
        config_dir_with(Os::Windows, &env, &known),
        Some(known_dir().join("SbarHive"))
    );
    assert_eq!(calls.get(), 1);
}

#[test]
fn ac1_windows_missing_appdata_falls_back_to_the_known_folder() {
    let calls = Cell::new(0);
    let known = || {
        calls.set(calls.get() + 1);
        Some(known_dir())
    };
    let env = env_of(&[]);
    assert_eq!(
        config_dir_with(Os::Windows, &env, &known),
        Some(known_dir().join("SbarHive"))
    );
    assert_eq!(calls.get(), 1);
}

#[test]
fn ac1_windows_ignores_xdg_and_home() {
    // Without APPDATA, Windows asks the known folder; the Linux variables
    // must not leak into the Windows branch.
    let env = env_of(&[("XDG_CONFIG_HOME", "X"), ("HOME", "H")]);
    assert_eq!(
        config_dir_with(Os::Windows, &env, &|| Some(known_dir())),
        Some(known_dir().join("SbarHive"))
    );
}

#[test]
fn ac1_windows_without_appdata_and_without_known_folder_is_none() {
    assert_eq!(config_dir_with(Os::Windows, &env_of(&[]), &|| None), None);
    assert_eq!(
        config_dir_with(Os::Windows, &env_of(&[("APPDATA", "")]), &|| None),
        None
    );
}

#[test]
fn ac1_windows_appdata_is_not_read_as_localappdata() {
    let env = env_of(&[("LOCALAPPDATA", "L")]);
    assert_eq!(
        config_dir_with(Os::Windows, &env, &|| Some(known_dir())),
        Some(known_dir().join("SbarHive"))
    );
}

// AC1, Linux: today's behaviour.

#[test]
fn ac1_linux_xdg_config_home_first() {
    let calls = Cell::new(0);
    let known = || {
        calls.set(calls.get() + 1);
        Some(known_dir())
    };
    let env = env_of(&[("XDG_CONFIG_HOME", "X"), ("HOME", "H"), ("APPDATA", "A")]);
    assert_eq!(
        config_dir_with(Os::Linux, &env, &known),
        Some(Path::new("X").join("sbarhive"))
    );
    assert_eq!(calls.get(), 0, "Linux never asks the known folder");
}

#[test]
fn ac1_linux_home_dot_config_second() {
    let calls = Cell::new(0);
    let known = || {
        calls.set(calls.get() + 1);
        Some(known_dir())
    };
    let env = env_of(&[("HOME", "H"), ("APPDATA", "A")]);
    assert_eq!(
        config_dir_with(Os::Linux, &env, &known),
        Some(Path::new("H").join(".config").join("sbarhive"))
    );
    assert_eq!(calls.get(), 0, "Linux never asks the known folder");
}

#[test]
fn ac1_linux_without_xdg_and_home_is_none_even_with_appdata() {
    let calls = Cell::new(0);
    let known = || {
        calls.set(calls.get() + 1);
        Some(known_dir())
    };
    let env = env_of(&[("APPDATA", "A"), ("LOCALAPPDATA", "L")]);
    assert_eq!(config_dir_with(Os::Linux, &env, &known), None);
    assert_eq!(calls.get(), 0, "Linux never asks the known folder");
}

// AC2.

#[test]
fn ac2_windows_log_dir_is_localappdata_sbarhive_logs_without_known() {
    let calls = Cell::new(0);
    let known = || {
        calls.set(calls.get() + 1);
        Some(known_dir())
    };
    let env = env_of(&[("LOCALAPPDATA", "L"), ("APPDATA", "A")]);
    assert_eq!(
        log_dir_with(Os::Windows, &env, &known),
        Some(Path::new("L").join("SbarHive").join("logs"))
    );
    assert_eq!(
        calls.get(),
        0,
        "known must not be called when LOCALAPPDATA is set"
    );
}

#[test]
fn ac2_windows_empty_or_missing_localappdata_falls_back_to_the_known_folder() {
    // APPDATA alone must not stand in for LOCALAPPDATA.
    for env in [
        env_of(&[("LOCALAPPDATA", ""), ("APPDATA", "A")]),
        env_of(&[("APPDATA", "A")]),
        env_of(&[]),
    ] {
        let calls = Cell::new(0);
        let known = || {
            calls.set(calls.get() + 1);
            Some(PathBuf::from("known").join("Local"))
        };
        assert_eq!(
            log_dir_with(Os::Windows, &env, &known),
            Some(
                PathBuf::from("known")
                    .join("Local")
                    .join("SbarHive")
                    .join("logs")
            )
        );
        assert_eq!(calls.get(), 1);
    }
}

#[test]
fn ac2_windows_without_localappdata_and_without_known_folder_is_none() {
    assert_eq!(log_dir_with(Os::Windows, &env_of(&[]), &|| None), None);
    assert_eq!(
        log_dir_with(Os::Windows, &env_of(&[("LOCALAPPDATA", "")]), &|| None),
        None
    );
}

#[test]
fn ac2_linux_has_no_log_dir() {
    // The log stays on stderr on Linux, whatever the environment holds.
    let env = env_of(&[
        ("LOCALAPPDATA", "L"),
        ("XDG_CONFIG_HOME", "X"),
        ("XDG_STATE_HOME", "S"),
        ("HOME", "H"),
    ]);
    assert_eq!(log_dir_with(Os::Linux, &env, &|| Some(known_dir())), None);
}

// AC3: the real entry points agree with config_dir().

// Linux only: XDG_CONFIG_HOME and HOME do not exist on Windows (APPDATA is covered by the Windows branch tests).

#[cfg(unix)]
#[test]
fn ac3_config_dir_follows_the_process_environment_on_linux() {
    // No env mutation: the expected value is read the way today's Linux
    // code reads it, then compared with the public entry point.
    let expected = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .map(|b| b.join("sbarhive"));
    assert!(expected.is_some(), "test needs XDG_CONFIG_HOME or HOME");
    assert_eq!(config_dir(), expected);
}

#[test]
fn ac3_config_path_is_config_dir_joined_with_config_toml() {
    let dir = config_dir();
    assert!(dir.is_some(), "test needs XDG_CONFIG_HOME or HOME");
    assert_eq!(Config::path(), dir.map(|d| d.join("config.toml")));
}

#[test]
fn ac3_actions_dir_is_config_dir_joined_with_actions() {
    let dir = config_dir();
    assert!(dir.is_some(), "test needs XDG_CONFIG_HOME or HOME");
    assert_eq!(Config::actions_dir(), dir.map(|d| d.join("actions")));
}
