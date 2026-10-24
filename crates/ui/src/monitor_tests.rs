//! S-046 AC1 and AC2 (backlog/S-046-windows-hive-window.md): the work area
//! under the pointer, and the hive plan kept inside it.
//!
//! Contract (tester, from AC1; do not edit). In `crate::monitor`:
//! - `#[derive(Clone, Copy, Debug, PartialEq)] pub struct Rect { pub x: i32,
//!   pub y: i32, pub w: i32, pub h: i32 }`, the order of AC1's `(x, y, w, h)`.
//! - `pub fn work_area_for(point: (i32, i32), monitors: &[Rect]) -> Rect`:
//!   the monitor containing the point (right and bottom edges exclusive),
//!   else the nearest one by Euclidean distance to its rectangle, and
//!   `Rect { x: 0, y: 0, w: 1920, h: 1080 }` for an empty list.

use crate::HiveCell;
use crate::layout::plan_at;
use crate::monitor::{Rect, work_area_for};
use crate::tokens::HEX_SIZE_MD;

const A: Rect = Rect {
    x: 0,
    y: 0,
    w: 1920,
    h: 1040,
};
const B: Rect = Rect {
    x: -1280,
    y: 0,
    w: 1280,
    h: 984,
};

fn at(point: (i32, i32)) -> Rect {
    work_area_for(point, &[A, B])
}

#[test]
fn ac1_point_inside_a_gives_a() {
    assert_eq!(at((100, 100)), A);
}

#[test]
fn ac1_point_inside_b_gives_b() {
    assert_eq!(at((-5, 10)), B);
}

#[test]
fn ac1_last_pixel_of_a_gives_a() {
    assert_eq!(at((1919, 1039)), A);
}

#[test]
fn ac1_point_outside_all_gives_the_nearest() {
    assert_eq!(at((3000, 500)), A);
}

#[test]
fn ac1_the_list_order_does_not_matter() {
    assert_eq!(work_area_for((-5, 10), &[B, A]), B);
    assert_eq!(work_area_for((100, 100), &[B, A]), A);
}

#[test]
fn ac1_the_boundary_between_b_and_a_belongs_to_a() {
    assert_eq!(at((0, 10)), A);
    assert_eq!(at((-1, 10)), B);
}

#[test]
fn ac1_below_b_but_beside_it_the_closer_monitor_wins() {
    // Just under B (984 high): distance 1 to B, 5 to A.
    assert_eq!(at((-5, 985)), B);
    // 17 under B, 1 left of A.
    assert_eq!(at((-1, 1000)), A);
}

#[test]
fn ac1_empty_list_gives_the_default_1080p() {
    assert_eq!(
        work_area_for((5, 5), &[]),
        Rect {
            x: 0,
            y: 0,
            w: 1920,
            h: 1080
        }
    );
}

#[test]
fn ac1_a_single_monitor_is_returned_for_any_point() {
    assert_eq!(work_area_for((-9999, 9999), &[B]), B);
}

fn cell(i: usize, ring: u8) -> HiveCell {
    HiveCell {
        id: format!("c{i}"),
        code: format!("C{i}"),
        label: format!("label {i}"),
        icon: String::new(),
        ring,
    }
}

#[test]
fn ac2_cursor_at_the_top_right_edge_keeps_every_cell_inside_area_a() {
    let cells: Vec<HiveCell> = (0..12)
        .map(|i| cell(i, if i < 6 { 1 } else { 2 }))
        .collect();
    let (plan, place) = plan_at(
        cells,
        false,
        1,
        (A.w as u32, A.h as u32),
        Some((1900.0, 20.0)),
        120,
    );
    assert!(!plan.cells.is_empty());
    let half_w = HEX_SIZE_MD / 2.0;
    let radius = HEX_SIZE_MD / 3f32.sqrt();
    for p in &plan.cells {
        let (cx, cy) = (place.x as f32 + p.x, place.y as f32 + p.y);
        assert!(
            cx - half_w >= -0.01 && cx + half_w <= A.w as f32 + 0.01,
            "cell {} x {cx} leaves the area",
            p.cell.id
        );
        assert!(
            cy - radius >= -0.01 && cy + radius <= A.h as f32 + 0.01,
            "cell {} y {cy} leaves the area",
            p.cell.id
        );
    }
}
