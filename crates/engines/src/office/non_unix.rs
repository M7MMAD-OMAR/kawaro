//! Non-Unix fallbacks for `office`. Listed in `.cargo/mutants.toml`
//! `exclude_globs`, since the Linux mutation runs never build this file.

use std::path::Path;

pub(super) fn is_executable(path: &Path) -> bool {
    path.is_file()
}

pub(super) fn kill_group(child: &mut std::process::Child) {
    let _ = child.kill();
}
