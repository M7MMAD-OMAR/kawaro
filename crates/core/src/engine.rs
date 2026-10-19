//! The engine contract. An engine turns one [`Task`] into exactly one output
//! file at `task.output`, a temporary path the queue later renames
//! atomically. Engines never choose output names and never touch inputs.

use crate::media::MediaKind;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// One unit of work handed to an engine.
#[derive(Clone, Debug)]
pub struct Task {
    /// Action id from the registry, for logs and errors.
    pub action_id: String,
    /// Engine-specific operation name, from the action TOML `op` field.
    pub op: String,
    /// Engine-specific parameters, from the action TOML `[action.params]` table.
    pub params: toml::Table,
    /// One input for per-file actions, all inputs in selection order for combined ones.
    pub inputs: Vec<PathBuf>,
    /// Temporary path to write. Same directory as the final output, so the
    /// final rename is atomic. Its extension is the final extension.
    pub output: PathBuf,
}

impl Task {
    pub fn param_str(&self, key: &str) -> Option<&str> {
        self.params.get(key).and_then(|v| v.as_str())
    }
    pub fn param_i64(&self, key: &str) -> Option<i64> {
        self.params.get(key).and_then(|v| v.as_integer())
    }
    pub fn param_f64(&self, key: &str) -> Option<f64> {
        self.params
            .get(key)
            .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
    }
    pub fn param_bool(&self, key: &str) -> Option<bool> {
        self.params.get(key).and_then(|v| v.as_bool())
    }
}

/// Progress callback, fraction in `0.0..=1.0`. Cheap to call often.
pub type Progress<'a> = &'a (dyn Fn(f32) + Send + Sync);

/// Cooperative cancellation shared between the queue and an engine.
#[derive(Clone, Debug, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
    /// Convenience for engines: `token.check()?` between steps.
    pub fn check(&self) -> Result<(), EngineError> {
        if self.is_cancelled() {
            Err(EngineError::Cancelled)
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("cancelled")]
    Cancelled,
    #[error("unsupported operation `{0}`")]
    UnsupportedOp(String),
    #[error("bad parameter `{name}`: {reason}")]
    BadParam { name: String, reason: String },
    #[error("cannot read {path}: {reason}")]
    Input { path: PathBuf, reason: String },
    #[error("engine tool not available: {0}")]
    Unavailable(String),
    #[error("{0}")]
    Failed(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Rough cost of a task, for ordering and for the progress indicator.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Estimate {
    pub seconds: f32,
    pub memory_mb: u32,
}

pub trait Engine: Send + Sync {
    /// Matches the `engine` field of action TOML files.
    fn id(&self) -> &'static str;

    /// False when an external tool is missing. Actions of unavailable engines
    /// are hidden from the hive.
    fn available(&self) -> bool {
        true
    }

    /// Operations this engine understands, as (op, accepted input kinds).
    fn capabilities(&self) -> Vec<(&'static str, Vec<MediaKind>)>;

    fn estimate(&self, task: &Task) -> Estimate {
        let bytes: u64 = task
            .inputs
            .iter()
            .filter_map(|p| std::fs::metadata(p).ok())
            .map(|m| m.len())
            .sum();
        Estimate {
            seconds: bytes as f32 / 50_000_000.0,
            memory_mb: (bytes / 1_000_000) as u32 * 4 + 16,
        }
    }

    /// Writes `task.output`. Must call `progress` at least once with 1.0 on
    /// success and must return [`EngineError::Cancelled`] soon after `cancel`
    /// fires. A partial `task.output` left behind is removed by the queue.
    fn run(&self, task: &Task, progress: Progress, cancel: &CancelToken)
    -> Result<(), EngineError>;
}
