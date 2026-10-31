//! S-114 (backlog/S-114-x11-private-session-checks.md), the fast loop half
//! that needs no display: the standalone x11rb tool, its licence record, the
//! manual release step in the README and in docs/PLAN.md, and the rule that
//! `cargo test` never starts the private X session. The script behaviour is
//! `tests/x11/s114.sh`. Unix only, like the other source tests. No network.
#![cfg(unix)]

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    let path = root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The lines of one `[section]` of a manifest, comments and blanks dropped.
fn section(toml: &str, name: &str) -> Vec<String> {
    let head = format!("[{name}]");
    toml.lines()
        .skip_while(|l| l.trim() != head)
        .skip(1)
        .take_while(|l| !l.trim_start().starts_with('['))
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect()
}

const TOOL: &str = "tests/x11/x11-tool";

#[test]
fn ac3_tool_is_a_standalone_crate_outside_the_workspace() {
    let toml = read(&format!("{TOOL}/Cargo.toml"));
    assert!(
        toml.lines().any(|l| l.trim() == "[workspace]"),
        "its own empty [workspace] table"
    );
    let ws = read("Cargo.toml");
    assert!(
        !ws.contains("x11-tool") && !ws.contains("tests/x11"),
        "not a workspace member"
    );
    assert!(
        !section(&ws, "workspace")
            .iter()
            .any(|l| l.contains("tests")),
        "no tests path in the workspace table"
    );
    assert!(
        section(&toml, "package")
            .iter()
            .any(|l| l == "name = \"x11-tool\"")
    );
    assert!(
        section(&toml, "package")
            .iter()
            .any(|l| l == "license = \"GPL-3.0-or-later\"")
    );
    assert!(
        section(&toml, "package")
            .iter()
            .any(|l| l == "publish = false")
    );
}

#[test]
fn ac3_tool_depends_on_x11rb_only_with_xtest() {
    let toml = read(&format!("{TOOL}/Cargo.toml"));
    let deps = section(&toml, "dependencies");
    assert_eq!(deps.len(), 1, "x11rb only: {deps:?}");
    assert!(deps[0].starts_with("x11rb"), "{deps:?}");
    assert!(deps[0].contains("0.13"), "{deps:?}");
    assert!(
        deps[0].contains("xtest"),
        "XTEST feature for `move`: {deps:?}"
    );
    assert!(section(&toml, "dev-dependencies").is_empty());
    assert!(section(&toml, "build-dependencies").is_empty());
}

#[test]
fn ac3_tool_has_the_five_subcommands_and_reads_through_the_server() {
    let src = read(&format!("{TOOL}/src/main.rs"));
    for sub in ["move", "key", "geometry", "pixel", "stacking"] {
        assert!(src.contains(&format!("\"{sub}\"")), "subcommand {sub}");
    }
    assert!(
        src.contains("translate_coordinates"),
        "geometry to the root"
    );
    assert!(src.contains("query_pointer"), "move reads the pointer back");
    assert!(src.contains("xtest"), "input goes through XTEST");
}

#[test]
fn ac3_tool_target_dir_and_lock_are_handled() {
    let ignore = read(".gitignore");
    assert!(
        ignore
            .lines()
            .any(|l| l.trim() == "tests/x11/x11-tool/target/")
    );
    assert!(
        root().join(TOOL).join("Cargo.lock").is_file(),
        "Cargo.lock committed so the tool builds the same way each time"
    );
}

#[test]
fn ac3_licence_of_x11rb_in_the_tool_is_recorded() {
    let engines = read("docs/engines.md");
    let row = engines
        .lines()
        .find(|l| l.starts_with("| x11rb"))
        .expect("an x11rb row");
    assert!(row.contains("MIT OR Apache-2.0"), "{row}");
    assert!(row.contains("S-114"), "the row names S-114: {row}");
    assert!(row.contains("x11-tool"), "the row names the tool: {row}");
    assert!(
        row.contains("xtest"),
        "the row names the xtest feature: {row}"
    );
}

#[test]
fn ac3_no_opener_binary_names_in_the_new_rust_files() {
    let needles = [
        concat!("xdg", "-open"),
        concat!("gvfs", "-open"),
        concat!("kde", "-open"),
        concat!("gnome", "-open"),
        concat!("exo", "-open"),
        concat!("gio", " open"),
        concat!("wsl", "view"),
    ];
    for rel in [
        format!("{TOOL}/src/main.rs"),
        "crates/core/tests/s114_sources.rs".to_string(),
    ] {
        let text = read(&rel);
        for n in needles {
            assert!(!text.contains(n), "{rel} names {n}");
        }
    }
}

#[test]
fn ac1_cargo_test_never_calls_the_script() {
    let needle = concat!("x11/", "run.sh");
    assert!(
        root().join("tests/x11/run.sh").is_file(),
        "the script exists"
    );
    let mut hits = Vec::new();
    for krate in ["core", "engines", "ui", "daemon", "cli"] {
        for dir in ["src", "tests"] {
            let d = root().join("crates").join(krate).join(dir);
            let Ok(entries) = std::fs::read_dir(&d) else {
                continue;
            };
            for e in entries.flatten() {
                let p = e.path();
                if p.file_name().is_some_and(|n| n == "s114_sources.rs") {
                    continue;
                }
                if p.extension().is_some_and(|x| x == "rs")
                    && std::fs::read_to_string(&p).is_ok_and(|t| t.contains(needle))
                {
                    hits.push(p.display().to_string());
                }
            }
        }
    }
    assert!(hits.is_empty(), "{hits:?}");
}

#[test]
fn ac10_readme_names_the_command_the_skip_rule_and_the_manual_step() {
    let readme = read("tests/x11/README.md");
    assert!(readme.contains("tests/x11/run.sh"), "the command");
    assert!(readme.contains("SKIP: Xvfb not found"), "the SKIP line");
    assert!(readme.contains("xfwm4"), "the xfwm4 half of the skip rule");
    let l = readme.to_lowercase();
    assert!(l.contains("by hand"), "run by hand");
    assert!(l.contains("0.5.0"), "before the 0.5.0 tag");
    assert!(
        l.contains("release notes"),
        "output kept in the release notes"
    );
    assert!(
        !readme.contains('\u{2014}') && !readme.contains('\u{2013}'),
        "no long dashes"
    );
}

#[test]
fn ac10_plan_has_one_line_in_the_050_table() {
    let plan = read("docs/PLAN.md");
    let sec = plan
        .split("## 0.5.0 \"X11\"")
        .nth(1)
        .expect("the 0.5.0 section")
        .split("\n## ")
        .next()
        .unwrap();
    let lines: Vec<&str> = sec
        .lines()
        .filter(|l| l.contains("tests/x11/run.sh"))
        .collect();
    assert_eq!(lines.len(), 1, "exactly one PLAN line: {lines:?}");
    assert!(lines[0].starts_with('|'), "a table row: {}", lines[0]);
    let l = lines[0].to_lowercase();
    assert!(l.contains("by hand"), "{l}");
    assert!(l.contains("0.5.0"), "{l}");
    assert!(l.contains("release notes"), "{l}");
}

#[test]
fn ac1_the_script_and_its_helpers_are_in_the_repo() {
    for rel in ["tests/x11/run.sh", "tests/x11/lib.sh", "tests/x11/s114.sh"] {
        let meta = std::fs::metadata(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"));
        assert!(meta.is_file(), "{rel}");
    }
}
