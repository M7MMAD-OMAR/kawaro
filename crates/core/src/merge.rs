//! Merging of the paths Explorer sends one process at a time (S-046 AC7).
//! Pure: the caller passes the clock.

use std::path::PathBuf;

/// Pushes closer together than this many milliseconds belong to one batch.
pub const WINDOW_MS: u64 = 300;

/// What a pushed path did to the batch.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Push {
    /// It started a new batch.
    First,
    /// It joined the batch in progress.
    Merged,
}

/// The paths of the batch in progress, in first seen order.
#[derive(Debug, Default)]
pub struct Collector {
    last_ms: Option<u64>,
    files: Vec<PathBuf>,
}

impl Collector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `path` at time `at_ms`. The gap is measured from the last push,
    /// so a steady stream of paths stays one batch.
    pub fn push(&mut self, at_ms: u64, path: PathBuf) -> Push {
        let merged = self
            .last_ms
            .is_some_and(|last| at_ms.saturating_sub(last) < WINDOW_MS);
        if !merged {
            self.files.clear();
        }
        if !self.files.contains(&path) {
            self.files.push(path);
        }
        self.last_ms = Some(at_ms);
        if merged { Push::Merged } else { Push::First }
    }

    /// The paths of the batch, without duplicates.
    pub fn files(&self) -> Vec<PathBuf> {
        self.files.clone()
    }
}
