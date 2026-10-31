//! S-110 (backlog/S-110-open-at-position.md) AC7 (no Rust `hyprctl` in the
//! daemon, ui or core) and AC8 (docs): both Hyprland READMEs describe the
//! pointer position in the launcher, the changelog has one S-110 line in
//! English and Arabic, and the touched files hold no long dash.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    let p = root().join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} must exist: {e}", p.display()))
}

const TOUCHED: [&str; 4] = [
    "integrations/linux/hyprland/README.md",
    "integrations/linux/hyprland/README.ar.md",
    "integrations/linux/hyprland/kawaro-open-selection.sh",
    "CHANGELOG.md",
];

#[test]
fn ac8_both_readmes_describe_the_pointer_position_in_the_launcher() {
    for f in [TOUCHED[0], TOUCHED[1]] {
        let t = read(f);
        assert!(t.contains("--at"), "{f} must mention --at");
        assert!(
            t.contains("hyprctl cursorpos"),
            "{f} must mention hyprctl cursorpos"
        );
        // The drop section already mentions both; the launcher's own final
        // call must be spelled out.
        assert!(
            t.contains("open --from-clipboard --at"),
            "{f} must describe the launcher call `kawaro open --from-clipboard --at X,Y`"
        );
    }
}

#[test]
fn ac8_the_changelog_has_one_s110_line_in_english_and_arabic() {
    let t = read("CHANGELOG.md");
    let lines: Vec<&str> = t.lines().filter(|l| l.contains("S-110")).collect();
    assert_eq!(lines.len(), 1, "{lines:?}");
    let (en, ar) = lines[0].split_once(" | ").expect("English | Arabic");
    assert!(en.is_ascii(), "{en}");
    assert!(
        ar.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c)),
        "{ar}"
    );
}

#[test]
fn ac8_the_touched_files_hold_no_long_dash() {
    for f in TOUCHED {
        let t = read(f);
        assert!(!t.contains('\u{2014}') && !t.contains('\u{2013}'), "{f}");
    }
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n != "target") {
                rust_files(&p, out);
            }
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

#[test]
fn ac7_hyprctl_is_only_run_by_the_launcher_script_and_the_existing_doctor() {
    // The needle is built in pieces so this file does not match itself.
    let needle = format!("Command::new({q}hypr{q})", q = '"').replace("hypr\"", "hyprctl\"");
    let mut files = Vec::new();
    rust_files(&root().join("crates"), &mut files);
    let hits: Vec<String> = files
        .iter()
        .filter(|p| !p.ends_with("s110_docs.rs"))
        .filter(|p| std::fs::read_to_string(p).is_ok_and(|t| t.contains(&needle)))
        .map(|p| {
            p.strip_prefix(root())
                .unwrap()
                .display()
                .to_string()
                .replace('\\', "/")
        })
        .collect();
    // crates/cli/src/main.rs runs it once for `kawaro doctor` (S-080); the
    // story adds none.
    let script = read(TOUCHED[2]);
    assert!(
        script.contains("hyprctl cursorpos") && script.contains("--at"),
        "the launcher reads the pointer with hyprctl cursorpos and passes --at"
    );
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert!(hits[0].ends_with("crates/cli/src/main.rs"), "{hits:?}");
    let doctor = read("crates/cli/src/main.rs");
    assert_eq!(doctor.matches(&needle).count(), 1);
}
