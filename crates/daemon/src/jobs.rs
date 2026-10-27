//! Job bookkeeping: submission and the event pump that feeds clients, the
//! tray and the end-of-job notification.

use crate::notify_logic::open_folder_command;
use crate::state::{JobInfo, Shared, State, lock};
use crate::{notify, tray};
use kawaro_core::ActionDef;
use kawaro_core::ipc::Event;
use kawaro_core::paths::CURRENT;
use kawaro_core::queue::{JobEvent, JobId, Queue};
use std::path::PathBuf;
use std::sync::mpsc::Receiver;

pub fn submit(st: &mut State, queue: &Queue, action: ActionDef, files: Vec<PathBuf>) -> JobId {
    let label = action.label.get(st.lang).to_string();
    let action = st.configured(action);
    let job = queue.submit(action, files);
    st.jobs.entry(job).or_insert(JobInfo {
        label,
        tasks: 0,
        fraction: 0.0,
        ok: 0,
        failed: 0,
        first_output: None,
        first_error: None,
    });
    job
}

pub fn pump(events: Receiver<JobEvent>, state: Shared, tray: tray::TrayHandle) {
    for ev in events {
        let mut st = lock(&state);
        let job = match &ev {
            JobEvent::Started { job, .. }
            | JobEvent::Progress { job, .. }
            | JobEvent::TaskDone { job, .. }
            | JobEvent::TaskFailed { job, .. }
            | JobEvent::Finished { job, .. } => *job,
        };
        let mut finished = None;
        if let Some(info) = st.jobs.get_mut(&job) {
            match &ev {
                JobEvent::Started { tasks, .. } => info.tasks = *tasks,
                JobEvent::Progress { fraction, .. } => info.fraction = *fraction,
                JobEvent::TaskDone { output, .. } => {
                    info.ok += 1;
                    info.first_output.get_or_insert_with(|| output.clone());
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
        let task_notice = st.jobs.get(&job).and_then(|info| {
            task_notice(
                &ev,
                &info.label,
                st.config.notify,
                st.config.notify_long_only,
            )
        });
        if let Some(notice) = task_notice {
            notify::finished(st.lang, &notice, false);
        }
        st.publish(job, Event::from(ev));
        if let Some((cancelled, _elapsed)) = finished
            && let Some(info) = st.jobs.remove(&job)
        {
            let Some(dir) = info
                .first_output
                .as_deref()
                .and_then(std::path::Path::parent)
            else {
                continue;
            };
            if st.config.open_folder_when_done && !cancelled {
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
    }
}

/// A missing or cancelled task has no completion notice. Combined actions
/// represent one task, so they produce at most one notice for the batch.
fn task_notice(event: &JobEvent, label: &str, enabled: bool, long_only: bool) -> Option<JobInfo> {
    let (elapsed, output, error) = match event {
        JobEvent::TaskDone {
            output, elapsed, ..
        } => (*elapsed, Some(output.clone()), None),
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
            (*elapsed, None, Some(format!("{name}: {error}")))
        }
        _ => return None,
    };
    if !should_notify(enabled, long_only, Some(elapsed)) {
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
    })
}

fn should_notify(enabled: bool, long_only: bool, elapsed: Option<std::time::Duration>) -> bool {
    enabled && (!long_only || elapsed.is_some_and(|d| d >= std::time::Duration::from_secs(10)))
}

#[cfg(test)]
mod tests {
    use super::should_notify;
    use std::time::Duration;

    #[test]
    fn notification_policy_respects_duration_and_switch() {
        assert!(!should_notify(false, false, Some(Duration::from_secs(30))));
        assert!(should_notify(true, false, Some(Duration::from_secs(1))));
        assert!(!should_notify(true, true, Some(Duration::from_secs(9))));
        assert!(should_notify(true, true, Some(Duration::from_secs(10))));
        assert!(!should_notify(true, true, None));
    }
    #[test]
    fn batch_notices_follow_each_task_duration_and_policy() {
        use super::{JobInfo, pump};
        use crate::{notify::captured_log, state::State};
        use kawaro_core::{config::Config, queue::JobEvent};
        use std::{
            path::PathBuf,
            sync::{Arc, Mutex, mpsc::channel},
        };

        for (enabled, long_only, expected) in [(false, false, 0), (true, false, 3), (true, true, 2)]
        {
            let state = Arc::new(Mutex::new(State::new(
                Config {
                    notify: enabled,
                    notify_long_only: long_only,
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
                tasks: 4,
            })
            .unwrap();
            for (name, seconds) in [("fast", 1), ("slow", 10)] {
                tx.send(JobEvent::TaskDone {
                    job: 1,
                    output: PathBuf::from(name),
                    elapsed: Duration::from_secs(seconds),
                })
                .unwrap();
            }
            tx.send(JobEvent::TaskFailed {
                job: 1,
                input: "broken".into(),
                error: "error".into(),
                elapsed: Duration::from_secs(12),
                cancelled: false,
            })
            .unwrap();
            tx.send(JobEvent::TaskFailed {
                job: 1,
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
                "enabled={enabled} long={long_only}"
            );
            assert!(state.lock().unwrap().jobs.is_empty());
        }
    }
}
