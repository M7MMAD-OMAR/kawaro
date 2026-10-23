//! S-040a AC7 and S-040b AC8, AC10 and AC12 (backlog/S-040-windows-ipc.md):
//! the Windows transport. It cannot run on Linux, so this checks the file
//! itself; `W` (the Windows clippy run in the story) proves it builds without
//! a warning, and its `const` asserts prove the flags match `windows_sys`.
//!
//! S-040b tester: the S-040a seed check for `ErrorKind::Unsupported` is gone,
//! because S-040b replaces the seed with the real pipe. The exclusion from
//! mutants and the ban on `todo!` stay.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn windows_rs() -> String {
    let path = root().join("crates/core/src/transport/windows.rs");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} must exist: {e}", path.display()))
}

fn exclude_globs() -> String {
    let mutants = std::fs::read_to_string(root().join(".cargo/mutants.toml")).unwrap();
    mutants
        .split("exclude_globs")
        .nth(1)
        .and_then(|rest| rest.split(']').next())
        .expect("exclude_globs list")
        .to_string()
}

fn assert_mentions(text: &str, needles: &[&str], why: &str) {
    let missing: Vec<&str> = needles
        .iter()
        .copied()
        .filter(|n| !text.contains(n))
        .collect();
    assert!(missing.is_empty(), "windows.rs lacks {missing:?}: {why}");
}

#[test]
fn ac7_windows_transport_is_excluded_from_mutants_and_has_no_todo() {
    let text = windows_rs();
    assert!(!text.contains("todo!") && !text.contains("unimplemented!"));
    let globs = exclude_globs();
    assert!(
        globs.contains("\"crates/core/src/transport/windows.rs\""),
        "windows.rs must be in exclude_globs: {globs}"
    );
}

#[test]
fn ac10_deadline_rs_is_mutated_on_linux() {
    let path = root().join("crates/core/src/transport/deadline.rs");
    assert!(path.is_file(), "{} must exist", path.display());
    assert!(
        !exclude_globs().contains("deadline.rs"),
        "deadline.rs is pure and must stay mutated"
    );
}

#[test]
fn ac8_windows_builds_the_descriptor_from_dacl_sddl() {
    assert_mentions(
        &windows_rs(),
        &[
            "dacl_sddl(",
            "ConvertStringSecurityDescriptorToSecurityDescriptorW",
            "CreateNamedPipeW",
        ],
        "the pipe must be created with the DACL from dacl_sddl (AC8)",
    );
}

#[test]
fn ac10_windows_cancels_the_read_with_cancel_io_ex() {
    assert_mentions(
        &windows_rs(),
        &["read_with_deadline(", "CancelIoEx"],
        "the read timeout must cancel the pipe read (AC10)",
    );
}

#[test]
fn ac9_ac11_ac12_windows_uses_the_pure_decisions() {
    assert_mentions(
        &windows_rs(),
        &[
            "open_mode(",
            "pipe_mode(",
            "bind_result(",
            "connect_plan(",
            "verify_owner(",
            "WaitNamedPipeW",
        ],
        "windows.rs must call the tested pure functions (AC9, AC11, AC12)",
    );
}

#[test]
fn ac12_windows_asserts_every_flag_against_windows_sys_at_compile_time() {
    let text = windows_rs();
    assert!(
        text.contains("const _: () = assert!"),
        "compile time asserts are required (AC12)"
    );
    assert_mentions(
        &text,
        &[
            "PIPE_ACCESS_DUPLEX",
            "FILE_FLAG_FIRST_PIPE_INSTANCE",
            "PIPE_REJECT_REMOTE_CLIENTS",
            "PIPE_TYPE_BYTE",
            "PIPE_READMODE_BYTE",
            "PIPE_WAIT",
            "ERROR_ACCESS_DENIED",
            "ERROR_FILE_NOT_FOUND",
            "ERROR_PIPE_BUSY",
        ],
        "each constant must be checked against windows_sys (AC12)",
    );
}

#[test]
fn ac12_windows_checks_the_owner_and_names_the_endpoint_from_the_token() {
    assert_mentions(
        &windows_rs(),
        &[
            "GetNamedPipeServerProcessId",
            "ProcessIdToSessionId",
            "GetTokenInformation",
            "TokenUser",
            "pipe_name(",
        ],
        "owner check and endpoint come from the process token (AC12)",
    );
}
