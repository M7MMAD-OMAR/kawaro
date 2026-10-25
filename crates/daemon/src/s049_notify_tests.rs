//! S-049 AC5 (backlog/S-049-windows-notifications.md): one job that ends
//! writes exactly one `kawarod: notify:` line. The job really runs (the
//! image engine on a fixture, on this thread) and its events go through the
//! daemon's own event pump, `jobs::pump`, in process. `open_folder_when_done`
//! is off, and a test build never reaches the notification service.
//!
//! Contract (tester, from the acceptance criteria; do not edit):
//! - `mod notify_logic;` in main.rs with `count_notify_lines` (AC4).
//! - In test builds, `crate::notify::captured_log() -> String` returns every
//!   `kawarod: notify:` line written so far on the calling thread, each
//!   followed by `"\n"`, in the `notify_logic::log_line` format. The lines
//!   still go to stderr as today. (Thread local, so parallel tests do not mix.)

use crate::jobs::pump;
use crate::notify::captured_log;
use crate::notify_logic::count_notify_lines;
use crate::state::{JobInfo, State};
use kawaro_core::Registry;
use kawaro_core::config::Config;
use kawaro_core::engine::CancelToken;
use kawaro_core::queue::run_job;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/images")
        .join(name)
}

#[test]
fn ac5_one_finished_job_writes_exactly_one_notify_line() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("photo.jpg");
    std::fs::copy(fixture("orient6.jpg"), &input).unwrap();

    let config = Config {
        notify: true,
        open_folder_when_done: false,
        ..Config::default()
    };
    let engines = kawaro_engines::all();
    let state = Arc::new(Mutex::new(State::new(config, &engines)));
    let action = {
        let def = Registry::builtin()
            .get("image.to-png")
            .expect("built in action image.to-png")
            .clone();
        state.lock().unwrap().configured(def)
    };
    let engine = engines
        .iter()
        .find(|e| e.id() == action.engine)
        .expect("the engine of image.to-png");
    const JOB: u64 = 49;
    state.lock().unwrap().jobs.insert(
        JOB,
        JobInfo {
            label: "PNG".into(),
            tasks: 0,
            fraction: 0.0,
            ok: 0,
            failed: 0,
            first_output: None,
            first_error: None,
        },
    );

    let before = count_notify_lines(&captured_log());
    let (tx, rx) = channel();
    let outputs = run_job(
        JOB,
        &action,
        std::slice::from_ref(&input),
        engine.as_ref(),
        &CancelToken::new(),
        &tx,
    );
    drop(tx);
    assert_eq!(outputs.len(), 1, "the job must produce one file");
    assert!(outputs[0].is_file(), "{}", outputs[0].display());

    pump(rx, state.clone(), None);

    let log = captured_log();
    assert_eq!(count_notify_lines(&log) - before, 1, "captured log:\n{log}");
    assert!(state.lock().unwrap().jobs.is_empty(), "the job is done");
}

/// AC5 follow-up: `notify::info` writes exactly one `kawarod: notify:`
/// line carrying its body, unchanged.
#[test]
fn ac5_info_writes_exactly_one_notify_line_with_its_body() {
    const BODY: &str = "s049 info body, صبار";
    let before = captured_log();
    crate::notify::info(kawaro_core::i18n::Lang::En, BODY);
    let log = captured_log();
    let added = &log[before.len()..];
    assert_eq!(count_notify_lines(added), 1, "captured log:\n{log}");
    assert_eq!(
        added,
        format!("{}\n", crate::notify_logic::log_line(BODY)),
        "captured log:\n{log}"
    );
    assert_eq!(added, format!("kawarod: notify: {BODY}\n"));
}
