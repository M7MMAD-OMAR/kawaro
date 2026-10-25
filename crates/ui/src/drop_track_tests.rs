//! S-047: dragging from Explorer onto a hive cell on Windows
//! (backlog/S-047-windows-drop.md). The pure tracker is tested here on Linux.
//!
//! Contract these tests fix, in `crate::drop_track`:
//! - `pub struct DropTracker` with `Default` (no stored files, nothing
//!   returned yet).
//! - `on_hover(&mut self, files: Vec<PathBuf>)` stores the files.
//! - `on_tick(&mut self, plan: &Plan, cursor: (f32, f32)) -> Option<i32>`.
//! - `on_drop(&mut self, plan: &Plan, cursor: (f32, f32)) -> DropOutcome`.
//! - `on_cancel(&mut self)`.
//! - `pub enum DropOutcome { Run { cell, files: Vec<PathBuf> }, Ignored }`
//!   with `Debug` and `PartialEq`; `cell` is an integer index into
//!   `plan.cells`.
//! - `pub const TICK_MS: u32 = 16` (the `uElapse` type of `SetTimer`).
//! - `pub enum Source { Shortcut, Menu }` and
//!   `pub fn drop_hive_allowed(source: Source, left_button_down: bool) -> bool`.
//!
//! On "Layout": the story writes `&Layout` and `Layout::pick` returning an
//! `i32`. The only `pick` that needs no extra argument and gives an `i32` is
//! `crate::layout::Plan::pick` (`geometry::Layout::pick` also needs the cell
//! slots and gives `Option<usize>`). So the layout argument is a `&Plan`, and
//! the value pick gives for no cell is `-1` (read in `Plan::pick`: "Index
//! into `cells` of the action under the point, or -1 for the centre, a gap,
//! an empty slot or the transparent corners").

use crate::HiveCell;
use crate::drop_track::{DropOutcome, DropTracker, Source, TICK_MS, drop_hive_allowed};
use crate::layout::{Plan, plan};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

fn cell(id: &str, ring: u8) -> HiveCell {
    HiveCell {
        id: id.into(),
        code: id.to_uppercase(),
        label: format!("label {id}"),
        icon: String::new(),
        ring,
    }
}

/// The full two-ring hive of the S-014 drop tests: 18 cells, left to right.
fn full_hive() -> Plan {
    let mut cells: Vec<HiveCell> = (0..6).map(|i| cell(&format!("a{i}"), 1)).collect();
    cells.extend((0..12).map(|i| cell(&format!("b{i}"), 2)));
    plan(cells, false)
}

/// Centre of cell `i`, surface local.
fn over(p: &Plan, i: usize) -> (f32, f32) {
    (p.cells[i].x, p.cells[i].y)
}

/// The hive centre: the centre cell, which is no action.
fn empty(p: &Plan) -> (f32, f32) {
    p.layout.origin
}

fn files_ab() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/tmp/s047/a.png"),
        PathBuf::from("/tmp/s047/b.jpg"),
    ]
}

/// Guards the fixture itself: cell 3 is picked at its centre, the hive
/// centre is no cell.
fn check_fixture(p: &Plan) {
    assert_eq!(p.cells.len(), 18);
    let (x, y) = over(p, 3);
    assert_eq!(p.pick(x, y), 3);
    let (x, y) = empty(p);
    assert_eq!(p.pick(x, y), -1);
}

#[test]
fn ac1_tick_returns_cell_only_on_change_and_minus_one_for_no_cell() {
    let p = full_hive();
    check_fixture(&p);
    let mut t = DropTracker::default();
    t.on_hover(files_ab());
    assert_eq!(t.on_tick(&p, over(&p, 3)), Some(3));
    assert_eq!(t.on_tick(&p, over(&p, 3)), None);
    assert_eq!(t.on_tick(&p, empty(&p)), Some(-1));
    // Still no cell: no change, nothing returned.
    assert_eq!(t.on_tick(&p, empty(&p)), None);
    // Another cell, then back to cell 3: each change is returned.
    assert_eq!(t.on_tick(&p, over(&p, 7)), Some(7));
    assert_eq!(t.on_tick(&p, over(&p, 3)), Some(3));
}

