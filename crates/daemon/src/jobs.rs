//! Job bookkeeping: submission and the event pump that feeds clients, the
//! tray and the end-of-job notification.

use crate::notify_logic::open_folder_command;
use crate::state::{JobInfo, RetryRefusal, Shared, State, lock};
use crate::{notify, tray};
use kawaro_core::ActionDef;
use kawaro_core::config::{NotifyMode, notify_decision};
use kawaro_core::ipc::Event;
use kawaro_core::job_report::{JobDetail, JobState, JobSummary};
use kawaro_core::job_results::{UnitOutcome, current_identity};
use kawaro_core::paths::CURRENT;
use kawaro_core::queue::{JobEvent, JobId, Queue};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;

pub fn submit(st: &mut State, queue: &Queue, action: ActionDef, files: Vec<PathBuf>) -> JobId {
    let label = action.label.get(st.lang).to_string();
    let action = st.configured(action);
    submit_resolved(st, queue, action, files, label)
}

/// Submits an action that is already resolved. The settings are not applied
/// again, so a retry runs the action exactly as it ran before (S-144).
pub fn submit_resolved(
    st: &mut State,
    queue: &Queue,
    action: ActionDef,
    files: Vec<PathBuf>,
    label: String,
) -> JobId {
    let job = queue.submit(action, files);
    st.jobs.entry(job).or_insert(JobInfo {
        label,
        tasks: 0,
        fraction: 0.0,
        ok: 0,
        failed: 0,
        first_output: None,
        first_error: None,
        detail: None,
        skipped: 0,
        first_skipped: None,
    });
    job
}

/// Submits a retry of a finished job (S-144).
///
/// The proposal comes from the ledger: it holds the action as it ran and
/// the inputs of the failed units, in the order of the original manifest.
/// The current settings are not applied again, so a changed configuration
/// never rewrites a retry. A retry that is already pending is refused, so a
/// double trigger never starts two child jobs.
pub fn retry(st: &mut State, queue: &Queue, job: JobId) -> Result<JobId, RetryRefusal> {
    if st.pending_retries.contains_key(&job) {
        return Err(RetryRefusal::AlreadySubmitting);
    }
    let action_id = match st.results.job(job) {
        Some(record) => record.action_id.clone(),
        // A job the queue holds and the ledger has no manifest for has not
        // finished, so it is not a missing job.
        None if st.jobs.contains_key(&job) => {
            return Err(RetryRefusal::from(
                kawaro_core::job_results::RetryRefusal::NotCompleted,
            ));
        }
        None => {
            return Err(RetryRefusal::from(
                kawaro_core::job_results::RetryRefusal::MissingJob,
            ));
        }
    };
    // The action as it would run now, so the settings the daemon applies
    // are already in it and are not read as a change of the action file.
    let registered = st.registry.get(&action_id).cloned();
    let current = registered.map(|action| st.configured(action));
    let engines = st.available.clone();
    let proposal = {
        let engine_ok = |id: &str| engines.iter().any(|a| a.as_str() == id);
        let identity = |path: &Path| current_identity(path);
        st.results
            .retry_plan(job, current.as_ref(), &engine_ok, &identity)?
    };
    let label = proposal.action.label.get(st.lang).to_string();
    let child = submit_resolved(st, queue, proposal.action, proposal.inputs, label);
    st.retry_parents.insert(child, job);
    st.pending_retries.insert(job, child);
    Ok(child)
}

/// The label of an action in the daemon's language, or its id when the
/// action is no longer in the registry (S-144-C). It is a display string
/// only: no `ActionDef` and no params leave the daemon through the results.
fn action_label(st: &State, action_id: &str) -> String {
    st.registry
        .get(action_id)
        .map(|action| action.label.get(st.lang).to_string())
        .unwrap_or_else(|| action_id.to_string())
}

