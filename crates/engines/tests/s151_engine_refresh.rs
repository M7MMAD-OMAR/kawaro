//! S-151: a program installed after the daemon started is seen by the
//! engines, without a restart and without a child process.
//!
//! Unix only: the lookup reads `PATH`, and the test points it at its own
//! directory. The fake program is a file with the exec bit set; no program
//! is ever run.

#![cfg(unix)]

use kawaro_core::Engine;
use kawaro_engines::ocr::OcrEngine;
use kawaro_engines::ocrmypdf::OcrmypdfEngine;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The `PATH` of this process belongs to this test alone.
static ENV: Mutex<()> = Mutex::new(());

/// A fake program file in `dir`, executable, never run.
fn program(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// An empty directory that is the whole `PATH` of this test.
fn empty_bin() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("bin");
    std::fs::create_dir(&bin).unwrap();
    dir
}

/// The `PATH` of this process, only this test uses it.
fn set_path(bin: &Path) {
    // SAFETY: this test file holds the only tests that read `PATH`, and
    // the guard in each test keeps them one at a time.
    unsafe { std::env::set_var("PATH", bin) };
}

#[test]
fn s151_a_refresh_sees_a_program_installed_after_the_start() {
    let _guard = ENV.lock().unwrap_or_else(|e| e.into_inner());
    let dir = empty_bin();
    let bin = dir.path().join("bin");
    set_path(&bin);

    let ocr = OcrEngine::default();
    assert!(!ocr.available(), "the tool is not installed yet");
    assert_eq!(ocr.program(), None);

    // The tool appears now, after the engine was built.
    let tesseract = program(&bin, "tesseract");
    ocr.refresh();
    assert!(ocr.available(), "the refresh missed the new program");
    assert_eq!(ocr.program(), Some(tesseract.as_path()));
    // An engine built later in the same process sees it too.
    let later = OcrEngine::default();
    assert_eq!(later.program(), Some(tesseract.as_path()));

    let ocrmypdf = OcrmypdfEngine::default();
    assert!(!ocrmypdf.available(), "the tool is not installed yet");
    let ocrmypdf_bin = program(&bin, "ocrmypdf");
    ocrmypdf.refresh();
    assert!(ocrmypdf.available(), "the refresh missed the new program");
    assert_eq!(ocrmypdf.program(), Some(ocrmypdf_bin.as_path()));
    let later = OcrmypdfEngine::default();
    assert_eq!(later.program(), Some(ocrmypdf_bin.as_path()));
}

#[test]
fn s151_a_program_given_by_hand_never_changes() {
    let _guard = ENV.lock().unwrap_or_else(|e| e.into_inner());
    let dir = empty_bin();
    set_path(&dir.path().join("bin"));

    let pinned = OcrEngine::with_program("/nonexistent/tesseract");
    pinned.refresh();
    assert_eq!(pinned.program(), None);

    let other = dir.path().join("other-tesseract");
    std::fs::write(&other, "#!/bin/sh\n").unwrap();
    let pinned = OcrEngine::with_program(other.clone());
    pinned.refresh();
    assert_eq!(pinned.program(), Some(other.as_path()));
}
