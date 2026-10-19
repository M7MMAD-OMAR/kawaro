//! Image engine. Owned by the image track.
use sbarhive_core::{CancelToken, Engine, EngineError, MediaKind, Progress, Task};

#[derive(Default)]
pub struct ImageEngine;

impl Engine for ImageEngine {
    fn id(&self) -> &'static str {
        "image"
    }
    fn capabilities(&self) -> Vec<(&'static str, Vec<MediaKind>)> {
        vec![]
    }
    fn run(
        &self,
        task: &Task,
        _progress: Progress,
        _cancel: &CancelToken,
    ) -> Result<(), EngineError> {
        Err(EngineError::UnsupportedOp(task.op.clone()))
    }
}
