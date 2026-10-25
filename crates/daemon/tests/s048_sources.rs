//! S-048 AC5, AC6 and AC8 (backlog/S-048-windows-tray-shortcut.md): source
//! checks for the Windows shell thread (it builds for Windows only; `W`
//! covers the build), the icon file and its script, and the bookkeeping.

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    let path = root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} must exist: {e}", path.display()))
}

fn code_lines(s: &str) -> String {
    s.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn ac5_shell_thread_registers_the_hotkey_and_the_tray_icon() {
    let s = code_lines(&read("crates/daemon/src/shell.rs"));
    assert!(s.contains("RegisterHotKey"));
    assert!(s.contains("TaskbarCreated"));
    assert!(s.contains("Shell_NotifyIconW"));
}

#[test]
fn ac5_the_window_is_a_hidden_top_level_window_not_message_only() {
    let all = [
        read("crates/daemon/src/shell.rs"),
        read("crates/daemon/src/tray/windows.rs"),
    ];
    for s in &all {
        assert!(!s.contains("HWND_MESSAGE"));
    }
}

#[test]
fn ac5_hotkey_uses_norepeat_with_the_parsed_mods() {
    let s = code_lines(&read("crates/daemon/src/shell.rs"));
    assert!(s.contains("MOD_NOREPEAT") || s.contains("0x4000"));
    assert!(s.contains("hotkey::parse") || s.contains("parse("));
    assert!(s.contains(".mods"));
    assert!(s.contains(".vk"));
}

#[test]
fn ac5_wm_hotkey_acts_as_drop_hold_then_polls_and_releases() {
    let s = code_lines(&read("crates/daemon/src/shell.rs"));
    assert!(s.contains("WM_HOTKEY"));
    assert!(s.contains("hold: true"));
    assert!(s.contains("GetAsyncKeyState"));
    assert!(s.contains("HoldPoll"));
    assert!(s.contains("POLL_MS"));
    assert!(s.contains("DropRelease"));
}

#[test]
fn ac5_com_is_not_initialised_on_the_main_thread() {
    let s = code_lines(&read("crates/daemon/src/main.rs"));
    assert!(
        s.contains("mod shell;"),
        "main.rs declares the shell module"
    );
    assert!(!s.contains("CoInitialize"));
    assert!(read("crates/daemon/src/shell.rs").contains("Shell_NotifyIconW"));
}

#[test]
fn ac5_menu_right_to_left_is_set_from_the_pure_function() {
    let s = code_lines(&read("crates/daemon/src/tray/windows.rs"))
        + &code_lines(&read("crates/daemon/src/shell.rs"));
    assert!(s.contains("MFT_RIGHTORDER"));
    assert!(s.contains("menu_right_to_left"));
    assert!(s.contains("TrackPopupMenu"));
}

#[test]
fn ac6_tray_loads_the_icon_from_the_exe_folder_with_loadfromfile() {
    let s = code_lines(&read("crates/daemon/src/tray/windows.rs"))
        + &code_lines(&read("crates/daemon/src/shell.rs"));
    assert!(s.contains("LR_LOADFROMFILE"));
    assert!(s.contains("kawaro.ico"));
    assert!(s.contains("current_exe"));
}

/// (width, height) of each image in an ICO file, 256 written as 0.
fn ico_sizes(bytes: &[u8]) -> Vec<(u32, u32)> {
    assert!(bytes.len() >= 6, "too short for an ICO header");
    assert_eq!(&bytes[0..4], &[0, 0, 1, 0], "reserved 0, type 1 (icon)");
    let n = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
    assert!(bytes.len() >= 6 + 16 * n);
    (0..n)
        .map(|i| {
            let e = &bytes[6 + 16 * i..6 + 16 * (i + 1)];
            let f = |b: u8| if b == 0 { 256 } else { u32::from(b) };
            (f(e[0]), f(e[1]))
        })
        .collect()
}

#[test]
fn ac6_icon_is_a_windows_icon_with_16_32_and_48() {
    let path = root().join("design/brand/kawaro.ico");
    let bytes =
        std::fs::read(&path).unwrap_or_else(|e| panic!("{} must exist: {e}", path.display()));
    let sizes = ico_sizes(&bytes);
    assert!(sizes.len() >= 3, "{sizes:?}");
    for want in [16, 32, 48] {
        assert!(sizes.contains(&(want, want)), "{want} missing in {sizes:?}");
    }
    assert!(
        bytes.len() > 2000,
        "an icon of blank images: {} bytes",
        bytes.len()
    );
}

#[test]
fn ac6_file_reports_a_windows_icon_resource() {
    let out = Command::new("file")
        .arg(root().join("design/brand/kawaro.ico"))
        .output()
        .expect("file(1) must be installed");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("MS Windows icon resource"), "{text}");
    let n: usize = text
        .split("icon resource - ")
        .nth(1)
        .and_then(|r| r.split_whitespace().next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("no image count in {text}"));
    assert!(n >= 3, "{text}");
}

#[test]
fn ac6_make_ico_regenerates_an_identical_file_twice() {
    let script = root().join("scripts/windows/make-ico.sh");
    let src = read("scripts/windows/make-ico.sh");
    assert!(src.starts_with("#!/usr/bin/env bash"));
    assert!(src.contains("design/brand/kawaro.svg"));
    assert!(src.contains("kawaro.ico"));
    let ico = root().join("design/brand/kawaro.ico");
    let run = || {
        let st = Command::new("bash")
            .arg(&script)
            .status()
            .expect("bash must run");
        assert!(st.success(), "make-ico.sh exited with {st}");
        std::fs::read(&ico).expect("the script writes design/brand/kawaro.ico")
    };
    let first = run();
    let second = run();
    assert!(first == second, "a re-run must give identical bytes");
    let sizes = ico_sizes(&first);
    for want in [16, 32, 48] {
        assert!(sizes.contains(&(want, want)), "{want} missing in {sizes:?}");
    }
}

#[test]
fn ac8_shell_glue_is_excluded_from_mutants_and_the_pure_files_are_not() {
    let s = read(".cargo/mutants.toml");
    let live = |name: &str| {
        s.lines()
            .any(|l| !l.trim_start().starts_with('#') && l.contains(name))
    };
    assert!(live("crates/daemon/src/shell.rs"));
    assert!(live("crates/daemon/src/tray/windows.rs"));
    assert!(!live("shell_logic.rs"));
    assert!(!live("hotkey.rs"));
}

#[test]
fn ac8_changelog_has_one_s048_line_in_english_and_arabic() {
    let s = read("CHANGELOG.md");
    let lines: Vec<&str> = s.lines().filter(|l| l.contains("(S-048)")).collect();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains('«') && lines[0].contains('»'));
}
