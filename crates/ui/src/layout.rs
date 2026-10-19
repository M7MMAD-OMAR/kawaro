//! Maps the cells given to `Hive::present` onto hive slots, and keyboard
//! navigation between them. Pure logic, no Slint, so it is unit tested and
//! mutation tested on its own.
//!
//! Slot rules (the contract leaves the edge cases open, these are decisions):
//! - `ring <= 1` means ring 1, anything above means ring 2.
//! - Ring 1 takes the first 6 ring-1 cells in the given order. Ring-1 cells
//!   beyond 6 move to ring 2, ahead of the ring-2 cells, because they were
//!   ranked higher.
//! - Ring 2 takes the first 12 of that queue. The rest is dropped, so at most
//!   18 cells show.
//! - A ring-2 cell never moves inwards to fill a short ring 1: ring 1 is for
//!   the common actions and an empty slot there reads better than a rare
//!   action in it.
//! - Each ring fills its slots from the top-right cell clockwise (LTR), or
//!   from the top-left cell counterclockwise (Arabic), see `geometry::ring`.

use crate::HiveCell;
use crate::geometry::{Axial, Layout, ring};
use crate::tokens::{HEX_GAP, HEX_SIZE_MD, PAD, RING1, RING2};

/// A cell with its slot and its pixel centre inside the hive window.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub cell: HiveCell,
    pub axial: Axial,
    pub x: f32,
    pub y: f32,
}

/// Everything the window needs for one `present` call.
#[derive(Clone, Debug)]
pub struct Plan {
    pub cells: Vec<Placed>,
    pub width: f32,
    pub height: f32,
    pub layout: Layout,
}

/// Splits cells into ring 1 and ring 2 following the slot rules above.
pub fn assign(cells: Vec<HiveCell>) -> (Vec<HiveCell>, Vec<HiveCell>) {
    let (mut ring1, ring2): (Vec<_>, Vec<_>) = cells.into_iter().partition(|c| c.ring <= 1);
    let mut outer: Vec<HiveCell> = if ring1.len() > RING1 {
        ring1.split_off(RING1)
    } else {
        Vec::new()
    };
    outer.extend(ring2);
    outer.truncate(RING2);
    (ring1, outer)
}

/// Lays out the cells for a window, using the token sizes. The window holds
/// one ring when ring 2 is empty, two otherwise.
pub fn plan(cells: Vec<HiveCell>, rtl: bool) -> Plan {
    let (ring1, ring2) = assign(cells);
    let rings = if ring2.is_empty() { 1 } else { 2 };
    let radius = HEX_SIZE_MD / 3f32.sqrt();
    let (width, height) = Layout::extent(radius, HEX_GAP, rings, PAD);
    let layout = Layout {
        radius,
        gap: HEX_GAP,
        rtl,
        origin: (width / 2.0, height / 2.0),
    };
    let slots = ring(1)
        .into_iter()
        .zip(ring1)
        .chain(ring(2).into_iter().zip(ring2));
    let cells = slots
        .map(|(axial, cell)| {
            let (x, y) = layout.center(axial);
            Placed { cell, axial, x, y }
        })
        .collect();
    Plan {
        cells,
        width,
        height,
        layout,
    }
}

impl Plan {
    /// Index into `cells` of the action under the point, or -1 for the
    /// centre, a gap, an empty slot or the transparent corners.
    pub fn pick(&self, x: f32, y: f32) -> i32 {
        let slots: Vec<Axial> = self.cells.iter().map(|c| c.axial).collect();
        self.layout.pick(&slots, x, y).map_or(-1, |i| i as i32)
    }
}

/// A navigation key, as sent by the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Nav {
    Next,
    Prev,
    Left,
    Right,
    Up,
    Down,
}

impl Nav {
    /// The integer code the .slint side sends: 0 Tab, 1 Shift+Tab, 2 Left,
    /// 3 Right, 4 Up, 5 Down.
    pub fn from_code(code: i32) -> Option<Nav> {
        match code {
            0 => Some(Nav::Next),
            1 => Some(Nav::Prev),
            2 => Some(Nav::Left),
            3 => Some(Nav::Right),
            4 => Some(Nav::Up),
            5 => Some(Nav::Down),
            _ => None,
        }
    }
}

