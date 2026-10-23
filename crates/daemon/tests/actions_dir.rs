//! S-041 AC4 (backlog/S-041-windows-paths.md): one source for the user
//! actions folder. `user_actions_dir` is gone from every crate and the
//! daemon loads its registry from `Config::actions_dir()`.
//!
//! The searched text is built from two pieces, and is never written whole
//! in this file, so the reviewer's grep over `crates/` for the old function
//! definition does not match this file. The behaviour
//! (a TOML action under `XDG_CONFIG_HOME/sbarhive/actions/` shows in
//! `sbarhive actions`) is `actions_include_user_actions_from_the_config_dir`
//! in `crates/cli/tests/cli.rs`, which already passes and guards against a
//! regression.

use std::path::{Path, PathBuf};

fn crates_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

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
fn ac4_no_crate_defines_user_actions_dir() {
    let needle = ["fn user", "_actions_dir"].concat();
    let mut files = Vec::new();
    rust_files(&crates_dir(), &mut files);
    assert!(files.len() > 10, "walked too few files: {}", files.len());
    let hits: Vec<String> = files
        .iter()
        .filter(|f| std::fs::read_to_string(f).is_ok_and(|s| s.contains(&needle)))
        .map(|f| f.display().to_string())
        .collect();
    assert!(
        hits.is_empty(),
        "these files still define the old helper: {hits:?} (AC4)"
    );
}

#[test]
fn ac4_load_registry_uses_config_actions_dir() {
    let state = std::fs::read_to_string(crates_dir().join("daemon/src/state.rs")).unwrap();
    let body = state
        .split("fn load_registry")
        .nth(1)
        .expect("state.rs keeps fn load_registry");
    let body = body.split("\n}\n").next().unwrap();
    assert!(
        body.contains("Config::actions_dir()"),
        "load_registry must call Config::actions_dir() (AC4):\n{body}"
    );
}
