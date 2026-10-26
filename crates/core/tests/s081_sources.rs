//! S-081 AC1 and AC5 (backlog/S-081-windows-long-path-and-dir-candidates.md):
//! the long-path form in the Windows place function and the bilingual
//! CHANGELOG line. They cannot run on Windows here, so this checks the files
//! themselves, like `paths_windows.rs`.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    let path = root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} must exist: {e}", path.display()))
}

#[test]
fn ac1_windows_place_sends_both_paths_through_the_long_path_form() {
    let src = read("crates/core/src/output_impl/windows.rs");
    let norm: String = src
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '&')
        .collect();
    for needle in ["win_long_path(from", "win_long_path(to"] {
        assert!(
            norm.contains(needle),
            "output_impl/windows.rs must send the temp file and the final name through win_long_path before wide (AC1): missing {needle}\n{src}"
        );
    }
    let impl_src = read("crates/core/src/output_impl.rs");
    assert!(
        impl_src.contains("fn win_long_path"),
        "the prefix logic is pure and unit tested, so it lives in output_impl.rs beside move_error_kind (AC1)"
    );
}

#[test]
fn ac5_changelog_has_one_s081_line_in_english_and_arabic() {
    let s = read("CHANGELOG.md");
    let lines: Vec<&str> = s.lines().filter(|l| l.contains("[S-081]")).collect();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].contains('«') && lines[0].contains('»'),
        "the S-081 line carries its Arabic part in guillemets like the other entries: {}",
        lines[0]
    );
}