/// Moves the keyboard highlight. `current` is -1 when nothing is highlighted.
/// Next, Down and the arrow pointing along the reading direction (Right in
/// LTR, Left in Arabic) go forward through the ring order; the others go
/// back. Wraps at both ends. Returns -1 when there are no cells.
pub fn step(current: i32, nav: Nav, count: usize, rtl: bool) -> i32 {
    if count == 0 {
        return -1;
    }
    let forward = match nav {
        Nav::Next | Nav::Down => true,
        Nav::Prev | Nav::Up => false,
        Nav::Right => !rtl,
        Nav::Left => rtl,
    };
    let n = count as i32;
    if current < 0 {
        return if forward { 0 } else { n - 1 };
    }
    let delta = if forward { 1 } else { -1 };
    (current + delta).rem_euclid(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(id: &str, ring: u8) -> HiveCell {
        HiveCell {
            id: id.into(),
            code: id.to_uppercase(),
            label: format!("label {id}"),
            ring,
        }
    }

    fn ids(v: &[HiveCell]) -> Vec<&str> {
        v.iter().map(|c| c.id.as_str()).collect()
    }

    fn many(prefix: &str, n: usize, ring: u8) -> Vec<HiveCell> {
        (0..n)
            .map(|i| cell(&format!("{prefix}{i}"), ring))
            .collect()
    }

    #[test]
    fn ring_one_and_two_keep_their_order() {
        let (r1, r2) = assign(vec![cell("webp", 1), cell("gray", 2), cell("png", 1)]);
        assert_eq!(ids(&r1), ["webp", "png"]);
        assert_eq!(ids(&r2), ["gray"]);
    }

    #[test]
    fn ring_zero_counts_as_ring_one_and_above_two_as_ring_two() {
        let (r1, r2) = assign(vec![cell("a", 0), cell("b", 3), cell("c", 255)]);
        assert_eq!(ids(&r1), ["a"]);
        assert_eq!(ids(&r2), ["b", "c"]);
    }

    #[test]
    fn ring_one_overflow_moves_to_the_front_of_ring_two() {
        let mut cells = many("r1-", 8, 1);
        cells.insert(0, cell("late", 2));
        let (r1, r2) = assign(cells);
        assert_eq!(ids(&r1), ["r1-0", "r1-1", "r1-2", "r1-3", "r1-4", "r1-5"]);
        assert_eq!(ids(&r2), ["r1-6", "r1-7", "late"]);
    }

    #[test]
    fn more_than_eighteen_cells_are_dropped() {
        let mut cells = many("a", 10, 1);
        cells.extend(many("b", 10, 2));
        let (r1, r2) = assign(cells);
        assert_eq!(r1.len(), 6);
        assert_eq!(r2.len(), 12);
        assert_eq!(ids(&r2)[..4], ["a6", "a7", "a8", "a9"]);
        assert_eq!(ids(&r2)[11], "b7");
    }

    #[test]
    fn short_ring_one_is_not_filled_from_ring_two() {
        let mut cells = many("a", 2, 1);
        cells.extend(many("b", 14, 2));
        let (r1, r2) = assign(cells);
        assert_eq!(ids(&r1), ["a0", "a1"]);
        assert_eq!(r2.len(), 12);
        assert_eq!(ids(&r2)[11], "b11");
    }

    #[test]
    fn plan_puts_cells_on_ring_slots_in_order() {
        let mut cells = many("a", 3, 1);
        cells.extend(many("b", 2, 2));
        let p = plan(cells, false);
        let slots: Vec<Axial> = p.cells.iter().map(|c| c.axial).collect();
        let r1 = ring(1);
        let r2 = ring(2);
        assert_eq!(slots, [r1[0], r1[1], r1[2], r2[0], r2[1]]);
        assert_eq!(
            ids(&p.cells.iter().map(|c| c.cell.clone()).collect::<Vec<_>>()),
            ["a0", "a1", "a2", "b0", "b1"]
        );
        for c in &p.cells {
            assert_eq!((c.x, c.y), p.layout.center(c.axial));
        }
    }

    #[test]
    fn plan_centres_the_hive_in_a_window_sized_for_the_rings_used() {
        let one = plan(many("a", 3, 1), false);
        let two = plan(vec![cell("a", 1), cell("b", 2)], false);
        let empty = plan(Vec::new(), false);
        assert!(one.width < two.width && one.height < two.height);
        assert_eq!((empty.width, empty.height), (one.width, one.height));
        for p in [&one, &two, &empty] {
            assert_eq!(p.layout.origin, (p.width / 2.0, p.height / 2.0));
        }
        let radius = crate::tokens::HEX_SIZE_MD / 3f32.sqrt();
        let (w, h) = Layout::extent(radius, crate::tokens::HEX_GAP, 2, crate::tokens::PAD);
        assert!((two.width - w).abs() < 1e-3 && (two.height - h).abs() < 1e-3);
        assert!((two.layout.radius - radius).abs() < 1e-3);
        assert_eq!(two.layout.gap, crate::tokens::HEX_GAP);
    }

    #[test]
    fn plan_mirrors_for_arabic() {
        let cells = vec![cell("a", 1), cell("b", 2)];
        let ltr = plan(cells.clone(), false);
        let rtl = plan(cells, true);
        assert!(rtl.layout.rtl && !ltr.layout.rtl);
        for (l, r) in ltr.cells.iter().zip(&rtl.cells) {
            assert_eq!(l.axial, r.axial);
            assert!((l.x - ltr.layout.origin.0 + (r.x - rtl.layout.origin.0)).abs() < 1e-3);
            assert_eq!(l.y, r.y);
        }
        // First ring-1 cell: top right in LTR, top left in Arabic.
        assert!(ltr.cells[0].x > ltr.layout.origin.0);
        assert!(rtl.cells[0].x < rtl.layout.origin.0);
    }

    #[test]
    fn pick_returns_action_index_and_minus_one_elsewhere() {
        let p = plan(vec![cell("a", 1), cell("b", 1), cell("c", 2)], true);
        for (i, c) in p.cells.iter().enumerate() {
            assert_eq!(p.pick(c.x, c.y), i as i32);
        }
        let (ox, oy) = p.layout.origin;
        assert_eq!(p.pick(ox, oy), -1, "centre");
        assert_eq!(p.pick(0.0, 0.0), -1, "corner");
        // Empty ring-1 slot (third slot is unused).
        let (ex, ey) = p.layout.center(ring(1)[2]);
        assert_eq!(p.pick(ex, ey), -1, "empty slot");
    }

    #[test]
    fn nav_codes_match_the_slint_side() {
        let want = [
            Nav::Next,
            Nav::Prev,
            Nav::Left,
            Nav::Right,
            Nav::Up,
            Nav::Down,
        ];
        for (code, nav) in want.iter().enumerate() {
            assert_eq!(Nav::from_code(code as i32), Some(*nav));
        }
        assert_eq!(Nav::from_code(6), None);
        assert_eq!(Nav::from_code(-1), None);
    }

    #[test]
    fn step_moves_forward_and_back_with_wrap() {
        assert_eq!(step(-1, Nav::Next, 4, false), 0);
        assert_eq!(step(-1, Nav::Prev, 4, false), 3);
        assert_eq!(step(0, Nav::Next, 4, false), 1);
        assert_eq!(step(3, Nav::Next, 4, false), 0);
        assert_eq!(step(0, Nav::Prev, 4, false), 3);
        assert_eq!(step(2, Nav::Prev, 4, false), 1);
        assert_eq!(step(1, Nav::Down, 4, false), 2);
        assert_eq!(step(1, Nav::Up, 4, false), 0);
        assert_eq!(step(-1, Nav::Next, 0, false), -1);
        assert_eq!(
            step(5, Nav::Next, 3, false),
            0,
            "stale index wraps into range"
        );
    }

    #[test]
    fn arrows_follow_the_reading_direction() {
        assert_eq!(step(1, Nav::Right, 4, false), 2);
        assert_eq!(step(1, Nav::Left, 4, false), 0);
        assert_eq!(step(1, Nav::Right, 4, true), 0);
        assert_eq!(step(1, Nav::Left, 4, true), 2);
        // Tab and vertical arrows do not depend on the direction.
        assert_eq!(step(1, Nav::Next, 4, true), 2);
        assert_eq!(step(1, Nav::Down, 4, true), 2);
        assert_eq!(step(1, Nav::Up, 4, true), 0);
    }
}
