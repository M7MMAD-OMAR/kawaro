//! S-108 (backlog/S-108-s096b-hive-cells.md): the structural checks of the
//! daemon side, each the story's scan with its expected count. They read the
//! workspace sources, so they run without compiling the daemon tests.
//! Needles are split with `concat!` so this file does not match itself.
//! Unix only, like `s029_sources.rs`.
//!
//! Hook pinned: the pure planner lives in `crates/daemon/src/plan.rs` (module
//! `plan`), re-exported by `server`; `server.rs` calls it.
#![cfg(unix)]

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    let path = root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The text above the first `#[cfg(test)]` line.
fn above_test(text: &str) -> String {
    text.lines()
        .take_while(|l| l.trim() != "#[cfg(test)]")
        .map(|l| format!("{l}\n"))
        .collect()
}

/// AC1: the daemon no longer joins the centre text and the message.
#[test]
fn ac1_server_does_not_join_centre_and_message() {
    let server = read("crates/daemon/src/server.rs");
    let needle = concat!("format!(\"{}", "\\n{message}\"");
    assert_eq!(server.matches(needle).count(), 0);
}

/// AC2 and AC10: `plan` and the strings it uses do no I/O, no process, no
/// network, no file access.
#[test]
fn ac2_ac10_plan_and_unsupported_are_pure() {
    let plan = above_test(&read("crates/daemon/src/plan.rs"));
    let core = above_test(&read("crates/core/src/unsupported.rs"));
    let needles = [
        concat!("Com", "mand"),
        concat!("std::", "process"),
        concat!("std::", "net"),
        concat!("Tcp", "Stream"),
        concat!("Udp", "Socket"),
        concat!("req", "west"),
    ];
    for (name, text) in [("plan.rs", &plan), ("unsupported.rs", &core)] {
        for n in needles {
            assert_eq!(text.matches(n).count(), 0, "{name} holds {n}");
        }
    }
    assert_eq!(plan.matches(concat!("std::", "fs")).count(), 0);
    assert!(plan.contains("fn plan"), "plan.rs must define `plan`");
}

/// AC5: the opener command is spelled only in `notify_logic.rs`.
#[test]
fn ac5_open_url_command_lives_next_to_open_folder_command() {
    let logic = above_test(&read("crates/daemon/src/notify_logic.rs"));
    assert!(logic.contains("fn open_url_command"));
    let notify = above_test(&read("crates/daemon/src/notify.rs"));
    assert_eq!(notify.matches(concat!("xdg", "-open")).count(), 0);
    let plan = above_test(&read("crates/daemon/src/plan.rs"));
    assert_eq!(plan.matches(concat!("xdg", "-open")).count(), 0);
}
