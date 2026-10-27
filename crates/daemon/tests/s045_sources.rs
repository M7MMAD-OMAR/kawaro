//! S-045 (backlog/S-045-windows-cfg-gates.md): source and file checks for
//! AC2, AC3, AC4, AC5, AC8 and AC9 (the Cargo.lock count is in the build script). The Windows build itself (AC1, AC5
//! subsystem values, AC8 scripts run) is checked by
//! `tests/windows/s045_build.sh`.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    let path = root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} must exist: {e}", path.display()))
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn is_code(line: &str) -> bool {
    let t = line.trim();
    !t.is_empty() && !t.starts_with("//") && !t.starts_with("#[") && !t.starts_with('}')
}

/// True when the attributes directly above `lines[i]` include a unix gate.
fn gated_above(lines: &[&str], i: usize) -> bool {
    let mut j = i;
    while j > 0 {
        j -= 1;
        let t = lines[j].trim();
        if t.starts_with("#[") {
            if t.starts_with("#[cfg(unix)]") || t.starts_with("#[cfg(all(unix") {
                return true;
            }
        } else if !t.starts_with("//") {
            return false;
        }
    }
    false
}

/// True when the line, or an enclosing item above it, carries `cfg(unix)`.
fn gated(lines: &[&str], hit: usize) -> bool {
    if gated_above(lines, hit) {
        return true;
    }
    let mut cur = indent(lines[hit]);
    for i in (0..hit).rev() {
        if is_code(lines[i]) && indent(lines[i]) < cur {
            cur = indent(lines[i]);
            if gated_above(lines, i) {
                return true;
            }
        }
    }
    false
}

#[test]
fn s045_ac2_startup_notify_and_token_hook_only_under_cfg_unix() {
    let text = read("crates/ui/src/lib.rs");
    let lines: Vec<&str> = text.lines().collect();
    let mut hits = 0;
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim_start();
        if t.starts_with("//") {
            continue;
        }
        if line.contains("startup_notify") || line.contains("token_hook") {
            hits += 1;
            assert!(
                gated(&lines, i),
                "lib.rs:{} is not inside a cfg(unix) item: {line}",
                i + 1
            );
        }
    }
    assert!(hits > 0, "the Unix path must still use the activation hook");
}

#[test]
fn s045_ac3_ksni_only_under_the_linux_target_table() {
    let text = read("crates/daemon/Cargo.toml");
    let mut section = String::new();
    let mut seen = 0;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            section = t.to_string();
        } else if t.starts_with("ksni") {
            seen += 1;
            assert_eq!(
                section, r#"[target.'cfg(target_os = "linux")'.dependencies]"#,
                "ksni declared in the wrong table: {t}"
            );
        }
    }
    assert_eq!(seen, 1, "ksni must be declared exactly once");
}

#[test]
fn s045_ac4_windows_tray_module_has_the_three_functions() {
    let tray = read("crates/daemon/src/tray.rs");
    assert!(
        tray.contains("windows"),
        "tray.rs must pick a Windows module"
    );
    let win = read("crates/daemon/src/tray/windows.rs");
    for sig in ["pub fn start(", "pub fn set_active(", "pub fn set_lang("] {
        assert!(win.contains(sig), "tray/windows.rs lacks `{sig}`");
    }
    assert!(win.contains("TrayHandle"));
}

#[test]
fn s045_ac5_main_rs_starts_with_the_windows_subsystem_attribute() {
    let text = read("crates/daemon/src/main.rs");
    let first = text
        .lines()
        .find(|l| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with("//")
        })
        .unwrap();
    assert_eq!(
        first.trim(),
        r#"#![cfg_attr(windows, windows_subsystem = "windows")]"#
    );
}

#[test]
fn s045_ac8_check_script_runs_daemon_and_ui_clippy() {
    let check = read("scripts/windows/check.sh");
    for want in [
        "-p kawaro-daemon",
        "-p kawaro-ui",
        "--lib --bins",
        "-D warnings",
        "x86_64-pc-windows-gnu",
    ] {
        assert!(check.contains(want), "check.sh lacks `{want}`");
    }
}

#[test]
fn s045_ac8_build_script_builds_the_daemon_in_release() {
    let build = read("scripts/windows/build.sh");
    assert!(
        build.contains("kawaro-daemon") || build.contains("kawarod"),
        "build.sh must build kawarod.exe"
    );
}

#[test]
fn s045_ac9_changelog_has_the_line() {
    // The line was under [Unreleased] until the 0.4.0 release. The check keeps
    // guarding the entry form (a list item with Arabic in «»), and no longer
    // pins the section a release cut moves it out of.
    let text = read("CHANGELOG.md");
    let line = text
        .lines()
        .find(|l| l.starts_with("- ") && l.contains("(S-045)"))
        .expect("an S-045 changelog line");
    assert!(
        line.contains('«') && line.contains('»'),
        "needs Arabic in «»"
    );
}
