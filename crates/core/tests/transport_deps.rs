//! S-040b AC13 and AC14 (backlog/S-040-windows-ipc.md): no new package,
//! `windows-sys 0.61.2` for core on Windows only, and its license recorded.
//! The `cargo tree` runs (`X` and the Linux one) and the prose of
//! `SECURITY.md` and `docs/architecture.md` stay with the reviewer.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// The `[[package]]` block of `name` in `Cargo.lock`.
fn lock_entry(lock: &str, name: &str) -> String {
    lock.split("[[package]]")
        .find(|b| b.lines().any(|l| l.trim() == format!("name = \"{name}\"")))
        .unwrap_or_else(|| panic!("{name} not in Cargo.lock"))
        .to_string()
}

#[test]
fn ac13_cargo_lock_keeps_671_packages() {
    // Merge into main (2026-10-23): the portal work on main added ashpd, so
    // the lock holds 672; the Windows stories still add no package.
    let lock = read("Cargo.lock");
    let n = lock.lines().filter(|l| *l == "[[package]]").count();
    assert_eq!(n, 672, "no new package may enter Cargo.lock");
}

#[test]
fn ac13_core_depends_on_windows_sys_0_61_2_in_the_lock() {
    let entry = lock_entry(&read("Cargo.lock"), "sbarhive-core");
    assert!(
        entry.contains("\"windows-sys 0.61.2\""),
        "sbarhive-core must use windows-sys 0.61.2:\n{entry}"
    );
}

#[test]
fn ac13_core_declares_windows_sys_for_windows_only() {
    let toml = read("crates/core/Cargo.toml");
    let (before, after) = toml
        .split_once("[target.'cfg(windows)'.dependencies]")
        .expect("crates/core/Cargo.toml needs [target.'cfg(windows)'.dependencies]");
    let table = after.split("\n[").next().unwrap();
    assert!(
        table.contains("windows-sys"),
        "windows-sys belongs in the cfg(windows) table:\n{table}"
    );
    assert!(
        table.contains("0.61"),
        "windows-sys must be the 0.61 line already in Cargo.lock:\n{table}"
    );
    let elsewhere = format!("{before}{}", &after[table.len()..]);
    assert!(
        !elsewhere.contains("windows-sys"),
        "windows-sys must not be a dependency on Linux"
    );
}

#[test]
fn ac14_engines_md_records_windows_sys_and_its_license() {
    // A table row or a short section: the name, version and license within
    // a few lines of each other.
    let doc = read("docs/engines.md");
    let lines: Vec<&str> = doc.lines().collect();
    let found = lines.iter().enumerate().any(|(i, l)| {
        l.contains("windows-sys") && {
            let near = lines[i..lines.len().min(i + 8)].join("\n");
            near.contains("0.61.2") && near.contains("MIT OR Apache-2.0")
        }
    });
    assert!(
        found,
        "docs/engines.md must record windows-sys 0.61.2 (MIT OR Apache-2.0)"
    );
}