/// The bounded summaries of the session, cloned while the caller holds the
/// state lock (S-144-C).
///
/// The order is stable: the running and queued jobs first, then the finished
/// ones, each group by job id. A job the queue holds and the ledger has no
/// manifest for is `Queued`: it is not a finished job and no count is
/// invented for it.
pub fn summaries(st: &State) -> Vec<JobSummary> {
    let mut out: Vec<JobSummary> = st
        .results
        .jobs()
        .map(|record| JobSummary::of(record, &action_label(st, &record.action_id)))
        .collect();
    let known: HashSet<JobId> = out.iter().map(|summary| summary.job).collect();
    for (job, info) in &st.jobs {
        if known.contains(job) {
            continue;
        }
        out.push(JobSummary::queued(*job, &info.label));
    }
    out.sort_by_key(|summary| (group(summary.state), summary.job));
    out
}

/// The bounded detail of one job, cloned while the caller holds the state
/// lock (S-144-C). A job the session does not know answers `Unknown`, never
/// a fabricated finished job.
pub fn detail(st: &State, job: JobId) -> JobDetail {
    if let Some(record) = st.results.job(job) {
        return JobDetail::of(record, &action_label(st, &record.action_id));
    }
    match st.jobs.get(&job) {
        Some(info) => JobDetail::queued(job, &info.label),
        None => JobDetail::unknown(job),
    }
}

/// The bounded details of every session job, in the same stable order as
/// [`summaries`], cloned while the caller holds the state lock (S-144).
pub fn details(st: &State) -> Vec<JobDetail> {
    summaries(st)
        .into_iter()
        .map(|summary| detail(st, summary.job))
        .collect()
}

/// Session snapshot for the Results surface: `(details, pending_retries, lang)`
/// built from the same bounded projection the IPC handlers use (S-144).
pub fn results_snapshot(st: &State) -> (Vec<JobDetail>, HashSet<JobId>, kawaro_core::i18n::Lang) {
    let list = details(st);
    let pending: HashSet<JobId> = st.pending_retries.keys().copied().collect();
    (list, pending, st.lang)
}

/// Removes every completed job from the session ledger (S-144). Active jobs
/// keep running and no output file is touched. Shared by the IPC handler and
/// the results surface.
pub fn clear_completed(st: &mut State) {
    st.results.clear_completed();
}

/// Asks the queue to cancel one job (S-144). Shared by the IPC handler and
/// the results surface.
pub fn cancel_job(queue: &Queue, job: JobId) {
    queue.cancel(job);
}

/// The sort group of a state: the jobs that are not finished yet first.
fn group(state: JobState) -> u8 {
    match state {
        JobState::Queued | JobState::Active => 0,
        _ => 1,
    }
}

/// Records what the queue reports into the ledger (S-144).
///
/// The unit of an event is the ordinal the queue put in it, so it never
/// depends on the order the events arrive in. A cancelled task is a
/// cancellation and not a failure, so it is never retried.
fn record(st: &mut State, event: &JobEvent) {
    match event {
        JobEvent::Started { job, manifest, .. } => st.results.start(*job, manifest.clone()),
        JobEvent::TaskDone {
            job,
            unit,
            output,
            elapsed,
            detail,
        } => {
            st.results.record_unit(
                *job,
                *unit,
                UnitOutcome::Success {
                    output: Some(output.clone()),
                    detail: detail.clone(),
                },
                *elapsed,
            );
        }
        JobEvent::TaskSkipped {
            job,
            unit,
            reason,
            elapsed,
            ..
        } => {
            st.results.record_unit(
                *job,
                *unit,
                UnitOutcome::Skipped { reason: *reason },
                *elapsed,
            );
        }
        JobEvent::TaskFailed {
            job,
            unit,
            error,
            elapsed,
            cancelled,
            ..
        } => {
            let outcome = if *cancelled {
                UnitOutcome::Cancelled
            } else {
                UnitOutcome::Failed {
                    error: error.clone(),
                }
            };
            st.results.record_unit(*job, *unit, outcome, *elapsed);
        }
        JobEvent::Finished {
            job,
            elapsed,
            cancelled,
            ..
        } => st.results.finish(*job, *elapsed, *cancelled),
        JobEvent::Progress { .. } => {}
    }
}

