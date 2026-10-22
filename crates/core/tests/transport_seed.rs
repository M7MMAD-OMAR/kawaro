//! S-022a AC7 (backlog/S-022-windows-ipc.md): the Windows transport seed.
//! It cannot run on Linux, so this checks the file itself; `W` (the Windows
//! clippy run in the story) proves it builds without a warning.

use std::path::Path;

#[test]
fn ac7_windows_seed_is_unsupported_and_excluded_from_mutants() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let seed = root.join("crates/core/src/transport/windows.rs");
    let text = std::fs::read_to_string(&seed)
        .unwrap_or_else(|e| panic!("{} must exist: {e}", seed.display()));
    assert!(
        text.contains("ErrorKind::Unsupported"),
        "the seed must answer Unsupported"
    );
    assert!(!text.contains("todo!") && !text.contains("unimplemented!"));
    let mutants = std::fs::read_to_string(root.join(".cargo/mutants.toml")).unwrap();
    let globs = mutants
        .split("exclude_globs")
        .nth(1)
        .and_then(|rest| rest.split(']').next())
        .expect("exclude_globs list");
    assert!(
        globs.contains("\"crates/core/src/transport/windows.rs\""),
        "windows.rs must be in exclude_globs: {globs}"
    );
}
