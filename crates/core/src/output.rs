//! Output naming and atomic finalisation. The original file is never touched.
//!
//! Naming: `<dir>/<stem><suffix>.<ext>`. When that exists, `<stem><suffix> (1).<ext>`,
//! then `(2)` and so on. When the output would equal the input path (same
//! extension, empty suffix), numbering starts at `(1)` as well.

use crate::action::OutputSpec;
use std::path::{Path, PathBuf};

/// Final output path for `input` under `spec`, inside `out_dir` (normally the input's directory).
pub fn final_path(input: &Path, spec: &OutputSpec, out_dir: &Path) -> PathBuf {
    crate::output_impl::final_path(input, spec, out_dir)
}

/// Hidden temporary path next to `final_path`, same extension, unique per call.
pub fn temp_path(final_path: &Path) -> PathBuf {
    crate::output_impl::temp_path(final_path)
}

/// Moves `temp` to `final_path` without replacing an existing file. If
/// `final_path` appeared meanwhile, picks the next free numbered name.
/// Returns the path actually used.
pub fn finalize(temp: &Path, final_path: &Path) -> std::io::Result<PathBuf> {
    crate::output_impl::finalize(temp, final_path)
}
