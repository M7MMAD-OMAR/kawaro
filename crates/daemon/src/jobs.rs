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
                JobEvent::Finished { cancelled, .. } => finished = Some(*cancelled),
            }
        }
        st.publish(job, Event::from(ev));
        let lang = st.lang;
        if let Some(cancelled) = finished
            && let Some(info) = st.jobs.remove(&job)
        {
            if st.config.notify {
                notify::finished(lang, &info, cancelled);
            }
            if st.config.open_folder_when_done
                && !cancelled
                && let Some(dir) = info
                    .first_output
                    .as_deref()
                    .and_then(std::path::Path::parent)
            {
                let file = info.first_output.as_deref();
                let (program, args) = open_folder_command(CURRENT, dir, file);
                let _ = std::process::Command::new(program).args(args).spawn();
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
