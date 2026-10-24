//! Where the settings, the user actions and the service log live (S-041,
//! ADR 0005 section 4).
//!
//! - Linux: `XDG_CONFIG_HOME/sbarhive`, else `HOME/.config/sbarhive`; the
//!   log stays on stderr, so there is no log folder.
//! - Windows: `%APPDATA%\SbarHive` and `%LOCALAPPDATA%\SbarHive\logs`; when
//!   the variable is missing or empty, the known folder from
//!   `SHGetKnownFolderPath` (`paths/windows.rs`).
//!
//! The choice is pure: the platform, the environment and the known folder
//! are parameters, so the Windows branch is tested and mutated on Linux.

use std::ffi::OsString;
use std::path::PathBuf;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::log_file;

/// The platforms whose folders differ.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
    Linux,
    Windows,
}

/// The running platform.
pub const CURRENT: Os = if cfg!(windows) {
    Os::Windows
} else {
    Os::Linux
};

/// Reads one environment variable, in the shape of `std::env::var_os`.
pub type EnvFn<'a> = &'a dyn Fn(&str) -> Option<OsString>;
/// Gives a known folder of the system, if any.
pub type KnownFn<'a> = &'a dyn Fn() -> Option<PathBuf>;

/// A Windows folder from `var`, or from `known` when it is missing or empty.
fn windows_base(env: EnvFn, var: &str, known: KnownFn) -> Option<PathBuf> {
    env(var)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(known)
}

/// The settings folder: `config.toml` and `actions/` live in it.
pub fn config_dir_with(os: Os, env: EnvFn, known: KnownFn) -> Option<PathBuf> {
    match os {
        Os::Linux => env("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| env("HOME").map(|h| PathBuf::from(h).join(".config")))
            .map(|base| base.join("sbarhive")),
        Os::Windows => windows_base(env, "APPDATA", known).map(|base| base.join("SbarHive")),
    }
}

/// The service log folder; `None` on Linux, where the log is stderr.
pub fn log_dir_with(os: Os, env: EnvFn, known: KnownFn) -> Option<PathBuf> {
    match os {
        Os::Linux => None,
        Os::Windows => {
            windows_base(env, "LOCALAPPDATA", known).map(|base| base.join("SbarHive").join("logs"))
        }
    }
}

/// [`config_dir_with`] for the running platform and process environment.
pub fn config_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    let known = windows::roaming_app_data;
    #[cfg(not(windows))]
    let known = || None;
    config_dir_with(CURRENT, &|k| std::env::var_os(k), &known)
}

/// The service log file: [`log_dir_with`] joined with `sbarhived.log`.
pub fn log_file_with(os: Os, env: EnvFn, known: KnownFn) -> Option<PathBuf> {
    log_dir_with(os, env, known).map(|dir| dir.join("sbarhived.log"))
}

/// The cache folder: downloaded updates live under it (S-053b).
pub fn cache_dir_with(os: Os, env: EnvFn, known: KnownFn) -> Option<PathBuf> {
    match os {
        Os::Linux => env("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| env("HOME").map(|h| PathBuf::from(h).join(".cache")))
            .map(|base| base.join("sbarhive")),
        Os::Windows => windows_base(env, "LOCALAPPDATA", known).map(|base| base.join("SbarHive")),
    }
}

/// Where the package of one version is kept: `<cache>/updates/<version>`.
pub fn update_dir_with(os: Os, env: EnvFn, known: KnownFn, version: &str) -> Option<PathBuf> {
    cache_dir_with(os, env, known).map(|dir| dir.join("updates").join(version))
}

/// The check schedule file, next to `config.toml`.
pub fn update_state_file_with(os: Os, env: EnvFn, known: KnownFn) -> Option<PathBuf> {
    config_dir_with(os, env, known).map(|dir| dir.join("update-state.json"))
}
