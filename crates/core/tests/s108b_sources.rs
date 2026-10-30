//! S-108 part B (backlog/S-108-s096b-hive-cells.md): source scans of the
//! daemon side of AC1 and AC6. Needles are split with `concat!` so this file
//! does not match itself. Unix only, like `s108_sources.rs`.
//!
//! Hook pinned: `ui::present` in `crates/daemon/src/ui.rs` takes the
//! suggestion next to the message and hands both to the hive through
//! `Hive::set_notes`; it no longer builds `"{centre}\n{message}"`.
#![cfg(unix)]

use std::path::Path;

fn read(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn ac1_ui_present_does_not_join_the_centre_and_the_message() {
    let ui = read("crates/daemon/src/ui.rs");
    let needle = concat!("format!(\"{centre}", "\\n{message}\"");
    assert_eq!(ui.matches(needle).count(), 0);
}

#[test]
fn ac6_ui_present_hands_message_and_suggestion_to_the_hive() {
    let ui = read("crates/daemon/src/ui.rs");
    assert!(ui.contains(".set_notes("), "ui.rs calls Hive::set_notes");
    let present = ui
        .split("pub fn present(")
        .nth(1)
        .and_then(|rest| rest.split(") {").next())
        .expect("ui::present exists");
    assert!(
        present.contains("suggestion: Option<String>"),
        "ui::present takes the suggestion: {present}"
    );
    let server = read("crates/daemon/src/server.rs");
    assert!(
        server.contains("suggestion"),
        "server.rs passes the plan's suggestion to the window"
    );
}
