//! S-132 AC3: a local conversion prints the downscaled dimensions and bytes
//! under the output path, in the CLI's English, from the shared detail line.
#![cfg(unix)]

use std::path::Path;
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_kawaro");

fn kawaro(dir: &Path, args: &[&str]) -> Output {
    Command::new(BIN)
        .current_dir(dir)
        .args(args)
        .env_clear()
        .env("HOME", dir)
        .env("XDG_RUNTIME_DIR", dir)
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_CACHE_HOME", dir.join("cache"))
        .env("PATH", dir)
        .output()
        .unwrap()
}

fn copy_png(dir: &Path) -> std::path::PathBuf {
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/images/kawaro-sample.png");
    let target = dir.join("source.png");
    std::fs::copy(source, &target).unwrap();
    target
}

fn detail_line(stdout: &str) -> String {
    stdout
        .lines()
        .find(|line| line.contains(" bytes"))
        .unwrap_or_else(|| panic!("no detail line in {stdout:?}"))
        .to_string()
}

#[test]
fn ac3_a_downscaled_local_run_prints_dimensions_and_bytes() {
    let dir = tempfile::tempdir().unwrap();
    copy_png(dir.path());
    let convert = kawaro(dir.path(), &["convert", "--to", "jpg", "source.png"]);
    assert!(
        convert.status.success(),
        "convert failed: {}",
        String::from_utf8_lossy(&convert.stderr)
    );
    assert!(dir.path().join("source.jpg").is_file());

    let out = kawaro(
        dir.path(),
        &[
            "run",
            "image.compress-to",
            "--set",
            "target_bytes=20000",
            "source.jpg",
        ],
    );
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "compress-to failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout
            .lines()
            .any(|line| line.trim_end().ends_with("source-small.jpg")),
        "the output path is printed: {stdout}"
    );
    let line = detail_line(&stdout);
    assert!(
        line.contains(" x "),
        "the final dimensions are printed: {line}"
    );
    assert!(
        line.contains("(downscaled from "),
        "the source dimensions are printed: {line}"
    );
    assert!(line.contains(" bytes"), "the bytes are printed: {line}");
}

#[test]
fn ac3_a_run_that_fits_at_the_original_size_has_no_downscaled_phrase() {
    let dir = tempfile::tempdir().unwrap();
    copy_png(dir.path());
    let convert = kawaro(dir.path(), &["convert", "--to", "jpg", "source.png"]);
    assert!(
        convert.status.success(),
        "convert failed: {}",
        String::from_utf8_lossy(&convert.stderr)
    );
    let out = kawaro(
        dir.path(),
        &[
            "run",
            "image.compress-to",
            "--set",
            "target_bytes=20000000",
            "source.jpg",
        ],
    );
    assert!(
        out.status.success(),
        "compress-to failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let line = detail_line(&String::from_utf8_lossy(&out.stdout));
    assert!(line.contains(" x "), "the dimensions are printed: {line}");
    assert!(
        !line.contains("downscaled from"),
        "no fallback means no downscaled phrase: {line}"
    );
}