pub fn pump(events: Receiver<JobEvent>, state: Shared, tray: tray::TrayHandle) {
    for ev in events {
        let mut st = lock(&state);
        let results_changed = !matches!(ev, JobEvent::Progress { .. });
        let job = match &ev {
            JobEvent::Started { job, .. }
            | JobEvent::Progress { job, .. }
            | JobEvent::TaskDone { job, .. }
            | JobEvent::TaskSkipped { job, .. }
            | JobEvent::TaskFailed { job, .. }
            | JobEvent::Finished { job, .. } => *job,
        };
        // The ledger is filled from the events, before the transient job
        // info of the job goes away.
        record(&mut st, &ev);
        let mut finished = None;
        if let Some(info) = st.jobs.get_mut(&job) {
            match &ev {
                JobEvent::Started { tasks, .. } => info.tasks = *tasks,
                JobEvent::Progress { fraction, .. } => info.fraction = *fraction,
                JobEvent::TaskDone { output, detail, .. } => {
                    info.ok += 1;
                    info.first_output.get_or_insert_with(|| output.clone());
                    info.detail = detail.clone();
                }
                JobEvent::TaskSkipped { input, reason, .. } => {
                    // S-133: the reason is the closed enum SkipReason; today
                    // its only value is SkipReason::AlreadyOptimized, and the
                    // surfaces format it in their own language.
                    info.skipped += 1;
                    if info.first_skipped.is_none() {
                        info.first_skipped = Some((input.clone(), *reason));
                    }
                }
                JobEvent::TaskFailed { input, error, .. } => {
                    info.failed += 1;
                    info.first_error.get_or_insert_with(|| {
                        let name = input
                            .file_name()
                            .map(|n| n.to_string_lossy())
                            .unwrap_or_default();
                        format!("{name}: {error}")
                    });
                }
                JobEvent::Finished {
                    cancelled, elapsed, ..
                } => finished = Some((*cancelled, *elapsed)),
            }
        }
        // A combined action is one task and receives one notification. Failed
        // tasks are reported too; cancellations and tasks never started are not.
        let task_notice = st
            .jobs
            .get(&job)
            .and_then(|info| task_notice(&ev, &info.label, st.config.notify_mode));
        if let Some(notice) = task_notice {
            notify::finished(st.lang, &notice, false);
        }
        st.publish(job, Event::from(ev));
        if let Some((cancelled, _elapsed)) = finished {
            // A finished child releases the pending retry of its parent, so
            // the user may retry again as a new explicit choice.
            if let Some(parent) = st.retry_parents.remove(&job) {
                st.pending_retries.remove(&parent);
            }
            if let Some(info) = st.jobs.remove(&job)
                && st.config.open_folder_when_done
                && !cancelled
                && let Some(dir) = info
                    .first_output
                    .as_deref()
                    .and_then(std::path::Path::parent)
            {
                let file = info.first_output.as_deref();
                let (program, args) = open_folder_command(CURRENT, dir, file);
                notify::open_folder(program, args);
            }
        }
        let active: Vec<(String, f32)> = st
            .jobs
            .values()
            .map(|j| (j.label.clone(), j.fraction))
            .collect();
        drop(st);
        tray::set_active(&tray, active);
        if results_changed {
            crate::ui::notify_results_changed(&state);
        }
    }
}

/// A missing or cancelled task has no completion notice. Combined actions
/// represent one task, so they produce at most one notice for the batch.
fn task_notice(event: &JobEvent, label: &str, mode: NotifyMode) -> Option<JobInfo> {
    let (elapsed, output, error, detail, skipped) = match event {
        JobEvent::TaskDone {
            output,
            elapsed,
            detail,
            ..
        } => (*elapsed, Some(output.clone()), None, detail.clone(), None),
        JobEvent::TaskFailed {
            input,
            error,
            elapsed,
            cancelled: false,
            ..
        } => {
            let name = input
                .file_name()
                .map(|s| s.to_string_lossy())
                .unwrap_or_default();
            (*elapsed, None, Some(format!("{name}: {error}")), None, None)
        }
        JobEvent::TaskSkipped {
            input,
            reason,
            elapsed,
            ..
        } => (*elapsed, None, None, None, Some((input.clone(), *reason))),
        _ => return None,
    };
    if !notify_decision(mode, Some(elapsed)) {
        return None;
    }
    Some(JobInfo {
        label: label.to_string(),
        tasks: 1,
        fraction: 1.0,
        ok: usize::from(output.is_some()),
        failed: usize::from(error.is_some()),
        first_output: output,
        first_error: error,
        detail,
        skipped: usize::from(skipped.is_some()),
        first_skipped: skipped,
    })
}

