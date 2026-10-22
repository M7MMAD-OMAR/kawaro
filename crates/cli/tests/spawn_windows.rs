//! S-024 AC2 and AC6 (backlog/S-024-windows-daemon-spawn.md): the Windows
//! side of starting the service. It cannot run on Linux, so this checks the
//! file itself; `W` and `B` in the story prove it builds for the target, and
//! the `const _: () = assert!(...)` lines prove the shared flag values match
//! `windows_sys` at compile time. AC7 has no test here: the story says the
//! Windows `cargo check` alone proves it.

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

#[test]
fn ac2_windows_spawn_checks_each_flag_against_windows_sys_at_compile_time() {
    let src = read("crates/cli/src/spawn/windows.rs");
    let asserts = src.matches("const _: () = assert!(").count();
    assert!(
        asserts >= 3,
        "spawn/windows.rs has {asserts} compile time asserts, one per flag is required (AC2)"
    );
    for name in [
        "DETACHED_PROCESS",
        "CREATE_NEW_PROCESS_GROUP",
        "CREATE_BREAKAWAY_FROM_JOB",
    ] {
        let checked = src.split("const _: () = assert!(").skip(1).any(|chunk| {
            let stmt = chunk.split(';').next().unwrap_or("");
            stmt.contains(name) && stmt.contains("windows_sys")
        });
        assert!(
            checked,
            "{name} must be checked against windows_sys in a const assert (AC2)"
        );
    }
}

#[test]
fn ac2_ac5_windows_spawn_uses_creation_flags_and_allow_set_foreground_window() {
    let src = read("crates/cli/src/spawn/windows.rs");
    for needle in ["creation_flags", "AllowSetForegroundWindow"] {
        assert!(
            src.contains(needle),
            "spawn/windows.rs must use {needle} (AC2, AC5)"
        );
    }
    assert!(
        !src.contains("todo!"),
        "spawn/windows.rs must not hold todo!"
    );
}

#[test]
fn ac6_windows_sys_is_a_windows_only_dependency_of_the_cli() {
    let toml = read("crates/cli/Cargo.toml");
    let section = toml
        .split("[target.'cfg(windows)'.dependencies]")
        .nth(1)
        .unwrap_or_else(|| {
            panic!("crates/cli/Cargo.toml needs [target.'cfg(windows)'.dependencies] (AC6)")
        });
    let section = section.split("\n[").next().unwrap_or("");
    assert!(
        section.contains("windows-sys"),
        "windows-sys must sit under cfg(windows) in crates/cli/Cargo.toml (AC6)"
    );
    let before = toml
        .split("[target.'cfg(windows)'.dependencies]")
        .next()
        .unwrap();
    assert!(
        !before.contains("windows-sys"),
        "windows-sys must not be an every platform dependency of the cli (AC6)"
    );
}

#[test]
fn ac2_windows_spawn_is_excluded_from_mutants_and_the_shared_files_are_not() {
    let globs = exclude_globs();
    assert!(
        globs.contains("\"crates/cli/src/spawn/windows.rs\""),
        "crates/cli/src/spawn/windows.rs must be in exclude_globs (story command M)"
    );
    for f in [
        "\"crates/cli/src/spawn.rs\"",
        "\"crates/cli/src/client.rs\"",
        "\"crates/cli/src/main.rs\"",
    ] {
        assert!(!globs.contains(f), "{f} must stay under mutation testing");
    }
}
