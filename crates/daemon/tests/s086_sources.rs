//! S-086 AC2, AC3 and AC5 (backlog/S-086-icon-opens-window.md): source
//! checks for the parts that only run on a live Windows machine, like
//! `s046_sources.rs`. The MSI Start shortcut command, the unchanged
//! autostart, the raise of the existing settings window and the W01
//! checklist text cannot be observed here, so this checks the files
//! themselves; the live click stays in the owner checklist.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    let path = root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} must exist: {e}", path.display()))
}

/// The `<Shortcut ...>` element of the MSI source.
fn shortcut_element(s: &str) -> &str {
    let start = s
        .find("<Shortcut ")
        .expect("a Shortcut element in kawaro.wxs");
    let end = s[start..].find("/>").expect("the Shortcut element ends") + start;
    &s[start..end + 2]
}

#[test]
fn ac2_the_msi_start_shortcut_runs_kawarod_settings() {
    let s = read("installer/windows/kawaro.wxs");
    let element = shortcut_element(&s);
    assert!(
        element.contains(r#"Target="[INSTALLDIR]kawarod.exe""#),
        "the Start shortcut targets kawarod.exe (AC2): {element}"
    );
    assert!(
        element.contains(r#"Arguments="settings""#),
        "the Start shortcut runs `kawarod.exe settings`, so a click opens the window (AC2): {element}"
    );
}

#[test]
fn ac3_the_run_value_still_starts_the_silent_daemon_with_no_window() {
    let s = read("installer/windows/kawaro.wxs");
    assert!(
        s.contains(r#"Name="Kawaro" Type="string" Value="&quot;[INSTALLDIR]kawarod.exe&quot;""#),
        "the autostart Run value carries no argument (AC3)"
    );
}

#[test]
fn ac2_kawarod_keeps_the_windows_subsystem() {
    let s = read("crates/daemon/src/main.rs");
    assert!(
        s.contains(r#"windows_subsystem = "windows""#),
        "no console window may appear or flash (AC2)"
    );
}

#[test]
fn ac2_the_existing_settings_window_is_raised_not_reopened() {
    let s = read("crates/daemon/src/ui.rs");
    let start = s
        .find("pub fn open_settings")
        .expect("ui::open_settings exists");
    let end = s.find("pub fn quit").expect("quit follows open_settings");
    let body = &s[start..end];
    assert!(
        body.contains("welcome_window()"),
        "the window is created only when it does not exist yet (AC2): {body}"
    );
    assert!(
        body.contains("settings_visible()"),
        "a second click preserves the currently edited settings: {body}"
    );
    assert!(
        body.contains("is_visible"),
        "a second click looks at the existing window instead of opening another (AC2): {body}"
    );
    assert!(
        body.contains("shell_open::foreground"),
        "the re-focus is the Windows foreground path the hive uses for `open` (AC2): {body}"
    );
}

#[test]
fn ac5_w01_carries_the_click_procedure_and_the_one_second_expectation() {
    let s = read("docs/windows-beta-checklist.md");
    let start = s.find("W01 ").expect("step W01 exists");
    let end = s[start..].find("W02 ").expect("step W02 follows") + start;
    let w01 = &s[start..end];
    let do_text = w01.split("Expect:").next().unwrap_or_default();
    let expect_text = w01.split("Expect:").nth(1).unwrap_or_default();
    assert!(
        do_text.contains("kawarod.exe settings") && do_text.contains("nine more"),
        "the Do names the command of the shortcut and the ten click procedure (AC2, AC3): {do_text}"
    );
    assert!(
        expect_text.contains("within 1 s"),
        "the Expect carries the observable 1 s budget (AC2): {expect_text}"
    );
    assert!(
        expect_text.contains("one Kawaro window")
            && expect_text.contains("one kawarod.exe process"),
        "the Expect carries the one window and one daemon count of ten clicks (AC3): {expect_text}"
    );
}

#[test]
fn ac5_changelog_has_one_s086_line_in_english_and_arabic() {
    let s = read("CHANGELOG.md");
    let lines: Vec<&str> = s.lines().filter(|l| l.contains("[S-086]")).collect();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].contains('«') && lines[0].contains('»'),
        "the S-086 line carries its Arabic part in guillemets like the other entries: {}",
        lines[0]
    );
    assert!(
        !lines[0].contains('\u{2013}') && !lines[0].contains('\u{2014}'),
        "no long dash in the S-086 line: {}",
        lines[0]
    );
}
