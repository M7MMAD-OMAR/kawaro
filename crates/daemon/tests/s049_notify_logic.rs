//! S-049 AC2 and AC4 (backlog/S-049-windows-notifications.md): the pure
//! helpers of the notifications. The daemon is a binary crate, so the module
//! file is included by path.
//!
//! Contract (tester, from the acceptance criteria; do not edit). Module
//! `crates/daemon/src/notify_logic.rs` (`mod notify_logic;` in main.rs). It
//! uses only `std` and `kawaro_core`, never `crate::`, so it compiles here:
//! - `Os` with variants `Linux` and `Windows`, reachable as
//!   `notify_logic::Os` (its own enum or a `pub use` of
//!   `kawaro_core::paths::Os`).
//! - `open_folder_command(os: Os, folder: &Path, file: Option<&Path>)
//!   -> (String, Vec<String>)`, pure.
//! - `log_line(body: &str) -> String` gives `kawarod: notify: <body>`.
//! - `count_notify_lines(log: &str) -> usize` counts the lines that start
//!   with `kawarod: notify: `.

#[allow(dead_code)]
#[path = "../src/notify_logic.rs"]
mod notify_logic;

use notify_logic::{Os, count_notify_lines, log_line, open_folder_command};
use std::path::Path;

fn owned(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

// ---------- AC2 ----------

#[test]
fn ac2_windows_with_a_file_selects_it_in_explorer_as_one_argument() {
    let got = open_folder_command(
        Os::Windows,
        Path::new(r"C:\out dir"),
        Some(Path::new(r"C:\out dir\a b.webp")),
    );
    assert_eq!(
        got,
        (
            "explorer.exe".to_string(),
            owned(&[r"/select,C:\out dir\a b.webp"])
        )
    );
}

#[test]
fn ac2_windows_without_a_file_opens_the_folder_in_explorer() {
    let got = open_folder_command(Os::Windows, Path::new(r"C:\out"), None);
    assert_eq!(got, ("explorer.exe".to_string(), owned(&[r"C:\out"])));
}

#[test]
fn ac2_linux_opens_the_folder_with_xdg_open_with_or_without_a_file() {
    let folder = Path::new("/home/u/out dir");
    assert_eq!(
        open_folder_command(Os::Linux, folder, None),
        ("xdg-open".to_string(), owned(&["/home/u/out dir"]))
    );
    assert_eq!(
        open_folder_command(
            Os::Linux,
            folder,
            Some(Path::new("/home/u/out dir/a b.webp"))
        ),
        ("xdg-open".to_string(), owned(&["/home/u/out dir"]))
    );
}

// ---------- AC4 ----------

#[test]
fn ac4_log_line_has_the_format_the_e2e_tests_count() {
    assert_eq!(log_line("Done: 1 file"), "kawarod: notify: Done: 1 file");
    assert_eq!(log_line(""), "kawarod: notify: ");
}

#[test]
fn ac4_count_notify_lines_counts_three_of_five() {
    let log = [
        "kawarod: notify: Done: 1 file",
        "kawarod: settings: bad key",
        "kawarod: notify: Failed: 1 of 2",
        "something else",
        "kawarod: notify: Cancelled",
    ]
    .join("\n");
    assert_eq!(count_notify_lines(&log), 3);
    assert_eq!(count_notify_lines(&format!("{log}\n")), 3);
}

#[test]
fn ac4_count_notify_lines_of_an_empty_text_is_zero() {
    assert_eq!(count_notify_lines(""), 0);
}

#[test]
fn ac4_the_prefix_in_the_middle_of_a_line_does_not_count() {
    let log = "x kawarod: notify: Done\n  kawarod: notify: indented\nkawarod: notify: real";
    assert_eq!(count_notify_lines(log), 1);
    assert_eq!(count_notify_lines(&log_line("one")), 1);
}
