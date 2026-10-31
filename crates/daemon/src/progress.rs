//! The progress of the update download (S-054b rework 1), kept apart from the
//! update lock: a download holds that lock from start to end, and the tray
//! must still show "Downloading update 42%". The holder knows the version,
//! the size from the signed manifest and the `.part` file; the bytes done
//! are the length of that file, read by whoever asks.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::state::lock;
use crate::update::Phase;

/// A download percentage, rounded to the nearest, 0 for an unknown size and
/// never above 100.
pub fn percent(done: u64, size: u64) -> u64 {
    if size == 0 {
        return 0;
    }
    let done = u128::from(done.min(size));
    let size = u128::from(size);
    ((done * 100 + size / 2) / size) as u64
}

struct Active {
    version: String,
    size: u64,
    part: PathBuf,
}

#[derive(Default)]
pub struct Progress {
    active: Mutex<Option<Active>>,
}

impl Progress {
    pub const fn new() -> Progress {
        Progress {
            active: Mutex::new(None),
        }
    }

    /// A download of `version` (`size` bytes) writes to `part`.
    pub fn begin(&self, version: &str, size: u64, part: &Path) {
        *lock(&self.active) = Some(Active {
            version: version.to_string(),
            size,
            part: part.to_path_buf(),
        });
    }

    pub fn end(&self) {
        *lock(&self.active) = None;
    }

    /// `None` when nothing downloads; otherwise the phase with the bytes
    /// `stat` finds in the part file (0 while it does not exist yet).
    pub fn snapshot(&self, stat: &dyn Fn(&Path) -> Option<u64>) -> Option<Phase> {
        lock(&self.active).as_ref().map(|a| Phase::Downloading {
            version: a.version.clone(),
            downloaded: stat(&a.part).unwrap_or(0),
            size: a.size,
        })
    }
}

static SHARED: Progress = Progress::new();

/// The daemon's one holder.
pub fn shared() -> &'static Progress {
    &SHARED
}

/// Downloads take turns in tests (see `download_step`), and a test that
/// reads the global progress holds it so no other download publishes into
/// it meanwhile.
#[cfg(test)]
pub static TEST_GATE: Mutex<()> = Mutex::new(());
