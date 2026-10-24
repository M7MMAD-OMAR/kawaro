//! S-046 AC3 and AC9 (backlog/S-046-windows-hive-window.md): source checks
//! for the Windows window, which builds for Windows only (`W`,
//! `scripts/windows/check.sh`), and the mutation and changelog bookkeeping.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    let path = root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} must exist: {e}", path.display()))
}

#[test]
fn ac3_window_is_undecorated_on_top_without_taskbar_button_or_shadow() {
    let s = read("crates/ui/src/win_window.rs");
    assert!(s.contains("with_undecorated_shadow(false)"));
    assert!(s.contains("with_decorations(false)") || s.contains("WS_POPUP"));
    assert!(s.contains("AlwaysOnTop") || s.contains("WS_EX_TOPMOST"));
    assert!(s.contains("with_skip_taskbar(true)") || s.contains("WS_EX_TOOLWINDOW"));
}

#[test]
fn ac3_corner_preference_is_do_not_round_and_pinned_with_three_asserts() {
    let s = read("crates/ui/src/win_window.rs");
    assert!(s.contains("DWMWA_WINDOW_CORNER_PREFERENCE"));
    assert!(s.contains("DWMWCP_DONOTROUND"));
    assert!(
        s.matches("const _: () = assert!(").count() >= 3,
        "three pinned constants"
    );
    assert!(s.contains("windows_sys"), "pinned against windows_sys");
    // The value 1 is the one pinned for DWMWCP_DONOTROUND.
    let line = s
        .lines()
        .find(|l| l.contains("assert!") && l.contains("DWMWCP_DONOTROUND"))
        .expect("an assert on DWMWCP_DONOTROUND");
    assert!(line.contains("== 1"), "{line}");
}

#[test]
fn ac3_the_window_has_no_shadow_gradient_or_acrylic() {
    let s = read("crates/ui/src/win_window.rs");
    for bad in ["box-shadow", "gradient", "Acrylic", "acrylic"] {
        assert!(!s.contains(bad), "{bad} in win_window.rs");
    }
}

#[test]
fn ac4_the_foreground_call_lives_in_shell_open() {
    let s = read("crates/daemon/src/shell_open.rs");
    assert!(s.contains("SetForegroundWindow"));
    assert!(s.contains("focus_log"));
}

#[test]
fn ac9_win32_glue_is_excluded_from_mutants_and_the_pure_files_are_not() {
    let s = read(".cargo/mutants.toml");
    assert!(s.contains("win_window.rs"));
    assert!(s.contains("shell_open.rs"));
    for pure in ["monitor.rs", "merge.rs", "args.rs"] {
        let excluded = s
            .lines()
            .any(|l| !l.trim_start().starts_with('#') && l.contains(pure));
        assert!(!excluded, "{pure} must stay mutated");
    }
}

#[test]
fn ac9_changelog_has_one_s046_line_in_english_and_arabic() {
    let s = read("CHANGELOG.md");
    let lines: Vec<&str> = s.lines().filter(|l| l.contains("(S-046)")).collect();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains('«') && lines[0].contains('»'));
}
