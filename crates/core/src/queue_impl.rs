//! Owned by the core track; see `queue.rs` for the contract.
//!
//! Accounting: `Finished.ok` and `Finished.failed` count tasks, like
//! `Started.tasks` (a combined action is one task). After a cancel the
//! interrupted task reports `TaskFailed` with "cancelled"; tasks that never
//! started are skipped without an event and are not counted.
use crate::action::{ActionDef, OutputMode};
use crate::engine::{CancelToken, Engine, EngineError, Task};
use crate::output::{finalize, temp_path};
use crate::output_impl::final_path_with;
use crate::queue::{JobEvent, JobId, Queue};
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;

type Jobs = Arc<Mutex<HashMap<JobId, CancelToken>>>;

struct Submitted {
    job: JobId,
    action: ActionDef,
    files: Vec<PathBuf>,
    cancel: CancelToken,
}

pub(crate) struct Inner {
    engines: HashMap<String, Arc<dyn Engine>>,
    jobs: Jobs,
    next: AtomicU64,
    tx: Mutex<Option<Sender<Submitted>>>,
    workers: Vec<JoinHandle<()>>,
}

/// Locks ignoring poison: every critical section here is a plain map or
/// value update that cannot leave the data half written.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl Drop for Inner {
    /// Cancels running and waiting jobs, then waits for the workers, so no
    /// temp file outlives the queue.
    fn drop(&mut self) {
        for token in lock(&self.jobs).values() {
            token.cancel();
        }
        lock(&self.tx).take();
        for w in self.workers.drain(..) {
            let _ = w.join();
        }
    }
}

pub(crate) fn plan(action: &ActionDef, files: &[PathBuf]) -> Vec<(Task, PathBuf)> {
    let groups: Vec<&[PathBuf]> = match action.output.mode {
        OutputMode::PerFile => files.chunks(1).collect(),
        OutputMode::Combined if files.is_empty() => Vec::new(),
        OutputMode::Combined => vec![files],
    };
    let mut planned: Vec<PathBuf> = Vec::new();
    let mut out = Vec::new();
    for inputs in groups {
        let first = &inputs[0];
        let dir = first.parent().unwrap_or(Path::new(""));
        let fin = final_path_with(first, &action.output, dir, &|p| {
            planned.iter().any(|q| q == p)
        });
        planned.push(fin.clone());
        let task = Task {
            action_id: action.id.clone(),
            op: action.op.clone(),
            params: action.params.clone(),
            inputs: inputs.to_vec(),
            output: temp_path(&fin),
        };
        out.push((task, fin));
    }
    out
}

fn panic_message(p: &(dyn std::any::Any + Send)) -> String {
    let msg = p
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| p.downcast_ref::<String>().cloned())
        .unwrap_or_default();
    format!("engine panicked: {msg}")
}

/// Runs one task and finalises its output. On error the temp file is gone.
fn run_task(
    engine: &dyn Engine,
    task: &Task,
    fin: &Path,
    cancel: &CancelToken,
    progress: &(dyn Fn(f32) + Send + Sync),
) -> Result<PathBuf, (String, bool)> {
    let r = match catch_unwind(AssertUnwindSafe(|| engine.run(task, progress, cancel))) {
        Ok(Ok(())) => finalize(&task.output, fin).map_err(|e| (e.to_string(), false)),
        Ok(Err(e)) => Err((e.to_string(), matches!(e, EngineError::Cancelled))),
        Err(p) => Err((panic_message(p.as_ref()), false)),
    };
    if r.is_err() {
        let _ = std::fs::remove_file(&task.output);
    }
    r
}