#[cfg(test)]
mod tests {
    use kawaro_core::config::{NotifyMode, notify_decision};
    use kawaro_core::job_results::JobManifest;
    use std::path::PathBuf;
    use std::time::Duration;

    #[test]
    fn notification_policy_respects_duration_and_switch() {
        let off = NotifyMode::Off;
        let (all, long) = (NotifyMode::All, NotifyMode::Long);
        assert!(!notify_decision(off, Some(Duration::from_secs(30))));
        assert!(notify_decision(all, Some(Duration::from_secs(1))));
        assert!(!notify_decision(long, Some(Duration::from_secs(9))));
        assert!(notify_decision(long, Some(Duration::from_secs(10))));
        assert!(!notify_decision(long, None));
    }
    #[test]
    fn batch_notices_follow_each_task_duration_and_policy() {
        use super::{JobInfo, pump};
        use crate::{notify::captured_log, state::State};
        use kawaro_core::{config::Config, queue::JobEvent};
        use std::sync::{Arc, Mutex, mpsc::channel};

        /// The manifest of a job with four units (S-144).
        fn manifest_of(tasks: usize) -> JobManifest {
            let action = kawaro_core::Registry::builtin()
                .get("image.to-webp")
                .cloned()
                .expect("the built-in action");
            JobManifest::new(
                action,
                (0..tasks)
                    .map(|n| vec![PathBuf::from(format!("file{n}.png"))])
                    .collect(),
            )
        }

        for (mode, expected) in [
            (NotifyMode::Off, 0),
            (NotifyMode::All, 3),
            (NotifyMode::Long, 2),
        ] {
            let state = Arc::new(Mutex::new(State::new(
                Config {
                    notify_mode: mode,
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
                    detail: None,
                    skipped: 0,
                    first_skipped: None,
                },
            );
            let (tx, rx) = channel();
            tx.send(JobEvent::Started {
                job: 1,
                action_id: "test".into(),
                tasks: 4,
                manifest: manifest_of(4),
            })
            .unwrap();
            for (unit, (name, seconds)) in [(0u64, ("fast", 1)), (1, ("slow", 10))] {
                tx.send(JobEvent::TaskDone {
                    job: 1,
                    unit,
                    output: PathBuf::from(name),
                    elapsed: Duration::from_secs(seconds),
                    detail: None,
                })
                .unwrap();
            }
            tx.send(JobEvent::TaskFailed {
                job: 1,
                unit: 2,
                input: "broken".into(),
                error: "error".into(),
                elapsed: Duration::from_secs(12),
                cancelled: false,
            })
            .unwrap();
            tx.send(JobEvent::TaskFailed {
                job: 1,
                unit: 3,
                input: "cancelled".into(),
                error: "cancelled".into(),
                elapsed: Duration::from_secs(12),
                cancelled: true,
            })
            .unwrap();
            tx.send(JobEvent::Finished {
                job: 1,
                ok: 2,
                failed: 2,
                skipped: 0,
                cancelled: true,
                elapsed: Duration::from_secs(35),
            })
            .unwrap();
            drop(tx);
            let before = captured_log().lines().count();
            pump(rx, state.clone(), None);
            assert_eq!(
                captured_log().lines().count() - before,
                expected,
                "mode={mode:?}"
            );
            assert!(state.lock().unwrap().jobs.is_empty());
        }
    }

    /// The manifest of a job with the given units (S-144).
    fn manifest(action: kawaro_core::ActionDef, units: Vec<Vec<PathBuf>>) -> JobManifest {
        JobManifest::new(action, units)
    }

    /// The built-in action the retry cases use.
    fn builtin_action() -> kawaro_core::ActionDef {
        kawaro_core::Registry::builtin()
            .get("image.to-webp")
            .cloned()
            .expect("the built-in action")
    }

    #[test]
    fn the_pump_records_the_units_of_the_job_in_the_ledger() {
        use super::{JobInfo, pump};
        use crate::state::State;
        use kawaro_core::config::Config;
        use kawaro_core::job_results::UnitOutcome;
        use kawaro_core::queue::JobEvent;
        use std::sync::{Arc, Mutex, mpsc::channel};

        let state = Arc::new(Mutex::new(State::new(Config::default(), &[])));
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
                detail: None,
                skipped: 0,
                first_skipped: None,
            },
        );
        let (tx, rx) = channel();
        tx.send(JobEvent::Started {
            job: 1,
            action_id: "test".into(),
            tasks: 2,
            manifest: manifest(
                builtin_action(),
                vec![vec![PathBuf::from("a.png")], vec![PathBuf::from("b.png")]],
            ),
        })
        .unwrap();
        tx.send(JobEvent::TaskDone {
            job: 1,
            unit: 0,
            output: PathBuf::from("a.webp"),
            elapsed: Duration::from_secs(1),
            detail: None,
        })
        .unwrap();
        tx.send(JobEvent::TaskFailed {
            job: 1,
            unit: 1,
            input: PathBuf::from("b.png"),
            error: "boom".into(),
            elapsed: Duration::from_secs(1),
            cancelled: false,
        })
        .unwrap();
        tx.send(JobEvent::Finished {
            job: 1,
            ok: 1,
            failed: 1,
            skipped: 0,
            cancelled: false,
            elapsed: Duration::from_secs(2),
        })
        .unwrap();
        drop(tx);
        pump(rx, state.clone(), None);
        let st = state.lock().unwrap();
        let record = st.results.job(1).expect("the job is in the ledger");
        assert_eq!(record.counts.succeeded, 1);
        assert_eq!(record.counts.failed, 1);
        assert_eq!(record.units.len(), 2);
        assert!(matches!(
            record.units[0].outcome,
            UnitOutcome::Success { .. }
        ));
        assert!(matches!(
            record.units[1].outcome,
            UnitOutcome::Failed { .. }
        ));
    }

