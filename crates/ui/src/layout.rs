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
//!   action in it. Exception: when every cell fits in ring 1 (6 or fewer in
//!   total), all of them go there, ring-1 cells first, so the hive is one
//!   compact ring instead of a lopsided two-ring window.
//! - A ring that is the only ring and is not full spreads its cells evenly
//!   over the six slots, so three actions sit at alternating slots.
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
    if ring1.len() + ring2.len() <= RING1 {
        ring1.extend(ring2);
        return (ring1, Vec::new());
    }
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
    plan_rings(cells, rtl, 1)
}

/// As [`plan`], with a window of at least `min_rings` rings. The drop hive
/// uses 2, so its size and centre stay the same when its cells arrive.
pub fn plan_rings(cells: Vec<HiveCell>, rtl: bool, min_rings: i32) -> Plan {
    let (ring1, ring2) = assign(cells);
    let rings = if ring2.is_empty() { 1 } else { 2 }.max(min_rings);
    let radius = HEX_SIZE_MD / 3f32.sqrt();
    let (width, height) = Layout::extent(radius, HEX_GAP, rings, PAD);
    let layout = Layout {
        radius,
        gap: HEX_GAP,
        rtl,
        origin: (width / 2.0, height / 2.0),
    };
    let inner = ring(1);
    let n = ring1.len();
    let ring1_slots: Vec<Axial> = if rings == 1 && n > 0 && n < RING1 {
        (0..n).map(|i| inner[i * RING1 / n]).collect()
    } else {
        inner
    };
    let slots = ring1_slots
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

/// The outermost ring the hive at an anchor may use (S-019 decision 5).
// S-019c and S-019d wire `plan_at` into the show path; until then only the
// tests call it.
#[cfg_attr(not(test), allow(dead_code))]
pub const MAX_RINGS: i32 = 4;

/// The hive with its centre at `anchor` on a `screen` sized output (S-019
/// AC3), and where it sits there at scale `scale120`/120. Without an anchor
/// it is [`plan_rings`] centred by [`place`](crate::overlay::place).
///
/// The anchor is clamped so the centre cell plus `PAD` is on screen. A slot
/// is used only when its hexagon plus `PAD` is on screen too. Ring-1 cells
/// take the accepted ring-1 slots; the rest, ring-1 overflow first, fill the
/// accepted slots of rings 2 to [`MAX_RINGS`] in spiral order. The surface
/// is the bounding box of the centre and every accepted slot of the rings
/// it sizes for.
#[cfg_attr(not(test), allow(dead_code))]
pub fn plan_at(
    cells: Vec<HiveCell>,
    rtl: bool,
    min_rings: i32,
    screen: (u32, u32),
    anchor: Option<(f32, f32)>,
    scale120: u32,
) -> (Plan, crate::overlay::Placement) {
    let Some(anchor) = anchor else {
        let plan = plan_rings(cells, rtl, min_rings);
        let size = (plan.width.ceil() as u32, plan.height.ceil() as u32);
        let placement = crate::overlay::place(screen, size, scale120);
        return (plan, placement);
    };
    const EPS: f32 = 0.01;
    let radius = HEX_SIZE_MD / 3f32.sqrt();
    // The same half width `Layout::extent` uses, so a hive with every slot
    // accepted has the size of `plan_rings`.
    let (mx, my) = (radius * 3f32.sqrt() / 2.0 + PAD, radius + PAD);
    let (w, h) = (screen.0 as f32, screen.1 as f32);
    let centre = (anchor.0.max(mx).min(w - mx), anchor.1.max(my).min(h - my));
    let offsets = Layout {
        radius,
        gap: HEX_GAP,
        rtl,
        origin: (0.0, 0.0),
    };
    let fits = |a: &Axial| {
        let (x, y) = offsets.center(*a);
        let (x, y) = (centre.0 + x, centre.1 + y);
        // The centre of every slot ranges where the clamped anchor may.
        (mx - EPS..=(w - mx) + EPS).contains(&x) && (my - EPS..=(h - my) + EPS).contains(&y)
    };
    let accepted: Vec<Vec<Axial>> = (1..=MAX_RINGS)
        .map(|k| ring(k).into_iter().filter(fits).collect())
        .collect();

    let (ring1, ring2) = assign(cells);
    let inner = &accepted[0];
    let n1 = ring1.len().min(inner.len());
    let mut ring1 = ring1;
    let mut outer = ring1.split_off(n1);
    outer.extend(ring2);
    let outer_slots: Vec<(i32, Axial)> = (2..=MAX_RINGS)
        .zip(&accepted[1..])
        .flat_map(|(k, v)| v.iter().map(move |a| (k, *a)))
        .collect();
    outer.truncate(outer_slots.len());
    let used = outer.len().checked_sub(1).map_or(1, |i| outer_slots[i].0);
    let rings = used.max(min_rings);
    // A lone ring spreads its cells; with every slot filled this is the
    // slots in order, and with no cell it is empty.
    let ring1_slots: Vec<Axial> = if rings == 1 {
        (0..n1).map(|i| inner[i * inner.len() / n1]).collect()
    } else {
        inner.clone()
    };

    let mut b = (0f32, 0f32, 0f32, 0f32);
    for a in accepted.iter().take(rings as usize).flatten() {
        let (x, y) = offsets.center(*a);
        b = (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y));
    }
    let (width, height) = (b.2 - b.0 + 2.0 * mx, b.3 - b.1 + 2.0 * my);
    let layout = Layout {
        origin: (mx - b.0, my - b.1),
        ..offsets
    };
    let slots = ring1_slots
        .into_iter()
        .zip(ring1)
        .chain(outer_slots.into_iter().map(|(_, a)| a).zip(outer));
    let cells = slots
        .map(|(axial, cell)| {
            let (x, y) = layout.center(axial);
            Placed { cell, axial, x, y }
        })
        .collect();
    let size = (width.ceil() as u32, height.ceil() as u32);
    let placement = crate::overlay::Placement {
        x: (centre.0 - layout.origin.0).round() as i32,
        y: (centre.1 - layout.origin.1).round() as i32,
        ..crate::overlay::place(screen, size, scale120)
    };
    let plan = Plan {
        cells,
        width,
        height,
        layout,
    };
    (plan, placement)
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
            icon: String::new(),
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
        let mut cells = vec![cell("webp", 1), cell("gray", 2), cell("png", 1)];
        cells.extend(many("x", 5, 2));
        let (r1, r2) = assign(cells);
        assert_eq!(ids(&r1), ["webp", "png"]);
        assert_eq!(ids(&r2), ["gray", "x0", "x1", "x2", "x3", "x4"]);
    }

    #[test]
    fn ring_zero_counts_as_ring_one_and_above_two_as_ring_two() {
        let mut cells = vec![cell("a", 0), cell("b", 3), cell("c", 255)];
        cells.extend(many("x", 4, 2));
        let (r1, r2) = assign(cells);
        assert_eq!(ids(&r1), ["a"]);
        assert_eq!(ids(&r2), ["b", "c", "x0", "x1", "x2", "x3"]);
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
    fn six_or_fewer_cells_all_go_to_ring_one() {
        let (r1, r2) = assign(vec![cell("gray", 2), cell("webp", 1), cell("png", 1)]);
        assert_eq!(ids(&r1), ["webp", "png", "gray"]);
        assert!(r2.is_empty());
        let mut six = many("a", 4, 1);
        six.extend(many("b", 2, 2));
        let (r1, r2) = assign(six);
        assert_eq!(r1.len(), 6);
        assert!(r2.is_empty());
        let mut seven = many("a", 4, 1);
        seven.extend(many("b", 3, 2));
        let (r1, r2) = assign(seven);
        assert_eq!((r1.len(), r2.len()), (4, 3));
    }

    #[test]
    fn a_partial_single_ring_spreads_evenly() {
        let r1 = ring(1);
        let slots = |n: usize| -> Vec<Axial> {
            plan(many("a", n, 1), false)
                .cells
                .iter()
                .map(|c| c.axial)
                .collect()
        };
        assert_eq!(slots(1), [r1[0]]);
        assert_eq!(slots(2), [r1[0], r1[3]]);
        assert_eq!(slots(3), [r1[0], r1[2], r1[4]]);
        assert_eq!(slots(4), [r1[0], r1[1], r1[3], r1[4]]);
        assert_eq!(slots(6), r1);
        // With a second ring the inner ring keeps consecutive slots.
        let mut cells = many("a", 2, 1);
        cells.extend(many("b", 5, 2));
        let p = plan(cells, false);
        assert_eq!(p.cells[0].axial, r1[0]);
        assert_eq!(p.cells[1].axial, r1[1]);
    }

    #[test]
    fn plan_puts_cells_on_ring_slots_in_order() {
        let mut cells = many("a", 3, 1);
        cells.extend(many("b", 4, 2));
        let p = plan(cells, false);
        let slots: Vec<Axial> = p.cells.iter().map(|c| c.axial).collect();
        let r1 = ring(1);
        let r2 = ring(2);
        assert_eq!(slots, [r1[0], r1[1], r1[2], r2[0], r2[1], r2[2], r2[3]]);
        assert_eq!(
            ids(&p.cells.iter().map(|c| c.cell.clone()).collect::<Vec<_>>()),
            ["a0", "a1", "a2", "b0", "b1", "b2", "b3"]
        );
        for c in &p.cells {
            assert_eq!((c.x, c.y), p.layout.center(c.axial));
        }
    }

    #[test]
    fn plan_centres_the_hive_in_a_window_sized_for_the_rings_used() {
        let one = plan(many("a", 3, 1), false);
        let mut seven = many("a", 1, 1);
        seven.extend(many("b", 6, 2));
        let two = plan(seven, false);
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
        // Empty ring-1 slot: three cells spread over slots 0, 2 and 4.
        let (ex, ey) = p.layout.center(ring(1)[1]);
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

    #[test]
    fn a_two_ring_minimum_keeps_the_window_and_the_first_slot() {
        let full = plan(many("x", 18, 1), false);
        for n in [0, 3, 18] {
            let p = plan_rings(many("x", n, 1), false, 2);
            assert_eq!((p.width, p.height), (full.width, full.height), "{n}");
            assert_eq!(p.layout.origin, full.layout.origin, "{n}");
            if n > 0 {
                assert_eq!(
                    (p.cells[0].x, p.cells[0].y),
                    (full.cells[0].x, full.cells[0].y)
                );
            }
        }
        let one = plan_rings(many("x", 3, 1), false, 1);
        assert!(one.width < full.width);
    }
}
