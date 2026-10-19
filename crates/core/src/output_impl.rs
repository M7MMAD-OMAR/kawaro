//! Owned by the core track; see `output.rs` for the contract.
use crate::action::OutputSpec;
use std::path::{Path, PathBuf};

pub(crate) fn final_path(_input: &Path, _spec: &OutputSpec, _out_dir: &Path) -> PathBuf {
    todo!("core track")
}
pub(crate) fn temp_path(_final_path: &Path) -> PathBuf {
    todo!("core track")
}
pub(crate) fn finalize(_temp: &Path, _final_path: &Path) -> std::io::Result<PathBuf> {
    todo!("core track")
}
