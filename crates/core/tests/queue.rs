//! S-003: plan, run_job and the background Queue.

mod common;

use common::{FakeEngine, action, files, listing};
use sbarhive_core::action::OutputMode::{Combined, PerFile};
use sbarhive_core::engine::{CancelToken, Engine};
use sbarhive_core::queue::{JobEvent, Queue, plan, run_job};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Barrier};
use std::time::Duration;

const WAIT: Duration = Duration::from_secs(20);

fn drain(rx: &Receiver<JobEvent>) -> Vec<JobEvent> {
    rx.try_iter().collect()
}

/// Collects events until `n` Finished events arrived.
fn until_finished(rx: &Receiver<JobEvent>, n: usize) -> Vec<JobEvent> {
    let mut out = Vec::new();
    let mut done = 0;
    while done < n {
        let e = rx.recv_timeout(WAIT).expect("event in time");
        if matches!(e, JobEvent::Finished { .. }) {
            done += 1;
        }
        out.push(e);
    }
    out
}

fn finished_of(events: &[JobEvent], job: u64) -> (usize, usize, bool) {
    events
        .iter()
        .find_map(|e| match e {
            JobEvent::Finished {
                job: j,
                ok,
                failed,
                cancelled,
            } if *j == job => Some((*ok, *failed, *cancelled)),
            _ => None,
        })
        .expect("finished event")
}

fn progress_of(events: &[JobEvent], job: u64) -> Vec<f32> {
    events
        .iter()
        .filter_map(|e| match e {
            JobEvent::Progress { job: j, fraction } if *j == job => Some(*fraction),
            _ => None,
        })
        .collect()
}

// ---------- plan ----------

#[test]
fn ac1_plan_per_file_one_task_each_next_to_input() {
    let d = tempfile::tempdir().unwrap();
    let sub = d.path().join("sub");
    std::fs::create_dir(&sub).unwrap();
    let mut fs = files(d.path(), &["a.png", "b.jpg"]);
    fs.extend(files(&sub, &["c.gif"]));
    let mut act = action("fake", PerFile, "webp", "");
    act.params
        .insert("quality".into(), toml::Value::Integer(85));
    let p = plan(&act, &fs);
    assert_eq!(p.len(), 3);
    let finals: Vec<PathBuf> = p.iter().map(|(_, f)| f.clone()).collect();
    assert_eq!(
        finals,
        [
            d.path().join("a.webp"),
            d.path().join("b.webp"),
            sub.join("c.webp")
        ]
    );
    for ((task, fin), input) in p.iter().zip(&fs) {
        assert_eq!(task.inputs, std::slice::from_ref(input));
        assert_eq!(task.action_id, "test.copy");
        assert_eq!(task.op, "copy");
        assert_eq!(task.param_i64("quality"), Some(85));
        assert_eq!(task.output.parent(), fin.parent());
        assert_eq!(task.output.extension().unwrap(), "webp");
        assert!(
            task.output
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with('.')
        );
        assert!(!task.output.exists());
    }
    // Pure: nothing written.
    assert_eq!(listing(d.path()), ["a.png", "b.jpg", "sub"]);
}

#[test]
fn ac1_plan_combined_single_task_named_after_first() {
    let d = tempfile::tempdir().unwrap();
    let fs = files(d.path(), &["z.png", "a.png", "m.png"]);
    let p = plan(&action("fake", Combined, "pdf", ""), &fs);
    assert_eq!(p.len(), 1);
    assert_eq!(p[0].0.inputs, fs);
    assert_eq!(p[0].1, d.path().join("z.pdf"));
    assert_eq!(p[0].0.output.parent(), Some(d.path()));
}

#[test]
fn ac1_plan_never_repeats_a_final_name() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("a.webp"), b"old").unwrap();
    let fs = files(d.path(), &["a.png", "a.jpg", "a.gif"]);
    let p = plan(&action("fake", PerFile, "webp", ""), &fs);
    let finals: Vec<PathBuf> = p.iter().map(|(_, f)| f.clone()).collect();
    assert_eq!(
        finals,
        [
            d.path().join("a (1).webp"),
            d.path().join("a (2).webp"),
            d.path().join("a (3).webp")
        ]
    );
    let temps: std::collections::HashSet<_> = p.iter().map(|(t, _)| t.output.clone()).collect();
    assert_eq!(temps.len(), 3);
}