/// Shared by `run_job` and the queue workers. `engine` is the error text
/// for every task when no usable engine exists. `before_finish` runs just
/// before the `Finished` event.
fn run_inner(
    job: JobId,
    action: &ActionDef,
    files: &[PathBuf],
    engine: Result<&dyn Engine, String>,
    cancel: &CancelToken,
    events: &Sender<JobEvent>,
    before_finish: &dyn Fn(),
) -> Vec<PathBuf> {
    let tasks = plan(action, files);
    let n = tasks.len();
    let _ = events.send(JobEvent::Started {
        job,
        action_id: action.id.clone(),
        tasks: n,
    });
    let last = Mutex::new(0.0f32);
    let report = |fraction: f32| {
        let mut last = lock(&last);
        if fraction > *last {
            *last = fraction;
            let _ = events.send(JobEvent::Progress { job, fraction });
        }
    };
    let (mut outputs, mut failed, mut cancelled) = (Vec::new(), 0, false);
    for (i, (task, fin)) in tasks.iter().enumerate() {
        if cancel.is_cancelled() {
            cancelled = true;
            break;
        }
        let progress = |f: f32| report((i as f32 + f.clamp(0.0, 1.0)) / n as f32);
        let result = match &engine {
            Ok(e) => run_task(*e, task, fin, cancel, &progress),
            Err(msg) => Err((msg.clone(), false)),
        };
        match result {
            Ok(path) => {
                outputs.push(path.clone());
                let _ = events.send(JobEvent::TaskDone { job, output: path });
            }
            Err((error, was_cancel)) => {
                failed += 1;
                cancelled |= was_cancel;
                let _ = events.send(JobEvent::TaskFailed {
                    job,
                    input: task.inputs[0].clone(),
                    error,
                });
            }
        }
        report((i + 1) as f32 / n as f32);
    }
    before_finish();
    let _ = events.send(JobEvent::Finished {
        job,
        ok: outputs.len(),
        failed,
        cancelled,
    });
    outputs
}

pub(crate) fn run_job(
    job: JobId,
    action: &ActionDef,
    files: &[PathBuf],
    engine: &dyn Engine,
    cancel: &CancelToken,
    events: &Sender<JobEvent>,
) -> Vec<PathBuf> {
    let engine = if engine.available() {
        Ok(engine)
    } else {
        Err(EngineError::Unavailable(engine.id().to_string()).to_string())
    };
    run_inner(job, action, files, engine, cancel, events, &|| {})
}

fn worker(
    rx: Arc<Mutex<Receiver<Submitted>>>,
    engines: HashMap<String, Arc<dyn Engine>>,
    jobs: Jobs,
    events: Sender<JobEvent>,
) {
    loop {
        let next = lock(&rx).recv();
        let Ok(s) = next else { break };
        let engine = match engines.get(&s.action.engine) {
            Some(e) if e.available() => Ok(e.as_ref()),
            _ => Err(EngineError::Unavailable(s.action.engine.clone()).to_string()),
        };
        run_inner(
            s.job,
            &s.action,
            &s.files,
            engine,
            &s.cancel,
            &events,
            &|| {
                lock(&jobs).remove(&s.job);
            },
        );
    }
}

pub(crate) fn new(
    engines: Vec<Arc<dyn Engine>>,
    workers: usize,
    events: Sender<JobEvent>,
) -> Queue {
    let engines: HashMap<String, Arc<dyn Engine>> = engines
        .into_iter()
        .map(|e| (e.id().to_string(), e))
        .collect();
    let jobs: Jobs = Arc::default();
    let (tx, rx) = channel();
    let rx = Arc::new(Mutex::new(rx));
    let workers = (0..workers.max(1))
        .map(|_| {
            let (rx, engines, jobs, events) =
                (rx.clone(), engines.clone(), jobs.clone(), events.clone());
            std::thread::spawn(move || worker(rx, engines, jobs, events))
        })
        .collect();
    Queue {
        inner: Inner {
            engines,
            jobs,
            next: AtomicU64::new(1),
            tx: Mutex::new(Some(tx)),
            workers,
        },
    }
}

pub(crate) fn engine_available(q: &Queue, id: &str) -> bool {
    q.inner.engines.get(id).is_some_and(|e| e.available())
}

pub(crate) fn submit(q: &Queue, action: ActionDef, files: Vec<PathBuf>) -> JobId {
    let job = q.inner.next.fetch_add(1, Ordering::SeqCst);
    let cancel = CancelToken::new();
    lock(&q.inner.jobs).insert(job, cancel.clone());
    // The sender is only taken in Drop, and workers never exit while it
    // lives (engine panics are caught), so this send cannot fail.
    if let Some(tx) = lock(&q.inner.tx).as_ref() {
        let _ = tx.send(Submitted {
            job,
            action,
            files,
            cancel,
        });
    }
    job
}

pub(crate) fn cancel(q: &Queue, job: JobId) {
    if let Some(token) = lock(&q.inner.jobs).get(&job) {
        token.cancel();
    }
}

pub(crate) fn cancel_all(q: &Queue) {
    for token in lock(&q.inner.jobs).values() {
        token.cancel();
    }
}

pub(crate) fn active(q: &Queue) -> usize {
    lock(&q.inner.jobs).len()
}
