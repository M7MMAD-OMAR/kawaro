//! Non-Unix versions for `office` (S-043a): the executable check. Listed in
//! `.cargo/mutants.toml` `exclude_globs`, since the Linux mutation runs never
//! build this file; the extension check stays mutated in `tree.rs`. The
//! process tree moved to `process/windows.rs` (S-030).

use super::tree;
use std::ffi::OsStr;
use std::path::Path;

pub(super) fn is_executable(path: &Path) -> bool {
    path.is_file()
        && path
            .file_name()
            .and_then(OsStr::to_str)
            .is_some_and(tree::has_exe_extension)
}