#[test]
fn ac2_drop_over_cell_runs_with_files_in_order_then_clears() {
    let p = full_hive();
    check_fixture(&p);
    let mut t = DropTracker::default();
    t.on_hover(files_ab());
    assert_eq!(
        t.on_drop(&p, over(&p, 3)),
        DropOutcome::Run {
            cell: 3,
            files: files_ab(),
        }
    );
    // The files were cleared by the outcome.
    assert_eq!(t.on_drop(&p, over(&p, 3)), DropOutcome::Ignored);
}

#[test]
fn ac2_drop_over_empty_space_is_ignored_then_clears() {
    let p = full_hive();
    check_fixture(&p);
    let mut t = DropTracker::default();
    t.on_hover(files_ab());
    assert_eq!(t.on_drop(&p, empty(&p)), DropOutcome::Ignored);
    // The files were cleared by the ignored outcome too.
    assert_eq!(t.on_drop(&p, over(&p, 3)), DropOutcome::Ignored);
}

#[test]
fn ac2_drop_without_stored_files_is_ignored() {
    let p = full_hive();
    check_fixture(&p);
    let mut t = DropTracker::default();
    assert_eq!(t.on_drop(&p, over(&p, 3)), DropOutcome::Ignored);
    // An empty hover stores no files either.
    t.on_hover(Vec::new());
    assert_eq!(t.on_drop(&p, over(&p, 3)), DropOutcome::Ignored);
}

#[test]
fn ac3_cancel_clears_files_and_next_drop_is_ignored() {
    let p = full_hive();
    check_fixture(&p);
    let mut t = DropTracker::default();
    t.on_hover(files_ab());
    t.on_cancel();
    assert_eq!(t.on_drop(&p, over(&p, 3)), DropOutcome::Ignored);
}

#[test]
fn ac4_tick_ms_is_16() {
    assert_eq!(TICK_MS, 16u32);
}

/// Source without comments, so a name in a comment never counts as code.
fn strip_comments(src: &str) -> String {
    src.lines()
        .map(|l| match l.find("//") {
            Some(i) => &l[..i],
            None => l,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

const EVENTS: [&str; 3] = ["HoveredFileCancelled", "DroppedFile", "HoveredFile"];

/// Every match arm body of `event` in `src`: from the `=>` that follows the
/// pattern to the next drag event name, `WindowEvent::` or `_ =>` (or the
/// end). A pattern is an
/// occurrence of the name followed by `=>` with no `;`, `{` or `=` alone in
/// between, so `if let ... = event {` is no arm.
fn arm_bodies<'a>(src: &'a str, event: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(off) = src[from..].find(event) {
        let at = from + off;
        from = at + event.len();
        // `HoveredFile` must not match inside `HoveredFileCancelled`.
        if src[from..].starts_with("Cancelled") && event == "HoveredFile" {
            continue;
        }
        let rest = &src[from..];
        let Some(arrow) = rest.find("=>") else {
            continue;
        };
        let head = &rest[..arrow];
        if head.contains(';') || head.contains('{') || head.contains('=') {
            continue;
        }
        let body_start = from + arrow + 2;
        let body = &src[body_start..];
        let end = EVENTS
            .iter()
            .chain(&["WindowEvent::", "_ =>"])
            .filter_map(|e| body.find(e))
            .min()
            .unwrap_or(body.len());
        out.push(&body[..end]);
    }
    out
}

fn kills_timer(body: &str) -> bool {
    body.contains("KillTimer") || body.contains(".stop()")
}

#[test]
fn ac4_windows_file_kills_tick_timer_on_both_end_events() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/dnd/windows.rs");
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("AC4: cannot read {}: {e}", path.display()));
    let src = strip_comments(&raw);
    assert!(
        src.contains("TICK_MS"),
        "AC4: dnd/windows.rs must start its timer with TICK_MS"
    );
    assert!(
        !arm_bodies(&src, "HoveredFile").is_empty(),
        "AC4: dnd/windows.rs has no HoveredFile match arm"
    );
    for event in ["DroppedFile", "HoveredFileCancelled"] {
        let arms = arm_bodies(&src, event);
        assert!(
            !arms.is_empty(),
            "AC4: dnd/windows.rs has no {event} match arm"
        );
        for body in arms {
            assert!(
                kills_timer(body),
                "AC4: the {event} arm does not kill the timer: {body}"
            );
        }
    }
}

