//! S-067 (backlog/S-067-review-fixes-update-drop-notify.md): text tests over
//! the Windows drop glue (AC4) and the `TICK_MS` comment (AC8). The glue
//! builds for Windows only, so its shape is checked by reading it.
//!
//! Contract (tester): `crates/ui/src/dnd/windows.rs` has a function
//! `hive_closed` (any signature) that calls the tracker's `on_close()` and
//! stops the poll timer with `.stop()`, and another file of `crates/ui/src`
//! calls `hive_closed(` from the hive's close path. Whether a drag is in
//! progress is read from `is_active()`, never from the timer's `running()`.

use std::path::{Path, PathBuf};

fn src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(src().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// The body of `fn name`, from its line to the first closing brace in
/// column 0.
fn body_of<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let start = text.find(&format!("fn {name}("))?;
    let rest = &text[start..];
    Some(&rest[..rest.find("\n}\n").unwrap_or(rest.len())])
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn ac4_the_windows_glue_keeps_no_file_list_and_feeds_the_tracker_per_file() {
    let w = read("dnd/windows.rs");
    assert!(!w.contains("HOVERED"), "no HOVERED list of its own");
    assert!(
        !w.contains("Vec<PathBuf>> ="),
        "no static file list of its own"
    );
    assert!(
        w.contains("on_hover_file("),
        "each HoveredFile goes to DropTracker::on_hover_file"
    );
    assert!(
        w.contains("is_active()"),
        "is_active decides whether a drag is in progress"
    );
    assert!(
        !w.contains(".running()"),
        "the timer state no longer decides whether a drag is fresh"
    );
}

#[test]
fn ac4_closing_the_hive_stops_the_poll_timer_and_clears_the_tracker() {
    let w = read("dnd/windows.rs");
    let body = body_of(&w, "hive_closed").expect("dnd/windows.rs has fn hive_closed");
    assert!(body.contains(".stop()"), "hive_closed stops the timer");
    assert!(
        body.contains("on_close()"),
        "hive_closed clears the tracker"
    );
    let mut files = Vec::new();
    rust_files(&src(), &mut files);
    let callers: Vec<String> = files
        .iter()
        .filter(|p| !p.ends_with("dnd/windows.rs"))
        .filter(|p| !p.to_string_lossy().ends_with("tests.rs"))
        .filter(|p| std::fs::read_to_string(p).unwrap().contains("hive_closed("))
        .map(|p| p.display().to_string())
        .collect();
    assert!(
        !callers.is_empty(),
        "the close path calls dnd::windows::hive_closed"
    );
}

#[test]
fn ac8_the_tick_ms_comment_names_the_slint_timer() {
    let track = read("drop_track.rs");
    let at = track.find("pub const TICK_MS").expect("TICK_MS");
    // The doc comment lines right above the constant.
    let comment: Vec<&str> = track[..at]
        .lines()
        .rev()
        .take_while(|l| l.trim_start().starts_with("///"))
        .collect();
    let comment = comment.join("\n");
    assert!(!comment.is_empty(), "TICK_MS has a doc comment");
    assert!(comment.contains("slint::Timer"), "{comment}");
    assert!(!comment.contains("SetTimer"), "{comment}");
}