#[test]
fn ac1_plan_same_name_in_different_dirs_is_not_a_clash() {
    let d = tempfile::tempdir().unwrap();
    let (x, y) = (d.path().join("x"), d.path().join("y"));
    std::fs::create_dir(&x).unwrap();
    std::fs::create_dir(&y).unwrap();
    let fs = [files(&x, &["a.png"]), files(&y, &["a.png"])].concat();
    let p = plan(&action("fake", PerFile, "webp", ""), &fs);
    assert_eq!(p[0].1, x.join("a.webp"));
    assert_eq!(p[1].1, y.join("a.webp"));
}

#[test]
fn ac1_plan_empty_selection_has_no_tasks() {
    assert!(plan(&action("fake", PerFile, "webp", ""), &[]).is_empty());
    assert!(plan(&action("fake", Combined, "pdf", ""), &[]).is_empty());
}

// ---------- run_job ----------

#[test]
fn ac2_run_job_event_sequence_and_outputs() {
    let d = tempfile::tempdir().unwrap();
    let fs = files(d.path(), &["a.png", "bad.png", "c.png"]);
    let eng = FakeEngine {
        fail_on: Some("bad"),
        ..FakeEngine::new()
    };
    let (tx, rx) = channel();
    let out = run_job(
        7,
        &action("fake", PerFile, "webp", ""),
        &fs,
        &eng,
        &CancelToken::new(),
        &tx,
    );
    assert_eq!(out, [d.path().join("a.webp"), d.path().join("c.webp")]);
    let ev = drain(&rx);
    assert_eq!(
        ev[0],
        JobEvent::Started {
            job: 7,
            action_id: "test.copy".into(),
            tasks: 3
        }
    );
    let important: Vec<&JobEvent> = ev
        .iter()
        .filter(|e| !matches!(e, JobEvent::Progress { .. }))
        .collect();
    assert_eq!(important.len(), 5);
    assert_eq!(
        *important[1],
        JobEvent::TaskDone {
            job: 7,
            output: d.path().join("a.webp")
        }
    );
    match important[2] {
        JobEvent::TaskFailed { job, input, error } => {
            assert_eq!(*job, 7);
            assert_eq!(*input, fs[1]);
            assert!(error.contains("fake failure"), "{error}");
        }
        e => panic!("unexpected {e:?}"),
    }
    assert_eq!(
        *important[3],
        JobEvent::TaskDone {
            job: 7,
            output: d.path().join("c.webp")
        }
    );
    assert_eq!(
        *ev.last().unwrap(),
        JobEvent::Finished {
            job: 7,
            ok: 2,
            failed: 1,
            cancelled: false
        }
    );
    assert_eq!(
        std::fs::read_to_string(d.path().join("c.webp")).unwrap(),
        "c.png"
    );
    assert_eq!(
        listing(d.path()),
        ["a.png", "a.webp", "bad.png", "c.png", "c.webp"]
    );
}

#[test]
fn ac2_run_job_combined_and_empty() {
    let d = tempfile::tempdir().unwrap();
    let fs = files(d.path(), &["p1.png", "p2.png"]);
    let eng = FakeEngine::new();
    let (tx, rx) = channel();
    let out = run_job(
        1,
        &action("fake", Combined, "pdf", ""),
        &fs,
        &eng,
        &CancelToken::new(),
        &tx,
    );
    assert_eq!(out, [d.path().join("p1.pdf")]);
    assert_eq!(std::fs::read_to_string(&out[0]).unwrap(), "p1.png\np2.png");
    assert_eq!(finished_of(&drain(&rx), 1), (1, 0, false));

    let out = run_job(
        2,
        &action("fake", PerFile, "webp", ""),
        &[],
        &eng,
        &CancelToken::new(),
        &tx,
    );
    assert!(out.is_empty());
    let ev = drain(&rx);
    assert_eq!(
        ev.first().unwrap(),
        &JobEvent::Started {
            job: 2,
            action_id: "test.copy".into(),
            tasks: 0
        }
    );
    assert_eq!(finished_of(&ev, 2), (0, 0, false));
}

