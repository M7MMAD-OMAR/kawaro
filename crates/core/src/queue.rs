//! Job queue. A job is one action applied to one selection; it expands into
//! tasks (one per file, or one for combined actions). Tasks run on a small
//! worker pool; every task writes a temp file that is finalised atomically.

use crate::action::ActionDef;
use crate::engine::{Engine, Task};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::Sender;

pub type JobId = u64;

#[derive(Clone, Debug, PartialEq)]
pub enum JobEvent {
    Started {
        job: JobId,
        action_id: String,
        tasks: usize,
    },
    /// Overall job fraction in `0.0..=1.0`, monotonic.
    Progress { job: JobId, fraction: f32 },
    TaskDone {
        job: JobId,
        output: PathBuf,
        elapsed: std::time::Duration,
    },
    TaskFailed {
        job: JobId,
        input: PathBuf,
        error: String,
        elapsed: std::time::Duration,
        cancelled: bool,
    },
    Finished {
        job: JobId,
        ok: usize,
        failed: usize,
        cancelled: bool,
        /// Time spent executing this job, excluding queue wait and event delivery.
        elapsed: std::time::Duration,
    },
}

/// Expands a job into tasks with their final output paths. Output goes next
/// to the first input of each task. Pure: does not touch the filesystem
/// except to check which names already exist.
pub fn plan(action: &ActionDef, files: &[PathBuf]) -> Vec<(Task, PathBuf)> {
    crate::queue_impl::plan(action, files)
}

/// Runs one job to completion on the calling thread, sending events. Used by
/// the CLI and by the queue workers. Returns the final output paths.
pub fn run_job(
    job: JobId,
    action: &ActionDef,
    files: &[PathBuf],
    engine: &dyn Engine,
    cancel: &crate::engine::CancelToken,
    events: &Sender<JobEvent>,
) -> Vec<PathBuf> {
    crate::queue_impl::run_job(job, action, files, engine, cancel, events)
}

/// Background queue used by the daemon.
pub struct Queue {
    pub(crate) inner: crate::queue_impl::Inner,
}

impl Queue {
    /// `workers` jobs run at once (at least 1).
    pub fn new(engines: Vec<Arc<dyn Engine>>, workers: usize, events: Sender<JobEvent>) -> Self {
        crate::queue_impl::new(engines, workers, events)
    }

    pub fn engine_available(&self, id: &str) -> bool {
        crate::queue_impl::engine_available(self, id)
    }

    /// Returns immediately. Unknown engine gives a `Finished` event with every file failed.
    pub fn submit(&self, action: ActionDef, files: Vec<PathBuf>) -> JobId {
        crate::queue_impl::submit(self, action, files)
    }

    pub fn cancel(&self, job: JobId) {
        crate::queue_impl::cancel(self, job)
    }

    pub fn cancel_all(&self) {
        crate::queue_impl::cancel_all(self)
    }

    /// Jobs submitted and not yet finished.
    pub fn active(&self) -> usize {
        crate::queue_impl::active(self)
    }
}
