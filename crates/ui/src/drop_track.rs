//! Dragging files from Explorer onto a hive cell on Windows (S-047, ADR 0005
//! section 5). winit gives no drop position, so `dnd/windows.rs` polls the
//! cursor every [`TICK_MS`] during a drag and feeds it here. Pure logic,
//! tested on Linux; it never touches the dropped files.

use crate::layout::Plan;
use std::path::PathBuf;

/// The cursor poll period during a drag, in milliseconds (the period of the
/// `slint::Timer` in `dnd/windows.rs`).
pub const TICK_MS: u32 = 16;

/// What a drop does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DropOutcome {
    /// Run the action of cell `cell` (an index into `plan.cells`) on `files`.
    Run { cell: i32, files: Vec<PathBuf> },
    /// The drop missed every cell, or no files were dragged.
    Ignored,
}

/// The files of the current drag and the cell last returned by
/// [`DropTracker::on_tick`].
#[derive(Debug, Default)]
pub struct DropTracker {
    files: Vec<PathBuf>,
    last: Option<i32>,
}

impl DropTracker {
    /// The drag carries `files`.
    pub fn on_hover(&mut self, files: Vec<PathBuf>) {
        self.files = files;
    }

    /// winit sends one hover event per file: the drag carries `path` too.
    pub fn on_hover_file(&mut self, path: PathBuf) {
        self.files.push(path);
    }

    /// The files of the current drag so far.
    pub fn files(&self) -> &[PathBuf] {
        &self.files
    }

    /// True from the first hover until a drop, a cancel or [`Self::on_close`].
    pub fn is_active(&self) -> bool {
        !self.files().is_empty()
    }

    /// The hive closed: the files and the last cell are forgotten, so the
    /// next drag starts empty.
    pub fn on_close(&mut self) {
        self.files.clear();
        self.last = None;
    }

    /// The cell under `cursor` (surface local, logical), or -1 for none, when
    /// it differs from the last one returned; otherwise `None`.
    pub fn on_tick(&mut self, plan: &Plan, cursor: (f32, f32)) -> Option<i32> {
        let cell = plan.pick(cursor.0, cursor.1);
        if self.last == Some(cell) {
            return None;
        }
        self.last = Some(cell);
        Some(cell)
    }

    /// The drop at `cursor`. Either way the stored files are cleared.
    pub fn on_drop(&mut self, plan: &Plan, cursor: (f32, f32)) -> DropOutcome {
        let files = std::mem::take(&mut self.files);
        let cell = plan.pick(cursor.0, cursor.1);
        if cell == -1 || files.is_empty() {
            return DropOutcome::Ignored;
        }
        DropOutcome::Run { cell, files }
    }

    /// The drag left or was cancelled: the files are forgotten.
    pub fn on_cancel(&mut self) {
        self.files.clear();
    }
}

/// What asked for the drop hive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// The global shortcut.
    Shortcut,
    /// The tray or context menu.
    Menu,
}

/// The S-021 rule on Windows: a shortcut shows the drop hive only during a
/// drag, that is with the left button held; a menu always shows it.
pub fn drop_hive_allowed(source: Source, left_button_down: bool) -> bool {
    match source {
        Source::Shortcut => left_button_down,
        Source::Menu => true,
    }
}
