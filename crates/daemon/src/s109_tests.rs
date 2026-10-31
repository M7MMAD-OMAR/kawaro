//! S-109 AC4 (backlog/S-109-first-run-remainder.md): failures and batches
//! under the three notification modes, through the daemon's own event pump
//! with a fake clock (the `elapsed` of each event is a plain `Duration`).
//! Written from the acceptance criteria only. The mode comes from
//! `Config.notify_mode` alone; the two legacy fields are left at their
//! defaults, so a daemon that still reads them gives the wrong counts.

use crate::jobs::pump;
use crate::notify::captured_log;
use crate::state::{JobInfo, State};
use kawaro_core::config::{Config, NotifyMode};
use kawaro_core::queue::JobEvent;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn run(mode: NotifyMode, events: Vec<JobEvent>, tasks: usize) -> usize {
    let state = Arc::new(Mutex::new(State::new(
        Config {
            notify_mode: mode,
            open_folder_when_done: false,
            ..Config::default()
        },
        &[],
    )));
    state.lock().unwrap().jobs.insert(
        1,
        JobInfo {
            label: "test".into(),
            tasks: 0,
            fraction: 0.0,
            ok: 0,
            failed: 0,
            first_output: None,
            first_error: None,
        },
    );
    let (tx, rx) = channel();
    tx.send(JobEvent::Started {
        job: 1,
        action_id: "test".into(),
        tasks,
    })
    .unwrap();
    for e in events {
        tx.send(e).unwrap();
    }
    tx.send(JobEvent::Finished {
        job: 1,
        ok: 0,
        failed: 0,
        cancelled: false,
        elapsed: Duration::from_secs(40),
    })
    .unwrap();
    drop(tx);
    let before = captured_log().lines().count();
    pump(rx, state, None);
    captured_log().lines().count() - before
}

fn done(name: &str, seconds: u64) -> JobEvent {
    JobEvent::TaskDone {
        job: 1,
        output: PathBuf::from(name),
        elapsed: Duration::from_secs(seconds),
    }
}

fn failed(name: &str, seconds: u64, cancelled: bool) -> JobEvent {
    JobEvent::TaskFailed {
        job: 1,
        input: name.into(),
        error: "error".into(),
        elapsed: Duration::from_secs(seconds),
        cancelled,
    }
}

#[test]
fn ac4_long_notifies_a_twelve_second_failure_and_not_a_three_second_one() {
    assert_eq!(run(NotifyMode::Long, vec![failed("slow", 12, false)], 1), 1);
    assert_eq!(run(NotifyMode::Long, vec![failed("quick", 3, false)], 1), 0);
}

#[test]
fn ac4_a_cancelled_task_never_notifies_in_any_mode() {
    for mode in [NotifyMode::Off, NotifyMode::All, NotifyMode::Long] {
        assert_eq!(run(mode, vec![failed("c", 12, true)], 1), 0, "{mode:?}");
    }
}

#[test]
fn ac4_a_batch_of_four_gives_0_3_and_2_notices() {
    for (mode, expected) in [
        (NotifyMode::Off, 0),
        (NotifyMode::All, 3),
        (NotifyMode::Long, 2),
    ] {
        let events = vec![
            done("fast", 1),
            done("slow", 10),
            failed("broken", 12, false),
            failed("cancelled", 12, true),
        ];
        assert_eq!(run(mode, events, 4), expected, "{mode:?}");
    }
}
