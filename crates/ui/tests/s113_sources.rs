//! S-113 AC7 and AC8: the x11rb dependency and the mutants exclusion, read
//! from the repository files.

use std::fs;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// `[[package]]` entries of Cargo.lock before S-113 (x11rb is already in
/// the tree through winit, so the count must not move).
const PACKAGES_BEFORE: usize = 674;

#[test]
fn ac7_x11rb_is_a_plain_linux_only_direct_dependency_of_ui() {
    let toml = read("crates/ui/Cargo.toml");
    let linux = toml
        .split("[target.'cfg(target_os = \"linux\")'.dependencies]")
        .nth(1)
        .expect("the Linux dependencies table")
        .split("\n[")
        .next()
        .unwrap();
    let line = linux
        .lines()
        .find(|l| l.trim_start().starts_with("x11rb"))
        .expect("x11rb in the Linux only table");
    assert!(line.contains("0.13.2"), "{line}");
    assert!(!line.contains("features"), "no extra feature: {line}");
    let before_linux = toml
        .split("[target.'cfg(target_os = \"linux\")'")
        .next()
        .unwrap();
    assert!(!before_linux.contains("x11rb"), "not a plain dependency");
}

#[test]
fn ac7_cargo_lock_keeps_its_package_count_and_x11rb_0_13_2() {
    let lock = read("Cargo.lock");
    assert_eq!(lock.matches("[[package]]").count(), PACKAGES_BEFORE);
    assert!(lock.contains("name = \"x11rb\"\nversion = \"0.13.2\""));
    let ui = lock
        .split("[[package]]\nname = \"kawaro-ui\"\n")
        .nth(1)
        .expect("the kawaro-ui package")
        .split("[[package]]")
        .next()
        .unwrap();
    assert!(ui.contains("\n \"x11rb\",\n"), "kawaro-ui depends on x11rb");
}

#[test]
fn ac7_engines_md_has_the_x11rb_row() {
    let doc = read("docs/engines.md");
    let row = doc
        .lines()
        .find(|l| l.starts_with('|') && l.contains("x11rb"))
        .expect("a table row for x11rb");
    for word in ["0.13.2", "MIT", "Apache", "ui", "Linux", "winit"] {
        assert!(row.contains(word), "row lacks {word}: {row}");
    }
}

#[test]
fn ac8_only_the_live_pointer_adapter_is_excluded_from_mutants() {
    let toml = read(".cargo/mutants.toml");
    assert!(toml.contains("live check S-114"), "reason text");
    assert!(
        toml.lines()
            .any(|l| l.trim_start().starts_with('"') && l.contains("crates/ui/src/x11_pointer.rs")),
        "the adapter file is in exclude_globs"
    );
    for pure in [
        "crates/ui/src/x11.rs",
        "crates/ui/src/session.rs",
        "detect",
        "plan_for_pointer",
        "plan_with",
        "x11_window_spec",
        "session_choice_line",
        "next_line",
    ] {
        let hit = toml
            .lines()
            .any(|l| l.trim_start().starts_with('"') && l.contains(pure));
        assert!(!hit, "{pure} must stay mutated");
    }
}
