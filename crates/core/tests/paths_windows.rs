//! S-023 AC5 and AC7 (backlog/S-023-windows-paths.md): the Windows files.
//! They cannot run on Linux, so this checks the files themselves; `W` (the
//! Windows clippy run in the story) proves they build without a warning.
//!
//! The ban on `MOVEFILE_REPLACE_EXISTING` covers the whole file, comments
//! included: the name must not appear in `output_impl/windows.rs` at all.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    let path = root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} must exist: {e}", path.display()))
}

fn exclude_globs() -> String {
    read(".cargo/mutants.toml")
        .split("exclude_globs")
        .nth(1)
        .and_then(|rest| rest.split(']').next())
        .expect("exclude_globs list")
        .to_string()
}

fn missing<'a>(text: &str, needles: &[&'a str]) -> Vec<&'a str> {
    needles
        .iter()
        .copied()
        .filter(|n| !text.contains(n))
        .collect()
}

#[test]
fn ac5_paths_windows_reads_both_known_folders_and_frees_them() {
    let src = read("crates/core/src/paths/windows.rs");
    let lacks = missing(
        &src,
        &[
            "SHGetKnownFolderPath",
            "FOLDERID_RoamingAppData",
            "FOLDERID_LocalAppData",
            "CoTaskMemFree",
        ],
    );
    assert!(
        lacks.is_empty(),
        "paths/windows.rs lacks {lacks:?}: known folders come from SHGetKnownFolderPath and are freed with CoTaskMemFree (AC5)"
    );
    assert!(
        !src.contains("todo!"),
        "paths/windows.rs must not hold todo!"
    );
}

#[test]
fn ac5_ac7_windows_files_are_excluded_from_mutants() {
    let globs = exclude_globs();
    for f in [
        "\"crates/core/src/paths/windows.rs\"",
        "\"crates/core/src/output_impl/windows.rs\"",
    ] {
        assert!(globs.contains(f), "{f} must be in exclude_globs (AC5, AC7)");
    }
}

#[test]
fn ac5_ac7_paths_rs_and_output_impl_rs_stay_mutated() {
    let globs = exclude_globs();
    for f in [
        "\"crates/core/src/paths.rs\"",
        "\"crates/core/src/output_impl.rs\"",
        "\"crates/core/src/config.rs\"",
    ] {
        assert!(!globs.contains(f), "{f} must stay under mutation testing");
    }
    assert!(
        root().join("crates/core/src/paths.rs").is_file(),
        "crates/core/src/paths.rs must exist"
    );
}

#[test]
fn ac7_windows_places_with_move_file_ex_without_replacing() {
    let src = read("crates/core/src/output_impl/windows.rs");
    let lacks = missing(&src, &["MoveFileExW", "move_error_kind("]);
    assert!(
        lacks.is_empty(),
        "output_impl/windows.rs lacks {lacks:?}: the place function is MoveFileExW and maps its code with move_error_kind (AC7)"
    );
    assert!(
        !src.contains("MOVEFILE_REPLACE_EXISTING"),
        "output_impl/windows.rs must not name MOVEFILE_REPLACE_EXISTING (AC7)"
    );
    assert!(
        !src.contains("todo!"),
        "output_impl/windows.rs must not hold todo!"
    );
}