fn sha256(path: &Path) -> Vec<u8> {
    Sha256::digest(std::fs::read(path).unwrap()).to_vec()
}

#[test]
fn ac5_drop_runs_copy_to_png_original_unchanged_output_new_next_to_it() {
    use kawaro_core::Registry;
    use kawaro_core::engine::CancelToken;
    use kawaro_core::queue::run_job;
    use kawaro_engines::image::ImageEngine;

    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("photo.bmp");
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/images/sample.bmp");
    std::fs::copy(&fixture, &original).unwrap();
    let before = sha256(&original);
    let listing_before: Vec<PathBuf> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(listing_before, vec![original.clone()]);

    // The hive for this file, built from the real actions.
    let registry = Registry::builtin();
    let actions = registry.matching(&[original.as_path()], &|_| true);
    let cells: Vec<HiveCell> = actions.iter().map(|a| cell(&a.id, a.ring)).collect();
    let p = plan(cells, false);
    let png = p
        .cells
        .iter()
        .position(|c| c.cell.id == "image.to-png")
        .expect("image.to-png is offered for a BMP");

    let mut t = DropTracker::default();
    t.on_hover(vec![original.clone()]);
    let DropOutcome::Run { cell: hit, files } = t.on_drop(&p, over(&p, png)) else {
        panic!("AC5: a drop over the PNG cell must run");
    };
    assert_eq!(hit as usize, png);
    assert_eq!(files, vec![original.clone()]);

    let action = registry.get(&p.cells[hit as usize].cell.id).unwrap();
    let (tx, _rx) = std::sync::mpsc::channel();
    let outputs = run_job(
        1,
        action,
        &files,
        &ImageEngine::default(),
        &CancelToken::new(),
        &tx,
    );

    // The original is still there, byte for byte.
    assert!(original.exists(), "AC5: the original was removed");
    assert_eq!(sha256(&original), before, "AC5: the original changed");
    // One new file, next to it, a real PNG of the same size.
    assert_eq!(outputs.len(), 1, "AC5: one output expected");
    let out = &outputs[0];
    assert_ne!(out, &original);
    assert!(!listing_before.contains(out), "AC5: the output is not new");
    assert_eq!(out.parent(), original.parent());
    assert_eq!(out.extension().and_then(|e| e.to_str()), Some("png"));
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(out).unwrap()));
    let reader = decoder.read_info().expect("AC5: the output is a PNG");
    let info = reader.info();
    assert_eq!((info.width, info.height), (40, 30));
    let mut listing_after: Vec<PathBuf> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    listing_after.sort();
    let mut expected = vec![original.clone(), out.clone()];
    expected.sort();
    assert_eq!(listing_after, expected, "AC5: no stray or missing files");
}

#[test]
fn ac6_drop_hive_allowed_four_combinations() {
    assert!(drop_hive_allowed(Source::Shortcut, true));
    assert!(!drop_hive_allowed(Source::Shortcut, false));
    assert!(drop_hive_allowed(Source::Menu, true));
    assert!(drop_hive_allowed(Source::Menu, false));
}
