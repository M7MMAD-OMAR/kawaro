//! Batching of `Open` requests (S-046 AC8): Explorer starts one process per
//! selected file, so the first path shows the hive at once and the paths
//! that follow within the merge window rebuild its cells for all files.

use kawaro_core::merge::{Collector, Push};
use std::path::PathBuf;

/// What the hive does for a pushed path.
#[derive(Debug, PartialEq)]
pub enum Step {
    /// A new batch: show the hive now.
    Show {
        files: Vec<PathBuf>,
        cells: Vec<String>,
    },
    /// The batch grew: rebuild the cells for all its files.
    Rebuild {
        files: Vec<PathBuf>,
        cells: Vec<String>,
    },
}

type CellsFor = Box<dyn Fn(&[PathBuf]) -> Vec<String> + Send>;

pub struct OpenBatch {
    collector: Collector,
    cells_for: CellsFor,
}

impl OpenBatch {
    /// `cells_for` gives the cell ids for a set of files.
    pub fn new(cells_for: impl Fn(&[PathBuf]) -> Vec<String> + Send + 'static) -> Self {
        Self {
            collector: Collector::new(),
            cells_for: Box::new(cells_for),
        }
    }

    pub fn push(&mut self, at_ms: u64, path: PathBuf) -> Step {
        let kind = self.collector.push(at_ms, path);
        let files = self.collector.files();
        let cells = (self.cells_for)(&files);
        match kind {
            Push::First => Step::Show { files, cells },
            Push::Merged => Step::Rebuild { files, cells },
        }
    }
}