#[test]
fn ac2_run_job_survives_dropped_receiver() {
    let d = tempfile::tempdir().unwrap();
    let fs = files(d.path(), &["a.png"]);
    let (tx, rx) = channel();
    drop(rx);
    let out = run_job(
        1,
        &action("fake", PerFile, "webp", ""),
        &fs,
        &FakeEngine::new(),
        &CancelToken::new(),
        &tx,
    );
    assert_eq!(out, [d.path().join("a.webp")]);
}

#[test]
fn ac2_engine_without_output_file_fails_the_task() {
    struct Lazy;
    impl Engine for Lazy {
        fn id(&self) -> &'static str {
            "fake"
        }
        fn capabilities(&self) -> Vec<(&'static str, Vec<sbarhive_core::MediaKind>)> {
            vec![]
        }
        fn run(
            &self,
            _: &sbarhive_core::Task,
            p: sbarhive_core::Progress,
            _: &CancelToken,
        ) -> Result<(), sbarhive_core::EngineError> {
            p(1.0);
            Ok(())
        }
    }
    let d = tempfile::tempdir().unwrap();
    let fs = files(d.path(), &["a.png"]);
    let (tx, rx) = channel();
    let out = run_job(
        1,
        &action("fake", PerFile, "webp", ""),
        &fs,
        &Lazy,
        &CancelToken::new(),
        &tx,
    );
    assert!(out.is_empty());
    assert_eq!(finished_of(&drain(&rx), 1), (0, 1, false));
    assert_eq!(listing(d.path()), ["a.png"]);
}

#[test]
fn ac3_progress_is_monotonic_weighted_and_ends_at_one() {
    let d = tempfile::tempdir().unwrap();
    let fs = files(d.path(), &["a.png", "b.png", "c.png", "d.png"]);
    let (tx, rx) = channel();
    run_job(
        3,
        &action("fake", PerFile, "webp", ""),
        &fs,
        &FakeEngine::new(),
        &CancelToken::new(),
        &tx,
    );
    let ev = drain(&rx);
    let p = progress_of(&ev, 3);
    assert!(!p.is_empty());
    for w in p.windows(2) {
        assert!(w[0] < w[1], "{p:?}");
    }
    assert!(p.iter().all(|f| (0.0..=1.0).contains(f)));
    assert_eq!(*p.last().unwrap(), 1.0);
    // Engine reports 0.5 inside the first of 4 tasks: overall 0.125.
    assert!(p.contains(&0.125), "{p:?}");
    assert!(p.contains(&0.25), "{p:?}");
    assert!(p.contains(&0.625), "{p:?}");
    // Finished is the last event, after every Progress.
    assert!(matches!(ev.last(), Some(JobEvent::Finished { .. })));
}

#[test]
fn ac3_progress_ignores_regressions_and_out_of_range_reports() {
    struct Wobbly;
    impl Engine for Wobbly {
        fn id(&self) -> &'static str {
            "fake"
        }
        fn capabilities(&self) -> Vec<(&'static str, Vec<sbarhive_core::MediaKind>)> {
            vec![]
        }
        fn run(
            &self,
            t: &sbarhive_core::Task,
            p: sbarhive_core::Progress,
            _: &CancelToken,
        ) -> Result<(), sbarhive_core::EngineError> {
            p(0.8);
            p(0.4);
            p(-3.0);
            p(f32::NAN);
            p(7.0);
            std::fs::write(&t.output, b"x")?;
            Ok(())
        }
    }
    let d = tempfile::tempdir().unwrap();
    let fs = files(d.path(), &["a.png", "b.png"]);
    let (tx, rx) = channel();
    run_job(
        1,
        &action("fake", PerFile, "webp", ""),
        &fs,
        &Wobbly,
        &CancelToken::new(),
        &tx,
    );
    let p = progress_of(&drain(&rx), 1);
    assert_eq!(p, [0.4, 0.5, 0.9, 1.0]);
}