    #[test]
    fn a_cancelled_task_is_a_cancellation_and_not_a_failure() {
        use super::{JobInfo, pump};
        use crate::state::State;
        use kawaro_core::config::Config;
        use kawaro_core::job_results::UnitOutcome;
        use kawaro_core::queue::JobEvent;
        use std::sync::{Arc, Mutex, mpsc::channel};

        let state = Arc::new(Mutex::new(State::new(Config::default(), &[])));
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
                detail: None,
                skipped: 0,
                first_skipped: None,
            },
        );
        let (tx, rx) = channel();
        tx.send(JobEvent::Started {
            job: 1,
            action_id: "test".into(),
            tasks: 2,
            manifest: manifest(
                builtin_action(),
                vec![vec![PathBuf::from("a.png")], vec![PathBuf::from("b.png")]],
            ),
        })
        .unwrap();
        tx.send(JobEvent::TaskFailed {
            job: 1,
            unit: 0,
            input: PathBuf::from("a.png"),
            error: "cancelled".into(),
            elapsed: Duration::from_secs(1),
            cancelled: true,
        })
        .unwrap();
        tx.send(JobEvent::Finished {
            job: 1,
            ok: 0,
            failed: 1,
            skipped: 0,
            cancelled: true,
            elapsed: Duration::from_secs(2),
        })
        .unwrap();
        drop(tx);
        pump(rx, state.clone(), None);
        let st = state.lock().unwrap();
        let record = st.results.job(1).expect("the job is in the ledger");
        assert_eq!(record.counts.cancelled, 2);
        assert_eq!(record.counts.failed, 0);
        assert!(
            record
                .units
                .iter()
                .all(|unit| matches!(unit.outcome, UnitOutcome::Cancelled)),
            "{:?}",
            record.units
        );
    }

    #[test]
    fn a_retry_is_refused_while_one_is_pending() {
        use super::retry;
        use crate::state::{RetryRefusal, State};
        use kawaro_core::config::Config;
        use kawaro_core::job_results::UnitOutcome;
        use kawaro_core::queue::Queue;
        use std::sync::mpsc::channel;

        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.png");
        std::fs::write(&file, b"input").unwrap();
        let mut st = State::new(Config::default(), &[]);
        st.available.push(String::from("image"));
        let action = builtin_action();
        st.results
            .start(100, manifest(action, vec![vec![file.clone()]]));
        st.results.record_unit(
            100,
            0,
            UnitOutcome::Failed {
                error: String::from("boom"),
            },
            Duration::from_secs(1),
        );
        st.results.finish(100, Duration::from_secs(2), false);
        let (tx, _rx) = channel();
        let queue = Queue::new(vec![], 1, tx);
        let child = retry(&mut st, &queue, 100).expect("the retry runs");
        assert_ne!(child, 100);
        assert_eq!(st.retry_parents.get(&child), Some(&100));
        // A second trigger while the first retry is pending starts nothing.
        let refusal = retry(&mut st, &queue, 100).expect_err("no second child");
        assert_eq!(refusal, RetryRefusal::AlreadySubmitting);
        assert_eq!(st.pending_retries.get(&100), Some(&child));
    }

    #[test]
    fn a_finished_child_releases_the_pending_retry_of_its_parent() {
        use super::{JobInfo, pump};
        use crate::state::State;
        use kawaro_core::config::Config;
        use kawaro_core::queue::JobEvent;
        use std::sync::{Arc, Mutex, mpsc::channel};

        let state = Arc::new(Mutex::new(State::new(Config::default(), &[])));
        {
            let mut st = state.lock().unwrap();
            st.jobs.insert(
                2,
                JobInfo {
                    label: "test".into(),
                    tasks: 0,
                    fraction: 0.0,
                    ok: 0,
                    failed: 0,
                    first_output: None,
                    first_error: None,
                    detail: None,
                    skipped: 0,
                    first_skipped: None,
                },
            );
            st.retry_parents.insert(2, 1);
            st.pending_retries.insert(1, 2);
        }
        let (tx, rx) = channel();
        tx.send(JobEvent::Started {
            job: 2,
            action_id: "test".into(),
            tasks: 1,
            manifest: manifest(builtin_action(), vec![vec![PathBuf::from("a.png")]]),
        })
        .unwrap();
        tx.send(JobEvent::Finished {
            job: 2,
            ok: 0,
            failed: 0,
            skipped: 0,
            cancelled: false,
            elapsed: Duration::from_secs(1),
        })
        .unwrap();
        drop(tx);
        pump(rx, state.clone(), None);
        let st = state.lock().unwrap();
        assert!(st.pending_retries.is_empty(), "{:?}", st.pending_retries);
        assert!(st.retry_parents.is_empty(), "{:?}", st.retry_parents);
    }

    // S-144-C: a queued job is not a finished job, and a job with no record
    // is unknown, never a fabricated finished one. Source only.
    #[test]
    fn the_summaries_tell_a_queued_job_from_an_unknown_one() {
        use super::{detail, summaries};
        use crate::state::{JobInfo, State};
        use kawaro_core::config::Config;
        use kawaro_core::job_report::JobState;

        let mut st = State::new(Config::default(), &[]);
        st.jobs.insert(
            5,
            JobInfo {
                label: String::from("Compress to WebP"),
                tasks: 0,
                fraction: 0.0,
                ok: 0,
                failed: 0,
                first_output: None,
                first_error: None,
                detail: None,
                skipped: 0,
                first_skipped: None,
            },
        );
        let summaries = summaries(&st);
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].job, 5);
        assert_eq!(summaries[0].state, JobState::Queued);
        assert_eq!(summaries[0].action_label, "Compress to WebP");
        assert!(summaries[0].action_id.is_empty(), "no action id yet");
        assert_eq!(summaries[0].counts.total, 0);
        assert!(!summaries[0].summary_only);
        assert!(!summaries[0].retryable);

        let queued = detail(&st, 5);
        assert_eq!(queued.state, JobState::Queued);
        assert!(queued.units.is_empty());
        assert!(!queued.retryable);

        let unknown = detail(&st, 9);
        assert_eq!(unknown.state, JobState::Unknown);
        assert_eq!(unknown.counts.total, 0);
        assert!(unknown.units.is_empty());
        assert!(unknown.action_id.is_empty());
        assert!(!unknown.retryable);

        let all = super::details(&st);
        assert_eq!(all.len(), 1);
        assert_eq!(all[0], queued);

        let (snap_details, pending, _lang) = super::results_snapshot(&st);
        assert_eq!(snap_details, all);
        assert!(pending.is_empty());
    }
}
