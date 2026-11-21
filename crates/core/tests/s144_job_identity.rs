//! S-144: the identity of a task comes from the queue, not from the order
//! the events arrive in.
//!
//! These cases are source only. They are never executed in this pass.
//!
//! What they pin down:
//! - the `Started` event carries the manifest of the job: the resolved
//!   action and the ordered units with the identity of their sources,
//!   captured before the first task runs;
//! - every task event carries the ordinal of its unit, taken from the plan
//!   order and not from the arrival order;
//! - the wire `Event` keeps exactly the shape it had: the new internal
//!   fields never reach the wire.

mod common;

use common::{FakeEngine, action, files};
use kawaro_core::action::OutputMode::{Combined, PerFile};
use kawaro_core::engine::{CancelToken, SkipReason};
use kawaro_core::ipc::Event;
use kawaro_core::job_results::JobManifest;
use kawaro_core::queue::{JobEvent, run_job};
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::time::Duration;

fn drain(rx: &std::sync::mpsc::Receiver<JobEvent>) -> Vec<JobEvent> {
    rx.try_iter().collect()
}

/// The manifest of a started job.
fn manifest_of(events: &[JobEvent]) -> JobManifest {
    events
        .iter()
        .find_map(|event| match event {
            JobEvent::Started { manifest, .. } => Some(manifest.clone()),
            _ => None,
        })
        .expect("a started event")
}

#[test]
fn the_started_event_carries_the_manifest_of_the_planned_job() {
    let dir = tempfile::tempdir().unwrap();
    let fs = files(dir.path(), &["a.png", "b.png", "c.png"]);
    let (tx, rx) = channel();
    let act = action("fake", PerFile, "webp", "");
    run_job(1, &act, &fs, &FakeEngine::new(), &CancelToken::new(), &tx);
    let events = drain(&rx);
    let manifest = manifest_of(&events);
    // The action of the manifest is the resolved action of the run.
    assert_eq!(manifest.action.id, act.id);
    assert_eq!(manifest.action.engine, act.engine);
    assert_eq!(manifest.action.op, act.op);
    // One unit per file for a per-file action, in the order of the plan.
    assert_eq!(manifest.units.len(), 3);
    for (ordinal, unit) in manifest.units.iter().enumerate() {
        assert_eq!(unit.id, ordinal as u64);
        assert_eq!(unit.inputs.len(), 1);
        assert_eq!(unit.inputs[0].path, fs[ordinal]);
        assert!(unit.inputs[0].regular_file);
        // The identity is the size and the modification time of the source.
        let meta = std::fs::metadata(&fs[ordinal]).unwrap();
        assert_eq!(unit.inputs[0].len, meta.len());
        assert!(unit.inputs[0].modified.is_some());
    }
}

#[test]
fn a_combined_action_keeps_the_whole_group_in_one_unit() {
    let dir = tempfile::tempdir().unwrap();
    let fs = files(dir.path(), &["a.png", "b.png"]);
    let (tx, rx) = channel();
    run_job(
        1,
        &action("fake", Combined, "pdf", ""),
        &fs,
        &FakeEngine::new(),
        &CancelToken::new(),
        &tx,
    );
    let manifest = manifest_of(&drain(&rx));
    assert_eq!(manifest.units.len(), 1);
    assert_eq!(manifest.units[0].id, 0);
    let paths: Vec<PathBuf> = manifest.units[0]
        .inputs
        .iter()
        .map(|input| input.path.clone())
        .collect();
    assert_eq!(paths, fs);
}

#[test]
fn every_task_event_carries_the_ordinal_of_its_unit() {
    let dir = tempfile::tempdir().unwrap();
    let fs = files(dir.path(), &["a.png", "b.png", "c.png"]);
    let eng = FakeEngine {
        fail_on: Some("b"),
        ..FakeEngine::new()
    };
    let (tx, rx) = channel();
    run_job(
        1,
        &action("fake", PerFile, "webp", ""),
        &fs,
        &eng,
        &CancelToken::new(),
        &tx,
    );
    let events = drain(&rx);
    let units: Vec<u64> = events
        .iter()
        .filter_map(|event| match event {
            JobEvent::TaskDone { unit, .. } => Some(*unit),
            JobEvent::TaskFailed { unit, .. } => Some(*unit),
            JobEvent::TaskSkipped { unit, .. } => Some(*unit),
            _ => None,
        })
        .collect();
    // The first file succeeded, the second failed and the third succeeded.
    assert_eq!(units, [0, 1, 2]);
}

#[test]
fn a_skipped_task_carries_the_ordinal_of_its_unit() {
    let dir = tempfile::tempdir().unwrap();
    let fs = files(dir.path(), &["a.png", "b.png"]);
    let (tx, rx) = channel();
    // The fake engine skips nothing, so the skipped event is built by hand
    // to pin the shape of the event and not the engine.
    tx.send(JobEvent::Started {
        job: 1,
        action_id: "test.copy".into(),
        tasks: 2,
        manifest: JobManifest::new(
            action("fake", PerFile, "webp", ""),
            fs.iter().map(|f| vec![f.clone()]).collect(),
        ),
    })
    .unwrap();
    tx.send(JobEvent::TaskSkipped {
        job: 1,
        unit: 1,
        input: fs[1].clone(),
        reason: SkipReason::AlreadyOptimized,
        elapsed: Duration::ZERO,
    })
    .unwrap();
    drop(tx);
    let events = drain(&rx);
    assert!(matches!(events[1], JobEvent::TaskSkipped { unit: 1, .. }));
}

#[test]
fn the_wire_event_keeps_its_published_shape() {
    let dir = tempfile::tempdir().unwrap();
    let fs = files(dir.path(), &["a.png"]);
    let manifest = JobManifest::new(
        action("fake", PerFile, "webp", ""),
        fs.iter().map(|f| vec![f.clone()]).collect(),
    );
    let started = Event::from(JobEvent::Started {
        job: 1,
        action_id: "test.copy".into(),
        tasks: 1,
        manifest,
    });
    let line = serde_json::to_string(&started).unwrap();
    assert_eq!(
        line,
        r#"{"kind":"started","job":1,"action_id":"test.copy","tasks":1}"#
    );
    assert!(!line.contains("manifest"), "{line}");

    let done = Event::from(JobEvent::TaskDone {
        job: 1,
        unit: 0,
        output: PathBuf::from("/tmp/out.webp"),
        elapsed: Duration::ZERO,
        detail: None,
    });
    let line = serde_json::to_string(&done).unwrap();
    assert_eq!(
        line,
        r#"{"kind":"task-done","job":1,"output":"/tmp/out.webp"}"#
    );
    assert!(!line.contains("unit"), "{line}");

    let failed = Event::from(JobEvent::TaskFailed {
        job: 1,
        unit: 0,
        input: PathBuf::from("/tmp/in.png"),
        error: "boom".into(),
        elapsed: Duration::ZERO,
        cancelled: false,
    });
    let line = serde_json::to_string(&failed).unwrap();
    assert_eq!(
        line,
        r#"{"kind":"task-failed","job":1,"input":"/tmp/in.png","error":"boom"}"#
    );
    assert!(!line.contains("unit"), "{line}");
}
