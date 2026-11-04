//! S-133 AC1: the neutral skip outcome, the queue event and the counts.
//! Contract pinned by this test (tester): kawaro_core::engine::TaskOutcome
//! { Written, Skipped(SkipReason) }; SkipReason { AlreadyOptimized } is a
//! serde enum spelled "already-optimized"; Engine::run returns
//! Result<TaskOutcome, EngineError>; the queue turns Skipped into
//! JobEvent::TaskSkipped { job, input, reason, elapsed }, removes the temp,
//! keeps Finished.ok/failed unchanged and adds Finished.skipped. The IPC
//! Event::Finished gains skipped with a serde default of 0.

use kawaro_core::action::{ActionDef, Localized, OutputSpec};
use kawaro_core::engine::{Engine, EngineError, SkipReason, Task, TaskOutcome};
use kawaro_core::queue::{JobEvent, run_job};
use kawaro_core::{CancelToken, MediaKind, OutputMode, Progress};
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;

struct Fake;

impl Engine for Fake {
    fn id(&self) -> &'static str {
        "fake"
    }
    fn capabilities(&self) -> Vec<(&'static str, Vec<MediaKind>)> {
        Vec::new()
    }
    fn run(
        &self,
        task: &Task,
        progress: Progress,
        cancel: &CancelToken,
    ) -> Result<TaskOutcome, EngineError> {
        progress(0.0);
        cancel.check()?;
        let name = task.inputs[0]
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if name.contains("already") {
            // A temp file the queue must remove after the skip.
            std::fs::write(&task.output, b"temp that must be removed").map_err(EngineError::Io)?;
            return Ok(TaskOutcome::Skipped(SkipReason::AlreadyOptimized));
        }
        std::fs::write(&task.output, b"smaller rewrite").map_err(EngineError::Io)?;
        progress(1.0);
        Ok(TaskOutcome::Written)
    }
}

fn action() -> ActionDef {
    ActionDef {
        id: "pdf.compress".to_string(),
        engine: "fake".to_string(),
        op: "compress".to_string(),
        ring: 1,
        order: 0,
        accepts: vec![MediaKind::Pdf],
        skip_ext: Vec::new(),
        min_files: 1,
        max_files: 0,
        output: OutputSpec {
            mode: OutputMode::PerFile,
            ext: String::new(),
            suffix: "-compressed".to_string(),
            dir: String::new(),
        },
        code: Localized {
            ar: "ضغط".to_string(),
            en: "SHRINK".to_string(),
        },
        label: Localized {
            ar: "ضغط ملف PDF".to_string(),
            en: "Compress PDF".to_string(),
        },
        icon: "compress".to_string(),
        params: toml::Table::new(),
        options: None,
    }
}

fn input(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, b"%PDF-1.5 small").unwrap();
    path
}

fn run(files: &[PathBuf]) -> Vec<JobEvent> {
    let (tx, rx) = channel();
    run_job(0, &action(), files, &Fake, &CancelToken::new(), &tx);
    drop(tx);
    rx.into_iter().collect()
}

fn finished(events: &[JobEvent]) -> (usize, usize, usize) {
    events
        .iter()
        .find_map(|event| match event {
            JobEvent::Finished {
                ok,
                failed,
                skipped,
                ..
            } => Some((*ok, *failed, *skipped)),
            _ => None,
        })
        .expect("a Finished event")
}

#[test]
fn ac1_a_skipped_task_is_its_own_event_and_writes_no_final_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = input(dir.path(), "already.pdf");
    let events = run(std::slice::from_ref(&file));
    let skipped: Vec<&JobEvent> = events
        .iter()
        .filter(|event| matches!(event, JobEvent::TaskSkipped { .. }))
        .collect();
    assert_eq!(skipped.len(), 1, "one TaskSkipped: {events:?}");
    match skipped[0] {
        JobEvent::TaskSkipped {
            job, input, reason, ..
        } => {
            assert_eq!(*job, 0);
            assert_eq!(input, &file);
            assert_eq!(reason, &SkipReason::AlreadyOptimized);
        }
        _ => unreachable!(),
    }
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, JobEvent::TaskDone { .. })),
        "a skip is not a TaskDone: {events:?}"
    );
    assert!(
        !dir.path().join("already-compressed.pdf").exists(),
        "no -compressed file is finalised for a skipped task"
    );
    assert_eq!(finished(&events), (0, 0, 1));
}

#[test]
fn ac1_written_still_finalises_and_counts_ok() {
    let dir = tempfile::tempdir().unwrap();
    let file = input(dir.path(), "small.pdf");
    let events = run(std::slice::from_ref(&file));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, JobEvent::TaskDone { .. }))
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, JobEvent::TaskSkipped { .. }))
    );
    assert_eq!(finished(&events), (1, 0, 0));
}

#[test]
fn ac1_a_mixed_job_counts_one_written_and_one_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let skipped = input(dir.path(), "already.pdf");
    let written = input(dir.path(), "small.pdf");
    let events = run(&[skipped, written]);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, JobEvent::TaskSkipped { .. }))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, JobEvent::TaskDone { .. }))
            .count(),
        1
    );
    assert_eq!(finished(&events), (1, 0, 1));
}

#[test]
fn ac1_skip_reason_serializes_as_a_kebab_case_enum() {
    let line = serde_json::to_string(&SkipReason::AlreadyOptimized).unwrap();
    assert_eq!(line, "\"already-optimized\"");
    let back: SkipReason = serde_json::from_str(&line).unwrap();
    assert_eq!(back, SkipReason::AlreadyOptimized);
}

#[test]
fn ac1_finished_skipped_defaults_to_zero_for_old_events() {
    use kawaro_core::ipc::Event;
    let old = r#"{"kind":"finished","job":1,"ok":1,"failed":0,"cancelled":false}"#;
    let event: Event = serde_json::from_str(old).unwrap();
    match event {
        Event::Finished {
            ok,
            failed,
            skipped,
            ..
        } => assert_eq!((ok, failed, skipped), (1, 0, 0)),
        other => panic!("expected Finished, got {other:?}"),
    }
    let line = serde_json::to_string(&Event::Finished {
        job: 1,
        ok: 1,
        failed: 0,
        skipped: 2,
        cancelled: false,
    })
    .unwrap();
    assert!(line.contains("\"skipped\":2"), "{line}");
}
