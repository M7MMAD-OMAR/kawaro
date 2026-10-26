//! The Windows update helper entry (the winfix batch, security audit D1):
//! `update --apply-msi <msi>` must be handled by a binary, and the copy
//! `apply_msi` stages must be the binary that owns that command.
//!
//! Contract fixed by the tester:
//! - `kawaro_core::update::apply::helper_exe(program_dir: &Path) -> PathBuf`
//!   (pure, tested on every platform): the helper program is `kawaro.exe`
//!   inside `program_dir`, never the running program itself. `apply_msi`
//!   runs in the daemon (`kawarod.exe`), which has no `update` command, so
//!   a copy of it renamed to `kawaro.exe` could never run `helper_main`.
//! - `apply/windows.rs` (builds for Windows only, checked by source here,
//!   compiled by `scripts/windows/check.sh`): the copy source is
//!   `helper_exe(...)` and the copy starts detached with
//!   `helper_args(msi)`. The helper order itself (wait for the daemon,
//!   `msiexec` by absolute path, write the result, start a daemon) is
//!   pinned in `s054a_apply.rs`.
//! - the `kawaro` binary dispatches `update --apply-msi` to `helper_main`
//!   so the copy runs `msiexec` and writes the result; the binary level
//!   proof is `crates/cli/tests/winfix_update_msi.rs`.

use kawaro_core::update::apply::helper_exe;
use std::ffi::OsStr;
use std::path::Path;

/// The source without comment lines, like `s054b_core.rs` reads it.
fn code_lines(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} must exist: {e}", path.display()))
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn helper_exe_is_kawaro_exe_inside_the_program_folder() {
    for dir in ["C:\\Program Files\\Kawaro", "/opt/kawaro"] {
        let dir = Path::new(dir);
        let got = helper_exe(dir);
        assert_eq!(got.file_name(), Some(OsStr::new("kawaro.exe")));
        assert_eq!(got.parent(), Some(dir), "{dir:?}");
    }
}

#[test]
fn apply_msi_stages_the_helper_exe_copy() {
    let win = code_lines("src/update/apply/windows.rs");
    assert!(
        win.contains("helper_exe("),
        "apply_msi must copy helper_exe(program_dir), the kawaro.exe that handles update --apply-msi:\n{win}"
    );
    assert!(
        win.contains("helper_args("),
        "the copy must start with helper_args(msi):\n{win}"
    );
}
