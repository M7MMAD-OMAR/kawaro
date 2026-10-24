//! The service log file (S-045 AC7). Stands alone: no `crate::` items, so a
//! test can include it by path.

use std::fs::{File, OpenOptions};
use std::io;
use std::path::Path;

/// Opens `path` for appending, creating it and any missing parent folder.
pub fn open_append(path: &Path) -> io::Result<File> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    OpenOptions::new().create(true).append(true).open(path)
}

// The Windows only stderr redirect lives in its own file so Linux mutation
// runs leave it out (.cargo/mutants.toml); S-052 checks it live.
#[cfg(windows)]
#[path = "logfile/windows.rs"]
mod windows;
#[cfg(windows)]
pub use windows::redirect_stderr;
