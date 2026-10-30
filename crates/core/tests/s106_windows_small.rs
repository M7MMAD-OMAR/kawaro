//! S-106 (backlog/S-106-windows-small-defects.md): A2 to A7, from the
//! acceptance criteria only.
//!
//! Hooks pinned here:
//! - new module `kawaro_core::update::cleanup` (file
//!   `crates/core/src/update/cleanup.rs`):
//!   - `stale_update_dirs(entries: &[&str], installed: &str) -> Vec<String>`
//!     = the folder names whose version parses and is <= installed, in input
//!     order; non version names and newer ones are kept (not returned);
//!   - `stale_helper_dirs(entries: &[(&str, u64)], now: u64, max_age: u64)
//!     -> Vec<String>`: entries are (folder name, modified time in seconds);
//!     returns names starting with `kawaro-update-helper-` and strictly
//!     older than `max_age`;
//!   - `clean_updates(updates: &Path, installed: &str)`: removes the stale
//!     version folders under `updates` (result ignored here).
//! - `HelperOps::cleanup_download(&self, msi: &Path, log: &Path)`, a new
//!   required method; `run_helper` calls it exactly once after
//!   `Outcome::Installed` and never after `Failed`.
//! - `kawaro_core::identity::shortcut_properties()`: the list of
//!   (property key, value) pairs set on both shortcuts.

use kawaro_core::appid::APP_ID;
use kawaro_core::identity;
use kawaro_core::update::UpdateError;
use kawaro_core::update::apply::{Daemon, HelperOps, run_helper};
use kawaro_core::update::cleanup::{clean_updates, stale_helper_dirs, stale_update_dirs};
use kawaro_core::update::result::{Outcome, UpdateResult};
use std::cell::RefCell;
use std::path::Path;
use std::time::Duration;

// ------------------------------------------------------------ A2

// ------------------------------------------------------------ A3

#[test]
fn s106_a3_stale_update_dirs_keeps_newer_and_non_version_names() {
    let entries = ["0.4.2", "0.4.3", "0.4.4", "0.5.0", "notes.txt", "0.4.4-x"];
    assert_eq!(
        stale_update_dirs(&entries, "0.4.4"),
        ["0.4.2", "0.4.3", "0.4.4"]
    );
}

#[test]
fn s106_a3_installed_0_4_1_returns_nothing() {
    let entries = ["0.4.2", "0.4.3", "0.4.4", "0.5.0", "notes.txt", "0.4.4-x"];
    assert!(stale_update_dirs(&entries, "0.4.1").is_empty());
}

// ------------------------------------------------------------ A4

fn total_bytes(dir: &Path) -> u64 {
    let mut n = 0;
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            n += total_bytes(&p);
        } else {
            n += std::fs::metadata(&p).unwrap().len();
        }
    }
    n
}

#[test]
fn s106_a4_cleanup_leaves_only_the_newer_version_bytes() {
    let tmp = tempfile::tempdir_in(
        std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap(),
    )
    .unwrap();
    let updates = tmp.path().join("updates");
    for (v, mb) in [("0.4.2", 1), ("0.4.3", 1), ("0.4.4", 1), ("0.5.0", 5)] {
        let d = updates.join(v);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("kawaro.msi"), vec![7u8; mb * 1024 * 1024]).unwrap();
    }
    let _ = clean_updates(&updates, "0.4.4");
    assert_eq!(total_bytes(&updates), 5 * 1024 * 1024);
    for v in ["0.4.2", "0.4.3", "0.4.4"] {
        assert!(
            !updates.join(v).join("kawaro.msi").exists(),
            "{v} content must be gone"
        );
    }
    assert!(updates.join("0.5.0").join("kawaro.msi").exists());
}

#[derive(Default)]
struct Fake {
    events: RefCell<Vec<String>>,
    code: i32,
}

impl HelperOps for Fake {
    fn wait_for_daemon_exit(&self, _t: Duration) -> bool {
        true
    }
    fn run_msiexec(&self, _a: &[String]) -> Result<i32, UpdateError> {
        self.events.borrow_mut().push("install".into());
        Ok(self.code)
    }
    fn daemon_present(&self) -> bool {
        true
    }
    fn write_result(&self, _r: &UpdateResult) -> Result<(), UpdateError> {
        Ok(())
    }
    fn start_daemon(&self, _w: Daemon) {}
    fn cleanup_download(&self, _msi: &Path, _log: &Path) {
        self.events.borrow_mut().push("cleanup".into());
    }
}

fn helper(code: i32) -> (UpdateResult, Vec<String>) {
    let fake = Fake {
        code,
        ..Default::default()
    };
    let r = run_helper(
        &fake,
        Path::new("C:/Cache/updates/0.4.5/kawaro.msi"),
        Path::new("C:/Cache/updates/0.4.5/kawaro.log"),
        "0.4.5",
        1,
    );
    let ev = fake.events.borrow().clone();
    (r, ev)
}

#[test]
fn s106_a4_helper_removes_the_download_after_installed() {
    let (r, ev) = helper(0);
    assert_eq!(r.outcome, Outcome::Installed);
    assert_eq!(ev.iter().filter(|e| *e == "cleanup").count(), 1);
}

#[test]
fn s106_a4_helper_keeps_the_download_after_failed() {
    let (r, ev) = helper(1603);
    assert!(matches!(r.outcome, Outcome::Failed(_)));
    assert!(!ev.iter().any(|e| e == "cleanup"));
}

// ------------------------------------------------------------ A5

#[test]
fn s106_a5_only_helper_folders_older_than_600_seconds_are_returned() {
    let now = 100_000;
    let entries = [
        ("kawaro-update-helper-111", now - 599),
        ("kawaro-update-helper-222", now - 601),
        ("somebody-else", now - 99_999),
    ];
    assert_eq!(
        stale_helper_dirs(&entries, now, 600),
        ["kawaro-update-helper-222"]
    );
}

// ------------------------------------------------------------ A6

#[test]
fn s106_a6_shortcut_properties_are_the_aumid_from_app_id() {
    let got: Vec<(String, String)> = identity::shortcut_properties()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    assert_eq!(
        got,
        [("System.AppUserModel.ID".to_string(), APP_ID.to_string())]
    );
    assert_eq!(APP_ID, "io.github.m7mmad_omar.Kawaro");
}

// ------------------------------------------------------------ A7
