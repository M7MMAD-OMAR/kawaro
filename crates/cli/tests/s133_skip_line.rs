//! S-133 AC3: a local pdf.compress that skips prints a skipped line and
//! exits 0, and writes no -compressed duplicate.
#![cfg(unix)]

use std::path::Path;
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_kawaro");

fn gs_available() -> bool {
    Command::new("gs")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn kawaro(dir: &Path, args: &[&str]) -> Output {
    Command::new(BIN)
        .current_dir(dir)
        .args(args)
        .env_clear()
        .env("HOME", dir)
        .env("XDG_RUNTIME_DIR", dir)
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_CACHE_HOME", dir.join("cache"))
        // The in-process engine lookup needs the real PATH to find gs; the
        // isolated XDG_RUNTIME_DIR still keeps the daemon away.
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .output()
        .unwrap()
}

#[test]
fn ac3_a_skipped_compress_exits_zero_and_writes_no_duplicate() {
    if !gs_available() {
        eprintln!("skipped: Ghostscript is not installed");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/docs/one-page.pdf");
    std::fs::copy(fixture, dir.path().join("one-page.pdf")).unwrap();

    let out = kawaro(
        dir.path(),
        &[
            "run",
            "pdf.compress",
            "--set",
            "preset=\"screen\"",
            "--set",
            "dpi=72",
            "one-page.pdf",
        ],
    );
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert_eq!(
        out.status.code(),
        Some(0),
        "a skip is not a failure: {stdout} {stderr}"
    );
    assert!(
        stdout.to_lowercase().contains("skipped:"),
        "the CLI prints a skipped line: {stdout}"
    );
    assert!(
        stdout.to_lowercase().contains("already optimized"),
        "the line says why: {stdout}"
    );
    assert!(
        !dir.path().join("one-page-compressed.pdf").exists(),
        "no duplicate -compressed file"
    );
}