#[test]
fn ac3_progress_advances_per_task_without_engine_reports() {
    struct Silent;
    impl Engine for Silent {
        fn id(&self) -> &'static str {
            "fake"
        }
        fn capabilities(&self) -> Vec<(&'static str, Vec<sbarhive_core::MediaKind>)> {
            vec![]
        }
        fn run(
            &self,
            t: &sbarhive_core::Task,
            _: sbarhive_core::Progress,
            _: &CancelToken,
        ) -> Result<(), sbarhive_core::EngineError> {
            std::fs::write(&t.output, b"x")?;
            Ok(())
        }
    }
    let d = tempfile::tempdir().unwrap();
    let fs = files(d.path(), &["a.png", "b.png", "c.png", "d.png"]);
    let (tx, rx) = channel();
    run_job(
        1,
        &action("fake", PerFile, "webp", ""),
        &fs,
        &Silent,
        &CancelToken::new(),
        &tx,
    );
    assert_eq!(progress_of(&drain(&rx), 1), [0.25, 0.5, 0.75, 1.0]);
}

#[test]
fn ac4_failure_and_panic_leave_no_files() {
    let d = tempfile::tempdir().unwrap();
    let fs = files(d.path(), &["bad.png", "boom.png"]);
    let eng = FakeEngine {
        fail_on: Some("bad"),
        panic_on: Some("boom"),
        ..FakeEngine::new()
    };
    let (tx, rx) = channel();
    let out = run_job(
        1,
        &action("fake", PerFile, "webp", ""),
        &fs,
        &eng,
        &CancelToken::new(),
        &tx,
    );
    assert!(out.is_empty());
    let ev = drain(&rx);
    assert_eq!(finished_of(&ev, 1), (0, 2, false));
    let panic_msg = ev
        .iter()
        .find_map(|e| match e {
            JobEvent::TaskFailed { input, error, .. } if *input == fs[1] => Some(error.clone()),
            _ => None,
        })
        .unwrap();
    assert!(panic_msg.contains("fake engine panic"), "{panic_msg}");
    assert_eq!(listing(d.path()), ["bad.png", "boom.png"]);
    assert_eq!(eng.calls(), 2);
}

#[test]
fn ac5_cancel_mid_task_stops_and_cleans_up() {
    let d = tempfile::tempdir().unwrap();
    let fs = files(d.path(), &["a.png", "b.png", "c.png"]);
    let (started_tx, started_rx) = channel();
    let eng = Arc::new(FakeEngine {
        slow: Some(Duration::from_secs(10)),
        started: std::sync::Mutex::new(Some(started_tx)),
        ..FakeEngine::new()
    });
    let cancel = CancelToken::new();
    let (tx, rx) = channel();
    let h = {
        let (eng, cancel, fs) = (eng.clone(), cancel.clone(), fs.clone());
        std::thread::spawn(move || {
            run_job(
                5,
                &action("fake", PerFile, "webp", ""),
                &fs,
                eng.as_ref(),
                &cancel,
                &tx,
            )
        })
    };
    assert_eq!(started_rx.recv_timeout(WAIT).unwrap(), fs[0]);
    cancel.cancel();
    let out = h.join().unwrap();
    assert!(out.is_empty());
    let ev = drain(&rx);
    assert_eq!(finished_of(&ev, 5), (0, 1, true));
    let failed: Vec<&JobEvent> = ev
        .iter()
        .filter(|e| matches!(e, JobEvent::TaskFailed { .. }))
        .collect();
    assert_eq!(failed.len(), 1);
    assert!(
        matches!(failed[0], JobEvent::TaskFailed { error, input, .. } if error == "cancelled" && *input == fs[0])
    );
    assert_eq!(eng.calls(), 1);
    assert_eq!(listing(d.path()), ["a.png", "b.png", "c.png"]);
}

