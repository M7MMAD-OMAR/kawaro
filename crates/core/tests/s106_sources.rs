//! S-106 (backlog/S-106-windows-small-defects.md): source scans for A2, A4
//! (daemon start), A5, A6 and A7. Separate from `s106_windows_small.rs` so
//! they compile and fail on their own assertions today.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn code(rel: &str) -> String {
    let p = root().join(rel);
    std::fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("{} must exist: {e}", p.display()))
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn s106_a2_allow_set_foreground_follows_spawn_and_takes_the_child_id() {
    let src = code("crates/core/src/update/apply/windows.rs");
    let spawn = src
        .find(".spawn()")
        .expect("start_daemon spawns the daemon");
    let after = &src[spawn..];
    let allow = after
        .find("AllowSetForegroundWindow")
        .expect("AllowSetForegroundWindow must follow .spawn()");
    let call = &after[allow..];
    let line_end = call.find(';').unwrap_or(call.len());
    assert!(
        call[..line_end].contains(".id()"),
        "the call must take the spawned child's id()"
    );
}

#[test]
fn s106_a4_daemon_runs_both_cleanups_at_start() {
    let mut all = String::new();
    for f in std::fs::read_dir(root().join("crates/daemon/src")).unwrap() {
        let p = f.unwrap().path();
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        if p.extension().is_some_and(|e| e == "rs") && !name.contains("test") {
            all.push_str(&std::fs::read_to_string(&p).unwrap());
        }
    }
    assert!(
        all.contains("clean_updates"),
        "daemon must call clean_updates"
    );
    assert!(
        all.contains("stale_helper_dirs"),
        "daemon must remove stale helper folders"
    );
}

#[test]
fn s106_a5_cleanup_source_never_names_the_system_installer_folder() {
    let src = code("crates/core/src/update/cleanup.rs");
    assert!(!src.contains("WINDIR"));
    assert!(!src.contains("Installer"));
}

#[test]
fn s106_a6_windows_identity_applies_it_through_ipropertystore_without_a_second_literal() {
    let src = code("crates/core/src/identity/windows.rs");
    assert!(
        src.contains("IPropertyStore") || src.contains("PKEY_AppUserModel_ID"),
        "the property store must be used"
    );
    assert!(src.contains("shortcut_properties"), "must apply the plan");
    assert!(
        !src.contains("io.github.m7mmad_omar"),
        "no second app id literal"
    );
}

#[test]
fn s106_a7_window_title_is_the_literal_kawaro_in_the_three_windows() {
    for f in ["settings.slint", "welcome.slint", "hive.slint"] {
        let src = code(&format!("crates/ui/ui/{f}"));
        let titles: Vec<&str> = src
            .lines()
            .filter(|l| l.starts_with("    title:") && l.trim_end().ends_with(';'))
            .map(str::trim)
            .collect();
        assert_eq!(titles, [r#"title: "Kawaro";"#], "{f}: window title");
    }
}
