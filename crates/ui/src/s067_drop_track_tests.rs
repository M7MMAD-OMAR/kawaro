//! S-067 AC4 (backlog/S-067-review-fixes-update-drop-notify.md): the
//! tracker owns the files of one drag, so a drop never runs on files from an
//! earlier drag. Pure, tested on Linux.
//!
//! Contract (tester), new methods of `crate::drop_track::DropTracker`:
//! - `on_hover_file(&mut self, path: PathBuf)` appends one path (winit sends
//!   one `HoveredFile` per file).
//! - `is_active(&self) -> bool`: true from the first hover until a drop
//!   (`on_drop`), a cancel (`on_cancel`) or `on_close`; false for a new
//!   tracker.
//! - `on_close(&mut self)`: the hive closed; clears the files and the last
//!   cell returned by `on_tick`.
//! The S-047 methods and their tests are unchanged.

use crate::HiveCell;
use crate::drop_track::{DropOutcome, DropTracker};
use crate::layout::{Plan, plan};
use std::path::PathBuf;

fn cell(id: &str, ring: u8) -> HiveCell {
    HiveCell {
        id: id.into(),
        code: id.to_uppercase(),
        label: format!("label {id}"),
        icon: String::new(),
        ring,
    }
}

/// The two-ring hive of the S-047 tests: 18 cells.
fn full_hive() -> Plan {
    let mut cells: Vec<HiveCell> = (0..6).map(|i| cell(&format!("a{i}"), 1)).collect();
    cells.extend((0..12).map(|i| cell(&format!("b{i}"), 2)));
    plan(cells, false)
}

fn over(p: &Plan, i: usize) -> (f32, f32) {
    (p.cells[i].x, p.cells[i].y)
}

fn a() -> PathBuf {
    PathBuf::from("/tmp/s067/a.png")
}

fn b() -> PathBuf {
    PathBuf::from("/tmp/s067/b.jpg")
}

#[test]
fn ac4_a_closed_hive_forgets_the_files_of_the_earlier_drag() {
    let p = full_hive();
    assert_eq!(p.pick(over(&p, 3).0, over(&p, 3).1), 3, "fixture");
    let mut t = DropTracker::default();
    t.on_hover_file(a());
    t.on_close();
    t.on_hover_file(b());
    assert_eq!(
        t.on_drop(&p, over(&p, 3)),
        DropOutcome::Run {
            cell: 3,
            files: vec![b()]
        }
    );
}

#[test]
fn ac4_on_hover_file_appends_one_path_per_event() {
    let p = full_hive();
    let mut t = DropTracker::default();
    t.on_hover_file(a());
    t.on_hover_file(b());
    assert_eq!(
        t.on_drop(&p, over(&p, 5)),
        DropOutcome::Run {
            cell: 5,
            files: vec![a(), b()]
        }
    );
    // The next drag starts empty.
    t.on_hover_file(b());
    assert_eq!(
        t.on_drop(&p, over(&p, 5)),
        DropOutcome::Run {
            cell: 5,
            files: vec![b()]
        }
    );
}

#[test]
fn ac4_is_active_from_the_first_hover_until_drop_cancel_or_close() {
    let p = full_hive();
    let mut t = DropTracker::default();
    assert!(!t.is_active(), "a new tracker is idle");

    t.on_hover_file(a());
    assert!(t.is_active(), "after the first hover");
    t.on_hover_file(b());
    assert!(t.is_active(), "after a second hover");
    let _ = t.on_tick(&p, over(&p, 2));
    assert!(t.is_active(), "a tick does not end the drag");
    let _ = t.on_drop(&p, over(&p, 2));
    assert!(!t.is_active(), "after a drop");

    t.on_hover_file(a());
    assert!(t.is_active());
    t.on_cancel();
    assert!(!t.is_active(), "after a cancel");

    t.on_hover_file(a());
    assert!(t.is_active());
    t.on_close();
    assert!(!t.is_active(), "after the hive closed");

    // A drop that misses every cell also ends the drag.
    t.on_hover_file(a());
    assert_eq!(t.on_drop(&p, p.layout.origin), DropOutcome::Ignored);
    assert!(!t.is_active(), "after a drop off every cell");
}

#[test]
fn ac4_on_close_clears_the_last_cell_too() {
    let p = full_hive();
    let mut t = DropTracker::default();
    t.on_hover_file(a());
    assert_eq!(t.on_tick(&p, over(&p, 3)), Some(3));
    assert_eq!(t.on_tick(&p, over(&p, 3)), None, "unchanged");
    t.on_close();
    t.on_hover_file(b());
    assert_eq!(
        t.on_tick(&p, over(&p, 3)),
        Some(3),
        "a new drag reports its first cell again"
    );
}