#[test]
fn ac5_cancel_between_tasks_skips_the_rest() {
    let d = tempfile::tempdir().unwrap();
    let fs = files(d.path(), &["a.png", "b.png"]);
    // Cancels the token from inside the first task, after it succeeded.
    struct CancelAfterFirst(CancelToken);
    impl Engine for CancelAfterFirst {
        fn id(&self) -> &'static str {
            "fake"
        }
        fn capabilities(&self) -> Vec<(&'static str, Vec<sbarhive_core::MediaKind>)> {
            vec![]
        }
        fn run(
            &self,
            t: &sbarhive_core::Task,
            p: sbarhive_core::Progress,
            _: &CancelToken,
        ) -> Result<(), sbarhive_core::EngineError> {
            std::fs::write(&t.output, b"x")?;
            p(1.0);
            self.0.cancel();
            Ok(())
        }
    }
    let cancel = CancelToken::new();
    let (tx, rx) = channel();
    let out = run_job(
        1,
        &action("fake", PerFile, "webp", ""),
        &fs,
        &CancelAfterFirst(cancel.clone()),
        &cancel,
        &tx,
    );
    assert_eq!(out, [d.path().join("a.webp")]);
    let ev = drain(&rx);
    assert_eq!(finished_of(&ev, 1), (1, 0, true));
    assert_eq!(progress_of(&ev, 1).last(), Some(&0.5));
    assert_eq!(listing(d.path()), ["a.png", "a.webp", "b.png"]);
}

#[test]
fn ac5_cancelled_before_start_runs_nothing() {
    let d = tempfile::tempdir().unwrap();
    let fs = files(d.path(), &["a.png"]);
    let eng = FakeEngine::new();
    let cancel = CancelToken::new();
    cancel.cancel();
    let (tx, rx) = channel();
    let out = run_job(
        1,
        &action("fake", PerFile, "webp", ""),
        &fs,
        &eng,
        &cancel,
        &tx,
    );
    assert!(out.is_empty());
    assert_eq!(finished_of(&drain(&rx), 1), (0, 0, true));
    assert_eq!(eng.calls(), 0);
}

