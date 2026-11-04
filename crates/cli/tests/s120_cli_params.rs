//! S-120 (backlog/S-120-cli-set-and-schema.md): kawaro run --set and
//! kawaro actions --schema.
//!
//! The real binary runs in an isolated HOME, config and XDG_RUNTIME_DIR with
//! no daemon on that runtime dir, so a --set run uses the in process path.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_kawaro");

fn kawaro(dir: &Path, args: &[&str]) -> Output {
    Command::new(BIN)
        .args(args)
        .env_clear()
        .env("HOME", dir)
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_RUNTIME_DIR", dir)
        .env("PATH", dir)
        .env("LANG", "C")
        .output()
        .unwrap()
}

/// The repository BMP copied into the test folder, so the run writes next to
/// the copy and never into the repository.
fn copy_sample(dir: &Path) -> PathBuf {
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/images/sample.bmp");
    let target = dir.join("sample.bmp");
    std::fs::copy(&source, &target).unwrap();
    target
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn schema(dir: &Path, id: &str) -> serde_json::Value {
    let out = kawaro(dir, &["actions", "--id", id, "--schema"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let text = String::from_utf8_lossy(&out.stdout);
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("not JSON: {e}\n{text}"))
}

#[test]
fn actions_schema_for_a_form_action_is_the_closed_params_schema() {
    let dir = tempfile::tempdir().unwrap();
    let schema = schema(dir.path(), "image.to-webp");
    assert_eq!(schema["type"].as_str(), Some("object"));
    assert!(schema["properties"].get("quality").is_some(), "{schema}");
    assert_eq!(schema["additionalProperties"].as_bool(), Some(false));
}

#[test]
fn actions_schema_for_an_action_without_options_is_empty() {
    let dir = tempfile::tempdir().unwrap();
    // image.to-png has no options (image.to-pdf gained them with S-123).
    let schema = schema(dir.path(), "image.to-png");
    assert_eq!(schema["type"].as_str(), Some("object"));
    assert!(
        schema["properties"].as_object().unwrap().is_empty(),
        "{schema}"
    );
    assert_eq!(schema["additionalProperties"].as_bool(), Some(false));
}

#[test]
fn actions_schema_with_an_unknown_id_fails_and_names_it() {
    let dir = tempfile::tempdir().unwrap();
    let out = kawaro(
        dir.path(),
        &["actions", "--id", "no.such.action", "--schema"],
    );
    assert!(!out.status.success());
    assert!(stderr(&out).contains("no.such.action"), "{}", stderr(&out));
}

#[test]
fn actions_schema_rejects_json_and_file_arguments() {
    let dir = tempfile::tempdir().unwrap();
    let with_json = kawaro(
        dir.path(),
        &["actions", "--id", "image.to-webp", "--schema", "--json"],
    );
    assert!(
        !with_json.status.success(),
        "schema with --json must fail: {}",
        stderr(&with_json)
    );

    let image = copy_sample(dir.path());
    let with_files = kawaro(
        dir.path(),
        &[
            "actions",
            "--id",
            "image.to-webp",
            "--schema",
            image.to_str().unwrap(),
        ],
    );
    assert!(
        !with_files.status.success(),
        "schema with files must fail: {}",
        stderr(&with_files)
    );
}

#[test]
fn run_set_writes_next_to_the_input_and_never_overwrites() {
    let dir = tempfile::tempdir().unwrap();
    let image = copy_sample(dir.path());
    let original = std::fs::read(&image).unwrap();
    let image = image.to_str().unwrap();

    let out = kawaro(
        dir.path(),
        &["run", "image.to-webp", "--set", "quality=10", image],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let first = dir.path().join("sample.webp");
    assert!(first.is_file(), "the webp is written next to the input");

    let out = kawaro(
        dir.path(),
        &["run", "image.to-webp", "--set", "quality=10", image],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let second = dir.path().join("sample (1).webp");
    assert!(second.is_file(), "the second run takes the next number");

    assert_eq!(
        std::fs::read(dir.path().join("sample.bmp")).unwrap(),
        original,
        "the input is unchanged"
    );
}

#[test]
fn run_set_with_an_unknown_key_fails_and_names_it() {
    let dir = tempfile::tempdir().unwrap();
    let image = copy_sample(dir.path());
    let out = kawaro(
        dir.path(),
        &[
            "run",
            "image.to-webp",
            "--set",
            "bogus=1",
            image.to_str().unwrap(),
        ],
    );
    assert!(!out.status.success());
    assert!(stderr(&out).contains("bogus"), "{}", stderr(&out));
    assert!(
        !dir.path().join("sample.webp").exists(),
        "a refused run writes no output"
    );
}

#[test]
fn run_set_with_a_bad_value_fails() {
    let dir = tempfile::tempdir().unwrap();
    let image = copy_sample(dir.path());
    let out = kawaro(
        dir.path(),
        &[
            "run",
            "image.to-webp",
            "--set",
            "quality=notanumber",
            image.to_str().unwrap(),
        ],
    );
    assert!(!out.status.success());
    assert!(stderr(&out).contains("quality"), "{}", stderr(&out));
    assert!(!dir.path().join("sample.webp").exists());
}
