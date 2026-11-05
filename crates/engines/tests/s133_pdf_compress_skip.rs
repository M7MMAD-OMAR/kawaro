//! S-133 AC2 and AC4: pdf.compress skips an already optimized result.
//! Contract pinned here (tester): GhostscriptEngine::compress returns
//! Ok(TaskOutcome::Skipped(SkipReason::AlreadyOptimized)) when the rewrite is
//! equal or larger, writes no task.output and leaves the input untouched;
//! a smaller rewrite returns Ok(TaskOutcome::Written) and the final file.

use kawaro_core::engine::{SkipReason, TaskOutcome};
use kawaro_core::{CancelToken, Engine, EngineError, Task};
use kawaro_engines::pdf::GhostscriptEngine;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn task(input: &Path, output: PathBuf) -> Task {
    Task {
        action_id: "pdf.compress".to_string(),
        op: "compress".to_string(),
        params: toml::from_str("preset = \"screen\"\ndpi = 72").unwrap(),
        inputs: vec![input.to_path_buf()],
        output,
    }
}

fn run(engine: &GhostscriptEngine, task: &Task) -> Result<TaskOutcome, EngineError> {
    engine.run(task, &|_fraction| {}, &CancelToken::new())
}

#[cfg(unix)]
fn fake_gs(dir: &Path, name: &str, size: usize) -> PathBuf {
    let path = dir.join(name);
    let script = String::from(
        "#!/bin/sh\nout=\"\"\nfor a in \"$@\"; do\n  case \"$a\" in\n    -sOutputFile=*) out=$(printf '%s' \"$a\" | cut -c14-);;\n  esac\ndone\nhead -c ",
    ) + &size.to_string()
        + " /dev/zero > \"$out\"\nexit 0\n";
    std::fs::write(&path, script).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    path
}

#[cfg(unix)]
fn input_pdf(dir: &Path) -> PathBuf {
    let path = dir.join("doc.pdf");
    let mut bytes = b"%PDF-1.5\n".to_vec();
    bytes.resize(100, 0);
    std::fs::write(&path, bytes).unwrap();
    path
}

#[cfg(unix)]
#[test]
fn ac2_a_larger_or_equal_rewrite_is_skipped_without_an_output() {
    for size in [200usize, 100] {
        let dir = tempdir().unwrap();
        let work = dir.path().join("work");
        std::fs::create_dir(&work).unwrap();
        let engine =
            GhostscriptEngine::with_program(fake_gs(dir.path(), "gs", size)).temp_root(&work);
        let input = input_pdf(dir.path());
        let before = std::fs::read(&input).unwrap();
        let output = dir.path().join("doc-compressed.pdf");
        let outcome = run(&engine, &task(&input, output.clone())).expect("a fake gs succeeds");
        assert_eq!(
            outcome,
            TaskOutcome::Skipped(SkipReason::AlreadyOptimized),
            "size {size} is not smaller"
        );
        assert!(!output.exists(), "no -compressed file for a skipped task");
        assert_eq!(std::fs::read(&input).unwrap(), before, "input untouched");
    }
}

#[cfg(unix)]
#[test]
fn ac2_a_smaller_rewrite_is_written() {
    let dir = tempdir().unwrap();
    let work = dir.path().join("work");
    std::fs::create_dir(&work).unwrap();
    let engine = GhostscriptEngine::with_program(fake_gs(dir.path(), "gs", 50)).temp_root(&work);
    let input = input_pdf(dir.path());
    let output = dir.path().join("doc-compressed.pdf");
    let outcome = run(&engine, &task(&input, output.clone())).expect("a fake gs succeeds");
    assert_eq!(outcome, TaskOutcome::Written);
    assert!(output.exists(), "the smaller rewrite is the output");
    assert_eq!(std::fs::metadata(&output).unwrap().len(), 50);
}

#[test]
fn ac4_a_real_ghostscript_skips_an_already_optimized_pdf() {
    let engine = GhostscriptEngine::default();
    if !engine.available() {
        eprintln!("skipped: Ghostscript is not installed");
        return;
    }
    let dir = tempdir().unwrap();
    let work = dir.path().join("work");
    std::fs::create_dir(&work).unwrap();
    let engine = engine.temp_root(&work);
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/docs/one-page.pdf");
    let output = dir.path().join("one-page-compressed.pdf");
    let outcome = run(&engine, &task(&fixture, output.clone())).expect("gs runs");
    assert_eq!(
        outcome,
        TaskOutcome::Skipped(SkipReason::AlreadyOptimized),
        "the tiny fixture has no smaller rewrite"
    );
    assert!(!output.exists());
}