#[test]
fn ac6_unavailable_engine_fails_every_task_without_running() {
    let d = tempfile::tempdir().unwrap();
    let fs = files(d.path(), &["a.png", "b.png"]);
    let eng = FakeEngine {
        unavailable: true,
        ..FakeEngine::new()
    };
    let (tx, rx) = channel();
    let out = run_job(
        1,
        &action("fake", PerFile, "webp", ""),
        &fs,
        &eng,
        &CancelToken::new(),
        &tx,
    );
    assert!(out.is_empty());
    let ev = drain(&rx);
    assert_eq!(finished_of(&ev, 1), (0, 2, false));
    let inputs: Vec<PathBuf> = ev
        .iter()
        .filter_map(|e| match e {
            JobEvent::TaskFailed { input, .. } => Some(input.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(inputs, fs);
    assert_eq!(eng.calls(), 0);
    assert_eq!(listing(d.path()), ["a.png", "b.png"]);
}

// ---------- Queue ----------

#[test]
fn ac6_queue_unknown_engine_fails_every_task() {
    let d = tempfile::tempdir().unwrap();
    let fs = files(d.path(), &["a.png", "b.png", "c.png"]);
    let eng = Arc::new(FakeEngine::new());
    let (tx, rx) = channel();
    let q = Queue::new(vec![eng.clone()], 1, tx);
    let job = q.submit(action("missing", PerFile, "webp", ""), fs);
    let ev = until_finished(&rx, 1);
    assert_eq!(finished_of(&ev, job), (0, 3, false));
    assert_eq!(
        ev.iter()
            .filter(
                |e| matches!(e, JobEvent::TaskFailed { error, .. } if error.contains("missing"))
            )
            .count(),
        3
    );
    assert_eq!(eng.calls(), 0);
}

#[test]
fn ac6_queue_unavailable_engine_fails_every_task() {
    let d = tempfile::tempdir().unwrap();
    let fs = files(d.path(), &["a.png", "b.png"]);
    let eng = Arc::new(FakeEngine {
        unavailable: true,
        ..FakeEngine::new()
    });
    let (tx, rx) = channel();
    let q = Queue::new(vec![eng.clone()], 1, tx);
    let job = q.submit(action("fake", PerFile, "webp", ""), fs);
    let ev = until_finished(&rx, 1);
    assert_eq!(finished_of(&ev, job), (0, 2, false));
    assert_eq!(eng.calls(), 0);
    assert_eq!(listing(d.path()), ["a.png", "b.png"]);
}

#[test]
fn ac7_queue_runs_jobs_and_ids_increase() {
    let d = tempfile::tempdir().unwrap();
    let a = files(d.path(), &["a.png"]);
    let b = files(d.path(), &["b.png"]);
    let eng: Arc<dyn Engine> = Arc::new(FakeEngine::new());
    let (tx, rx) = channel();
    let q = Queue::new(vec![eng], 2, tx);
    let j1 = q.submit(action("fake", PerFile, "webp", ""), a);
    let j2 = q.submit(action("fake", PerFile, "webp", ""), b);
    assert!(j2 > j1);
    let ev = until_finished(&rx, 2);
    assert_eq!(finished_of(&ev, j1), (1, 0, false));
    assert_eq!(finished_of(&ev, j2), (1, 0, false));
    assert_eq!(q.active(), 0);
    assert_eq!(listing(d.path()), ["a.png", "a.webp", "b.png", "b.webp"]);
}

#[test]
fn ac7_two_workers_run_jobs_concurrently() {
    let d = tempfile::tempdir().unwrap();
    // The barrier only opens when two tasks run at the same time.
    let eng = Arc::new(FakeEngine {
        barrier: Some(Barrier::new(2)),
        ..FakeEngine::new()
    });
    let (tx, rx) = channel();
    let q = Queue::new(vec![eng.clone()], 2, tx);
    q.submit(
        action("fake", PerFile, "webp", ""),
        files(d.path(), &["a.png"]),
    );
    q.submit(
        action("fake", PerFile, "webp", ""),
        files(d.path(), &["b.png"]),
    );
    until_finished(&rx, 2);
    assert_eq!(eng.max_running.load(std::sync::atomic::Ordering::SeqCst), 2);
}

#[test]
fn ac7_one_worker_runs_jobs_in_sequence() {
    let d = tempfile::tempdir().unwrap();
    let eng = Arc::new(FakeEngine {
        slow: Some(Duration::from_millis(50)),
        ..FakeEngine::new()
    });
    let (tx, rx) = channel();
    // workers = 0 is raised to 1.
    let q = Queue::new(vec![eng.clone()], 0, tx);
    for n in ["a.png", "b.png", "c.png"] {
        q.submit(action("fake", PerFile, "webp", ""), files(d.path(), &[n]));
    }
    let ev = until_finished(&rx, 3);
    assert_eq!(eng.max_running.load(std::sync::atomic::Ordering::SeqCst), 1);
    // Jobs never interleave: each Started is followed by its own Finished.
    let order: Vec<u64> = ev
        .iter()
        .filter_map(|e| match e {
            JobEvent::Started { job, .. } | JobEvent::Finished { job, .. } => Some(*job),
            _ => None,
        })
        .collect();
    assert_eq!(order, [1, 1, 2, 2, 3, 3]);
}

#[test]
fn ac7_submit_returns_immediately() {
    let d = tempfile::tempdir().unwrap();
    let eng = Arc::new(FakeEngine {
        slow: Some(Duration::from_secs(10)),
        ..FakeEngine::new()
    });
    let (tx, rx) = channel();
    let q = Queue::new(vec![eng], 1, tx);
    let t = std::time::Instant::now();
    q.submit(
        action("fake", PerFile, "webp", ""),
        files(d.path(), &["a.png"]),
    );
    q.submit(
        action("fake", PerFile, "webp", ""),
        files(d.path(), &["b.png"]),
    );
    assert!(t.elapsed() < Duration::from_secs(2));
    assert_eq!(q.active(), 2);
    q.cancel_all();
    until_finished(&rx, 2);
}

#[test]
fn ac8_cancel_one_job_only() {
    let d = tempfile::tempdir().unwrap();
    let (started_tx, started_rx) = channel();
    let eng = Arc::new(FakeEngine {
        slow: Some(Duration::from_secs(10)),
        started: std::sync::Mutex::new(Some(started_tx)),
        ..FakeEngine::new()
    });
    let (tx, rx) = channel();
    let q = Queue::new(vec![eng], 2, tx);
    let j1 = q.submit(
        action("fake", PerFile, "webp", ""),
        files(d.path(), &["a.png"]),
    );
    let j2 = q.submit(
        action("fake", PerFile, "webp", ""),
        files(d.path(), &["b.png"]),
    );
    started_rx.recv_timeout(WAIT).unwrap();
    started_rx.recv_timeout(WAIT).unwrap();
    assert_eq!(q.active(), 2);
    q.cancel(j1);
    let ev = until_finished(&rx, 1);
    assert_eq!(finished_of(&ev, j1), (0, 1, true));
    assert_eq!(q.active(), 1);
    q.cancel(999);
    q.cancel_all();
    let ev = until_finished(&rx, 1);
    assert_eq!(finished_of(&ev, j2), (0, 1, true));
    assert_eq!(q.active(), 0);
    assert_eq!(listing(d.path()), ["a.png", "b.png"]);
}

#[test]
fn ac8_cancel_all_includes_waiting_jobs() {
    let d = tempfile::tempdir().unwrap();
    let (started_tx, started_rx) = channel();
    let eng = Arc::new(FakeEngine {
        slow: Some(Duration::from_secs(10)),
        started: std::sync::Mutex::new(Some(started_tx)),
        ..FakeEngine::new()
    });
    let (tx, rx) = channel();
    let q = Queue::new(vec![eng.clone()], 1, tx);
    let jobs: Vec<u64> = ["a.png", "b.png", "c.png"]
        .iter()
        .map(|n| q.submit(action("fake", PerFile, "webp", ""), files(d.path(), &[n])))
        .collect();
    started_rx.recv_timeout(WAIT).unwrap();
    assert_eq!(q.active(), 3);
    q.cancel_all();
    let ev = until_finished(&rx, 3);
    assert_eq!(finished_of(&ev, jobs[0]), (0, 1, true));
    assert_eq!(finished_of(&ev, jobs[1]), (0, 0, true));
    assert_eq!(finished_of(&ev, jobs[2]), (0, 0, true));
    assert_eq!(eng.calls(), 1);
    assert_eq!(q.active(), 0);
}

#[test]
fn ac8_dropping_the_queue_cancels_running_jobs() {
    let d = tempfile::tempdir().unwrap();
    let (started_tx, started_rx) = channel();
    let eng = Arc::new(FakeEngine {
        slow: Some(Duration::from_secs(10)),
        started: std::sync::Mutex::new(Some(started_tx)),
        ..FakeEngine::new()
    });
    let (tx, rx) = channel();
    let q = Queue::new(vec![eng], 1, tx);
    let j = q.submit(
        action("fake", PerFile, "webp", ""),
        files(d.path(), &["a.png"]),
    );
    started_rx.recv_timeout(WAIT).unwrap();
    let t = std::time::Instant::now();
    drop(q);
    assert!(t.elapsed() < Duration::from_secs(5));
    let ev = until_finished(&rx, 1);
    assert_eq!(finished_of(&ev, j), (0, 1, true));
    assert_eq!(listing(d.path()), ["a.png"]);
}

#[test]
fn ac9_engine_available() {
    let ok: Arc<dyn Engine> = Arc::new(FakeEngine::new());
    let off: Arc<dyn Engine> = Arc::new(FakeEngine {
        id: "off",
        unavailable: true,
        ..FakeEngine::new()
    });
    let (tx, _rx) = channel();
    let q = Queue::new(vec![ok, off], 1, tx);
    assert!(q.engine_available("fake"));
    assert!(!q.engine_available("off"));
    assert!(!q.engine_available("nope"));
}
