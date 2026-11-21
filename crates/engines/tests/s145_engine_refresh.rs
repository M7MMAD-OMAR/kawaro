//! S-145: an engine refresh finds a program that was installed after the
//! engine was built, so the daemon sees a new tool without a restart.
//!
//! The lookup reads the `PATH` of the process, so the test points it at an
//! empty folder, builds the engine there, then writes the program into that
//! folder and refreshes. No child process runs: the refresh only reads the
//! file system.
#![cfg(unix)]

use kawaro_core::Engine;
use std::path::{Path, PathBuf};

/// An executable file at `dir/name`, returning its path.
fn program(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, "#!/bin/sh\n").unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    path
}

#[test]
fn s145_the_refresh_finds_a_program_installed_after_the_engine_was_built() {
    let dir = tempfile::tempdir().unwrap();
    // SAFETY: this test binary holds one test, and it is the only code here
    // that reads `PATH`.
    unsafe { std::env::set_var("PATH", dir.path()) };

    // Poppler: the tool is not installed yet.
    let poppler = kawaro_engines::poppler::PopplerEngine::discover();
    assert!(!poppler.available(), "the tool is not installed yet");

    // Office: the tool is not installed yet.
    let office = kawaro_engines::office::OfficeEngine::discover();
    assert!(!office.available(), "the tool is not installed yet");

    // Both tools appear now.
    let pdftotext = program(dir.path(), "pdftotext");
    let soffice = program(dir.path(), "soffice");
    assert!(pdftotext.is_file() && soffice.is_file());

    // The refresh looks them up again.
    poppler.refresh();
    office.refresh();

    assert!(poppler.available(), "the refresh found pdftotext");
    assert!(office.available(), "the refresh found soffice");

    // A refresh that finds nothing again says so, and never keeps a stale
    // answer.
    unsafe { std::env::set_var("PATH", dir.path().join("empty")) };
    poppler.refresh();
    assert!(!poppler.available(), "the missing tool is missing again");
}
