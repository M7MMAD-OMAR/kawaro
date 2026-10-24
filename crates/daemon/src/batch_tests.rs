//! S-046 AC4 and AC8 (backlog/S-046-windows-hive-window.md): the focus log
//! line and the batching of `Open` requests with a fake clock.
//!
//! Contract (tester, from AC4 and AC8; do not edit):
//! - `crate::server::focus_log(ok: bool) -> Option<&'static str>`: `None`
//!   when `SetForegroundWindow` succeeded, else
//!   `Some("sbarhived: foreground refused")`.
//! - `crate::batch::OpenBatch::new(cells_for: impl Fn(&[PathBuf]) -> Vec<String>
//!   and 'static)`, where `cells_for` gives the cell ids for a file set (the
//!   daemon passes the registry; the `Actions` response is built the same
//!   way). `push(&mut self, at_ms: u64, path: PathBuf) -> Step`, with
//!   `#[derive(Debug, PartialEq)] pub enum Step { Show { files: Vec<PathBuf>,
//!   cells: Vec<String> }, Rebuild { files: Vec<PathBuf>, cells:
//!   Vec<String> } }`. The first path of a batch gives `Show` at once, with
//!   no wait; a later path inside the window gives `Rebuild` with all files
//!   so far and the cells for all of them. It uses `core::merge::Collector`.

use crate::batch::{OpenBatch, Step};
use crate::server::focus_log;
use std::path::{Path, PathBuf};

#[test]
fn ac4_focus_log_is_silent_when_the_call_worked() {
    assert_eq!(focus_log(true), None);
}

#[test]
fn ac4_focus_log_names_the_refusal() {
    assert_eq!(focus_log(false), Some("sbarhived: foreground refused"));
}

/// Images share one set of actions; a mix of images and a PDF has another.
fn cells_for(files: &[PathBuf]) -> Vec<String> {
    let all_png = files
        .iter()
        .all(|f| f.extension().is_some_and(|e| e == "png"));
    if all_png {
        vec!["png-to-webp".into(), "png-to-jpg".into(), "resize".into()]
    } else {
        vec!["merge-pdf".into()]
    }
}

fn p(s: &str) -> PathBuf {
    Path::new(s).to_path_buf()
}

#[test]
fn ac8_first_open_shows_at_once_and_later_ones_rebuild_for_all_files() {
    let mut b = OpenBatch::new(cells_for);
    let steps = [
        b.push(0, p("a.png")),
        b.push(100, p("b.png")),
        b.push(200, p("c.pdf")),
    ];
    let shows = steps
        .iter()
        .filter(|s| matches!(s, Step::Show { .. }))
        .count();
    assert_eq!(shows, 1);
    assert!(
        matches!(steps[0], Step::Show { .. }),
        "no wait for the first"
    );
    let all = vec![p("a.png"), p("b.png"), p("c.pdf")];
    match &steps[2] {
        Step::Rebuild { files, cells } => {
            assert_eq!(files, &all);
            assert_eq!(cells, &cells_for(&all));
            assert_ne!(cells, &cells_for(&[p("a.png")]));
        }
        other => panic!("expected a rebuild, got {other:?}"),
    }
}

#[test]
fn ac8_the_first_show_carries_the_cells_of_the_first_file_only() {
    let mut b = OpenBatch::new(cells_for);
    assert_eq!(
        b.push(0, p("a.png")),
        Step::Show {
            files: vec![p("a.png")],
            cells: cells_for(&[p("a.png")]),
        }
    );
}

#[test]
fn ac8_after_a_gap_of_300_the_next_open_shows_again_with_only_its_files() {
    let mut b = OpenBatch::new(cells_for);
    b.push(0, p("a.png"));
    b.push(100, p("b.png"));
    assert_eq!(
        b.push(400, p("c.pdf")),
        Step::Show {
            files: vec![p("c.pdf")],
            cells: cells_for(&[p("c.pdf")]),
        }
    );
}
