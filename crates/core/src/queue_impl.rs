//! Owned by the core track; see `queue.rs` for the contract.
use crate::action::ActionDef;
use crate::engine::{CancelToken, Engine, Task};
use crate::queue::{JobEvent, JobId, Queue};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::Sender;

pub(crate) struct Inner;

pub(crate) fn plan(_action: &ActionDef, _files: &[PathBuf]) -> Vec<(Task, PathBuf)> {
    todo!("core track")
}
pub(crate) fn run_job(
    _job: JobId,
    _action: &ActionDef,
    _files: &[PathBuf],
    _engine: &dyn Engine,
    _cancel: &CancelToken,
    _events: &Sender<JobEvent>,
) -> Vec<PathBuf> {
    todo!("core track")
}
pub(crate) fn new(
    _engines: Vec<Arc<dyn Engine>>,
    _workers: usize,
    _events: Sender<JobEvent>,
) -> Queue {
    todo!("core track")
}
pub(crate) fn engine_available(_q: &Queue, _id: &str) -> bool {
    todo!("core track")
}
pub(crate) fn submit(_q: &Queue, _action: ActionDef, _files: Vec<PathBuf>) -> JobId {
    todo!("core track")
}
pub(crate) fn cancel(_q: &Queue, _job: JobId) {
    todo!("core track")
}
pub(crate) fn cancel_all(_q: &Queue) {
    todo!("core track")
}
pub(crate) fn active(_q: &Queue) -> usize {
    todo!("core track")
}
