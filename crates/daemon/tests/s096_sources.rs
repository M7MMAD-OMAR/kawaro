//! S-096 AC6: the old notification string (see the story)
//! appears nowhere under `crates/`. The files are read at test time; the
//! needle is built from pieces so this file does not match itself.

use std::path::{Path, PathBuf};

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn ac6_old_no_actions_string_appears_nowhere_in_crates() {
    let needle = ["No actions", "for these", "files"].join(" ");
    let crates = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    rust_files(&crates, &mut files);
    assert!(files.len() > 20);
    let hits: Vec<_> = files
        .iter()
        .filter(|f| std::fs::read_to_string(f).is_ok_and(|t| t.contains(&needle)))
        .collect();
    assert!(hits.is_empty(), "{hits:?}");
}
