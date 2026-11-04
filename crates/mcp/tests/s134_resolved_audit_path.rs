//! S-134 AC1: Config::resolved_audit_path.
//! Contract pinned here (tester): pub fn resolved_audit_path(&self) -> Option<PathBuf>.
//! None -> <config dir>/mcp-audit.jsonl; absolute as written; "~" and "~/..."
//! expanded with HOME; a relative value against the Kawaro config directory.
//! With HOME unset the tilde is not expanded and the literal path is returned.

use kawaro_mcp::Config;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static ENV: Mutex<()> = Mutex::new(());

struct EnvGuard {
    home: Option<OsString>,
    xdg: Option<OsString>,
}

impl EnvGuard {
    fn set(home: Option<&Path>, xdg: Option<&Path>) -> Self {
        let guard = EnvGuard {
            home: std::env::var_os("HOME"),
            xdg: std::env::var_os("XDG_CONFIG_HOME"),
        };
        match home {
            Some(path) => unsafe { std::env::set_var("HOME", path) },
            None => unsafe { std::env::remove_var("HOME") },
        }
        match xdg {
            Some(path) => unsafe { std::env::set_var("XDG_CONFIG_HOME", path) },
            None => unsafe { std::env::remove_var("XDG_CONFIG_HOME") },
        }
        guard
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match self.home.take() {
            Some(value) => unsafe { std::env::set_var("HOME", value) },
            None => unsafe { std::env::remove_var("HOME") },
        }
        match self.xdg.take() {
            Some(value) => unsafe { std::env::set_var("XDG_CONFIG_HOME", value) },
            None => unsafe { std::env::remove_var("XDG_CONFIG_HOME") },
        }
    }
}

fn with_env(home: Option<&Path>, xdg: Option<&Path>, f: impl FnOnce()) {
    let _lock = ENV.lock().unwrap();
    let _guard = EnvGuard::set(home, xdg);
    f();
}

fn config(path: Option<&str>) -> Config {
    Config {
        audit_path: path.map(PathBuf::from),
        ..Config::default()
    }
}

#[test]
fn ac1_none_resolves_to_the_config_dir_default() {
    let dir = tempfile::tempdir().unwrap();
    let xdg = dir.path().join("xdg");
    let home = dir.path().join("home");
    with_env(Some(&home), Some(&xdg), || {
        assert_eq!(
            config(None).resolved_audit_path(),
            Some(xdg.join("kawaro").join("mcp-audit.jsonl")),
            "the default is <config dir>/mcp-audit.jsonl"
        );
    });
}

#[test]
fn ac1_an_absolute_path_is_used_as_written() {
    let dir = tempfile::tempdir().unwrap();
    let absolute = dir.path().join("state").join("audit.jsonl");
    with_env(Some(dir.path()), Some(dir.path()), || {
        assert_eq!(
            config(Some(absolute.to_str().unwrap())).resolved_audit_path(),
            Some(absolute.clone())
        );
    });
}

#[test]
fn ac1_tilde_paths_expand_with_home() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let xdg = dir.path().join("xdg");
    with_env(Some(&home), Some(&xdg), || {
        assert_eq!(
            config(Some("~")).resolved_audit_path(),
            Some(home.clone()),
            "~ expands to HOME"
        );
        assert_eq!(
            config(Some("~/logs/audit.jsonl")).resolved_audit_path(),
            Some(home.join("logs").join("audit.jsonl"))
        );
    });
}

#[test]
fn ac1_without_home_a_tilde_stays_literal() {
    let dir = tempfile::tempdir().unwrap();
    let xdg = dir.path().join("xdg");
    with_env(None, Some(&xdg), || {
        assert_eq!(
            config(Some("~")).resolved_audit_path(),
            Some(PathBuf::from("~")),
            "no HOME, no expansion"
        );
        assert_eq!(
            config(Some("~/logs/audit.jsonl")).resolved_audit_path(),
            Some(PathBuf::from("~/logs/audit.jsonl"))
        );
    });
}

#[test]
fn ac1_a_relative_path_resolves_against_the_config_directory() {
    let dir = tempfile::tempdir().unwrap();
    let xdg = dir.path().join("xdg");
    with_env(Some(dir.path()), Some(&xdg), || {
        assert_eq!(
            config(Some("logs/audit.jsonl")).resolved_audit_path(),
            Some(xdg.join("kawaro").join("logs").join("audit.jsonl")),
            "a relative path is relative to the Kawaro config directory"
        );
    });
}

#[test]
fn ac1_from_table_plus_resolved_uses_the_default_when_absent() {
    let dir = tempfile::tempdir().unwrap();
    let xdg = dir.path().join("xdg");
    with_env(Some(dir.path()), Some(&xdg), || {
        let table: toml::Table = toml::from_str("[mcp]\nenabled = true\n").unwrap();
        assert_eq!(
            Config::from_table(&table).resolved_audit_path(),
            Some(xdg.join("kawaro").join("mcp-audit.jsonl"))
        );
    });
}
