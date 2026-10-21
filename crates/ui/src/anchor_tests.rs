//! S-019b (backlog/S-019-hive-at-pointer.md, AC3 and AC4): the hive at an
//! anchor, with a shape that adapts to the screen edges. Pure maths, under
//! `cargo mutants` (manager decision 7).
//!
//! Contract (written by the tester from the acceptance criteria; do not edit):
//! - `crate::layout::MAX_RINGS: i32 = 4` (manager decision 5).
//! - `crate::layout::plan_at(cells: Vec<HiveCell>, rtl: bool, min_rings: i32,
//!   screen: (u32, u32), anchor: Option<(f32, f32)>, scale120: u32)
//!   -> (Plan, crate::overlay::Placement)`. AC3 proposes the name and the
//!   first five parameters; AC4 asks for the buffer at scale 180, so the
//!   fractional-scale-v1 value is a sixth parameter and the result carries
//!   the placement as `place` does:
//!   - `Placement::x, y`: the hive subsurface's top-left corner on the output,
//!     in logical pixels, so the centre cell's centre on screen is
//!     `(x, y) + plan.layout.origin`.
//!   - `Placement::destination`: `(ceil(plan.width), ceil(plan.height))`.
//!   - `Placement::buffer`: `ceil(destination * scale120 / 120)`.
//!   - `screen` and `anchor` are logical pixels on that output.
//! - `plan`, `plan_rings` and `place` keep their signatures and results.
//!
//! Rules checked (AC3), with `r = HEX_SIZE_MD / sqrt(3)`:
//! - No anchor: the cells and size of `plan_rings`, and the placement of
//!   `place` for that size.
//! - The anchor is clamped so the centre hexagon (width `HEX_SIZE_MD`, height
//!   `2r`) plus `PAD` lies on screen, and moved no further.
//! - A slot is accepted when its hexagon plus `PAD` lies on screen
//!   (inclusive; the tests allow 0.01 px for f32 rounding).
//! - Slots in spiral order: ring 1, then 2, up to `MAX_RINGS`, each ring in
//!   `geometry::ring` order (drawn mirrored in Arabic).
//! - Ring-1 cells of `assign` take the accepted ring-1 slots; the overflow
//!   goes ahead of the ring-2 cells into the outer rings. A ring-2 cell never
//!   enters ring 1, and unfilled ring-1 slots stay empty.
//! - The only ring, when not full, spreads its cells evenly over its accepted
//!   slots (as `plan_rings` does over six).
//! - The subsurface is the bounding box of the centre and every accepted slot
//!   in rings 1 to `max(min_rings, rings used)`, each hexagon plus `PAD`.

use crate::HiveCell;
use crate::geometry::{Axial, Layout, ring};
use crate::layout::{MAX_RINGS, Plan, assign, plan_at, plan_rings};
use crate::overlay::{Placement, place};
use crate::tokens::{HEX_GAP, HEX_SIZE_MD, PAD};

const SCREEN: (u32, u32) = (1280, 800);
const EPS: f32 = 0.01;

/// AC4: the four corners, the four edge midpoints, the middle and (400, 500).
const ANCHORS: [(f32, f32); 10] = [
    (0.0, 0.0),
    (1280.0, 0.0),
    (0.0, 800.0),
    (1280.0, 800.0),
    (640.0, 0.0),
    (1280.0, 400.0),
    (640.0, 800.0),
    (0.0, 400.0),
    (640.0, 400.0),
    (400.0, 500.0),
];

const COUNTS: [usize; 4] = [18, 12, 6, 3];

fn cell(id: &str, ring: u8) -> HiveCell {
    HiveCell {
        id: id.into(),
        code: id.to_uppercase(),
        label: format!("label {id}"),
        icon: String::new(),
        ring,
    }
}

/// `n` cells with a ring mix: 18 is 6 ring-1 and 12 ring-2 cells, 12 is 4 and
/// 8 (ring 1 short with ring-2 cells waiting), 6 is 4 and 2 (`assign` puts
/// all six in ring 1), 3 and 2 are ring-1 cells only.
fn cells(n: usize) -> Vec<HiveCell> {
    let ones = match n {
        18 => 6,
        12 => 4,
        6 => 4,
        _ => n,
    };
    let mut v: Vec<HiveCell> = (0..ones).map(|i| cell(&format!("a{i}"), 1)).collect();
    v.extend((0..n - ones).map(|i| cell(&format!("b{i}"), 2)));
    v
}

fn radius() -> f32 {
    HEX_SIZE_MD / 3f32.sqrt()
}

fn half_w() -> f32 {
    HEX_SIZE_MD / 2.0
}

fn clamp(a: (f32, f32)) -> (f32, f32) {
    let (mx, my) = (half_w() + PAD, radius() + PAD);
    let (w, h) = (SCREEN.0 as f32, SCREEN.1 as f32);
    (a.0.clamp(mx, w - mx), a.1.clamp(my, h - my))
}

/// Offset of a slot from the centre cell, mirrored in Arabic.
fn offset(a: Axial, rtl: bool) -> (f32, f32) {
    Layout {
        radius: radius(),
        gap: HEX_GAP,
        rtl,
        origin: (0.0, 0.0),
    }
    .center(a)
}

fn accepted(c: (f32, f32)) -> bool {
    let (mx, my) = (half_w() + PAD, radius() + PAD);
    let (w, h) = (SCREEN.0 as f32, SCREEN.1 as f32);
    c.0 - mx >= -EPS && c.0 + mx <= w + EPS && c.1 - my >= -EPS && c.1 + my <= h + EPS
}

/// The oracle's answer for one case.
struct Expect {
    /// Clamped anchor: the centre cell's centre on screen.
    centre: (f32, f32),
    /// Cell id and slot, in `plan.cells` order.
    slots: Vec<(String, Axial)>,
    /// Bounding box of the slot centres that size the subsurface, as offsets
    /// from the centre cell: min x, min y, max x, max y.
    bounds: (f32, f32, f32, f32),
    /// Accepted slot count per ring, 1 to `MAX_RINGS`.
    accepted: Vec<usize>,
    ring1_cells: usize,
    other_cells: usize,
}

fn expect(cells: Vec<HiveCell>, rtl: bool, min_rings: i32, anchor: (f32, f32)) -> Expect {
    let c = clamp(anchor);
    let acc: Vec<Vec<Axial>> = (1..=4)
        .map(|k| {
            ring(k)
                .into_iter()
                .filter(|a| {
                    let o = offset(*a, rtl);
                    accepted((c.0 + o.0, c.1 + o.1))
                })
                .collect()
        })
        .collect();
    let (r1, r2) = assign(cells);
    let (ring1_cells, other_cells) = (r1.len(), r2.len());
    let n1 = r1.len().min(acc[0].len());
    let mut outer: Vec<HiveCell> = r1[n1..].to_vec();
    outer.extend(r2);
    let outer_slots: Vec<(i32, Axial)> = acc
        .iter()
        .enumerate()
        .skip(1)
        .flat_map(|(i, v)| v.iter().map(move |a| (i as i32 + 1, *a)))
        .collect();
    let n_out = outer.len().min(outer_slots.len());
    let used = if n_out > 0 {
        outer_slots[n_out - 1].0
    } else {
        1
    };
    let rings = used.max(min_rings);
    let inner = &acc[0];
    let spread = n_out == 0 && rings == 1 && n1 > 0 && n1 < inner.len();
    let ring1_slots: Vec<Axial> = if spread {
        (0..n1).map(|i| inner[i * inner.len() / n1]).collect()
    } else {
        inner[..n1].to_vec()
    };
    let mut slots: Vec<(String, Axial)> = r1[..n1]
        .iter()
        .zip(ring1_slots)
        .map(|(cell, a)| (cell.id.clone(), a))
        .collect();
    slots.extend(
        outer[..n_out]
            .iter()
            .zip(&outer_slots)
            .map(|(cell, (_, a))| (cell.id.clone(), *a)),
    );
    let mut b = (0f32, 0f32, 0f32, 0f32);
    for a in acc.iter().take(rings as usize).flatten() {
        let o = offset(*a, rtl);
        b = (b.0.min(o.0), b.1.min(o.1), b.2.max(o.0), b.3.max(o.1));
    }
    Expect {
        centre: c,
        slots,
        bounds: b,
        accepted: acc.iter().map(Vec::len).collect(),
        ring1_cells,
        other_cells,
    }
}

fn ids(plan: &Plan) -> Vec<(String, Axial)> {
    plan.cells
        .iter()
        .map(|p| (p.cell.id.clone(), p.axial))
        .collect()
}

fn hex_distance(a: Axial) -> i32 {
    (a.q.abs() + a.r.abs() + (a.q + a.r).abs()) / 2
}

/// Every AC4 check for one case.
fn check_case(n: usize, rtl: bool, min_rings: i32, anchor: (f32, f32), scale120: u32) {
    let ctx =
        format!("{n} cells, rtl {rtl}, min_rings {min_rings}, anchor {anchor:?}, scale {scale120}");
    let (plan, pl) = plan_at(cells(n), rtl, min_rings, SCREEN, Some(anchor), scale120);
    let e = expect(cells(n), rtl, min_rings, anchor);

    // Layout from the tokens, in the asked direction.
    assert!(
        (plan.layout.radius - radius()).abs() <= EPS,
        "{ctx}: radius"
    );
    assert!((plan.layout.gap - HEX_GAP).abs() <= EPS, "{ctx}: gap");
    assert_eq!(plan.layout.rtl, rtl, "{ctx}: rtl");

    // Which cells show, where, in priority order.
    assert_eq!(ids(&plan), e.slots, "{ctx}: cells and slots");

    // The count the priority rules fill, stated as AC4 writes it.
    let acc1 = e.accepted[0];
    let acc_outer: usize = e.accepted[1..].iter().sum();
    let first = e.ring1_cells.min(acc1);
    let rest = e.ring1_cells + e.other_cells - first;
    assert_eq!(
        plan.cells.len(),
        first + rest.min(acc_outer),
        "{ctx}: count"
    );

    // A cell `assign` puts in ring 2 never enters ring 1.
    let (_, second) = assign(cells(n));
    for p in &plan.cells {
        if second.iter().any(|c| c.id == p.cell.id) {
            assert!(hex_distance(p.axial) >= 2, "{ctx}: {} in ring 1", p.cell.id);
        }
        assert!(
            hex_distance(p.axial) <= MAX_RINGS,
            "{ctx}: beyond MAX_RINGS"
        );
    }

    // The centre cell sits at the clamped anchor within 0.5 px.
    let ox = pl.x as f32 + plan.layout.origin.0;
    let oy = pl.y as f32 + plan.layout.origin.1;
    assert!(
        (ox - e.centre.0).abs() <= 0.5 + EPS && (oy - e.centre.1).abs() <= 0.5 + EPS,
        "{ctx}: centre ({ox}, {oy}), clamped anchor {:?}",
        e.centre
    );

    // Each cell: its centre in the plan, on screen, inside the screen, picked.
    let (w, h) = (SCREEN.0 as f32, SCREEN.1 as f32);
    for (i, p) in plan.cells.iter().enumerate() {
        let (lx, ly) = plan.layout.center(p.axial);
        assert!(
            (p.x - lx).abs() <= EPS && (p.y - ly).abs() <= EPS,
            "{ctx}: cell {i} drawn off its slot"
        );
        let o = offset(p.axial, rtl);
        let (sx, sy) = (pl.x as f32 + p.x, pl.y as f32 + p.y);
        assert!(
            (sx - (e.centre.0 + o.0)).abs() <= 0.5 + EPS
                && (sy - (e.centre.1 + o.1)).abs() <= 0.5 + EPS,
            "{ctx}: cell {i} at ({sx}, {sy}) on screen"
        );
        assert!(
            sx - half_w() >= 0.0
                && sx + half_w() <= w
                && sy - radius() >= 0.0
                && sy + radius() <= h,
            "{ctx}: cell {i} hexagon at ({sx}, {sy}) leaves the screen"
        );
        assert_eq!(plan.pick(p.x, p.y), i as i32, "{ctx}: pick at cell {i}");
    }
    // The centre is no action.
    assert_eq!(
        plan.pick(plan.layout.origin.0, plan.layout.origin.1),
        -1,
        "{ctx}: centre"
    );

    // The subsurface: bounding box of the sized rings, each hexagon plus PAD.
    let (mx, my) = (half_w() + PAD, radius() + PAD);
    let bw = e.bounds.2 - e.bounds.0 + 2.0 * mx;
    let bh = e.bounds.3 - e.bounds.1 + 2.0 * my;
    assert!(
        (plan.width - bw).abs() <= EPS && (plan.height - bh).abs() <= EPS,
        "{ctx}: size {} x {}, want {bw} x {bh}",
        plan.width,
        plan.height
    );
    assert!(
        (plan.layout.origin.0 - (mx - e.bounds.0)).abs() <= EPS
            && (plan.layout.origin.1 - (my - e.bounds.1)).abs() <= EPS,
        "{ctx}: origin {:?}",
        plan.layout.origin
    );
    // The subsurface stays on screen (0.5 px of rounding).
    assert!(pl.x >= 0 && pl.y >= 0, "{ctx}: at ({}, {})", pl.x, pl.y);
    assert!(
        pl.x as f32 + plan.width <= w + 0.5 + EPS && pl.y as f32 + plan.height <= h + 0.5 + EPS,
        "{ctx}: box ends past the screen"
    );

    // Placement sizes.
    let dest = (plan.width.ceil() as u32, plan.height.ceil() as u32);
    assert_eq!(pl.destination, dest, "{ctx}: destination");
    let phys = |c: u32| (u64::from(c) * u64::from(scale120)).div_ceil(120) as u32;
    assert_eq!(pl.buffer, (phys(dest.0), phys(dest.1)), "{ctx}: buffer");
}

#[test]
fn ac3_max_rings_is_four() {
    assert_eq!(MAX_RINGS, 4);
}

#[test]
fn ac3_oracle_matches_layout_extent_where_every_slot_fits() {
    // Cross-check of the test's own box rule against `Layout::extent`.
    for (n, rings) in [(3, 1), (18, 2)] {
        let e = expect(cells(n), false, rings, (640.0, 400.0));
        let (mx, my) = (half_w() + PAD, radius() + PAD);
        let (ew, eh) = Layout::extent(radius(), HEX_GAP, rings, PAD);
        assert!((e.bounds.2 - e.bounds.0 + 2.0 * mx - ew).abs() <= EPS);
        assert!((e.bounds.3 - e.bounds.1 + 2.0 * my - eh).abs() <= EPS);
    }
}

#[test]
fn ac3_no_anchor_is_plan_rings_and_place() {
    for n in COUNTS {
        for rtl in [false, true] {
            for min_rings in [1, 2] {
                for scale in [120, 180] {
                    let ctx = format!("{n} cells, rtl {rtl}, min_rings {min_rings}, scale {scale}");
                    let (plan, pl) = plan_at(cells(n), rtl, min_rings, SCREEN, None, scale);
                    let want = plan_rings(cells(n), rtl, min_rings);
                    assert_eq!(plan.cells, want.cells, "{ctx}: cells");
                    assert_eq!(
                        (plan.width, plan.height),
                        (want.width, want.height),
                        "{ctx}: size"
                    );
                    assert_eq!(plan.layout.origin, want.layout.origin, "{ctx}: origin");
                    let size = (want.width.ceil() as u32, want.height.ceil() as u32);
                    let expected: Placement = place(SCREEN, size, scale);
                    assert_eq!(pl, expected, "{ctx}: placement");
                }
            }
        }
    }
}

#[test]
fn ac3_anchor_inside_is_not_moved_by_the_clamp() {
    // (400, 500) needs no clamp: the centre lands on it.
    let (plan, pl) = plan_at(cells(18), false, 1, SCREEN, Some((400.0, 500.0)), 120);
    let ox = pl.x as f32 + plan.layout.origin.0;
    let oy = pl.y as f32 + plan.layout.origin.1;
    assert!(
        (ox - 400.0).abs() <= 0.5 && (oy - 500.0).abs() <= 0.5,
        "({ox}, {oy})"
    );
}

#[test]
fn ac3_corner_anchor_is_clamped_to_centre_plus_pad() {
    let (plan, pl) = plan_at(cells(18), false, 1, SCREEN, Some((0.0, 0.0)), 120);
    let ox = pl.x as f32 + plan.layout.origin.0;
    let oy = pl.y as f32 + plan.layout.origin.1;
    assert!((ox - (half_w() + PAD)).abs() <= 0.5 + EPS, "x {ox}");
    assert!((oy - (radius() + PAD)).abs() <= 0.5 + EPS, "y {oy}");
}

#[test]
fn ac3_short_single_ring_spreads_over_its_accepted_slots() {
    // At (640, 0) ring 1 has four accepted slots; two cells take the first
    // and the third of them, not the first two.
    let e = expect(cells(2), false, 1, (640.0, 0.0));
    assert_eq!(
        e.accepted[0], 4,
        "oracle: accepted ring-1 slots at (640, 0)"
    );
    let (plan, _) = plan_at(cells(2), false, 1, SCREEN, Some((640.0, 0.0)), 120);
    assert_eq!(ids(&plan), e.slots);
    let acc: Vec<Axial> = ring(1)
        .into_iter()
        .filter(|a| {
            let o = offset(*a, false);
            let c = clamp((640.0, 0.0));
            accepted((c.0 + o.0, c.1 + o.1))
        })
        .collect();
    assert_eq!(
        plan.cells.iter().map(|p| p.axial).collect::<Vec<_>>(),
        vec![acc[0], acc[2]]
    );
}

#[test]
fn ac3_ring_one_overflow_goes_ahead_of_ring_two_cells() {
    // At the corner ring 1 holds two slots: a2 to a5 go outwards before b0.
    let (plan, _) = plan_at(cells(18), false, 1, SCREEN, Some((0.0, 0.0)), 120);
    let order: Vec<&str> = plan.cells.iter().map(|p| p.cell.id.as_str()).collect();
    assert_eq!(&order[..7], ["a0", "a1", "a2", "a3", "a4", "a5", "b0"]);
    assert!(plan.cells[..2].iter().all(|p| hex_distance(p.axial) == 1));
    assert!(plan.cells[2..].iter().all(|p| hex_distance(p.axial) >= 2));
}

#[test]
fn ac3_short_ring_one_keeps_its_empty_slots_at_an_edge() {
    // 12 cells: four ring-1 cells. At (640, 400) ring 1 has six slots, so two
    // stay empty and the ring-2 cells start in ring 2.
    let (plan, _) = plan_at(cells(12), false, 1, SCREEN, Some((640.0, 400.0)), 120);
    let inner = plan
        .cells
        .iter()
        .filter(|p| hex_distance(p.axial) == 1)
        .count();
    assert_eq!(inner, 4);
}

#[test]
fn ac4_every_case_at_scale_1() {
    for anchor in ANCHORS {
        for n in COUNTS {
            for rtl in [false, true] {
                for min_rings in [1, 2] {
                    check_case(n, rtl, min_rings, anchor, 120);
                }
            }
        }
    }
}

#[test]
fn ac4_every_case_at_scale_1_5() {
    for anchor in ANCHORS {
        for n in COUNTS {
            for rtl in [false, true] {
                for min_rings in [1, 2] {
                    check_case(n, rtl, min_rings, anchor, 180);
                }
            }
        }
    }
}

#[test]
fn ac4_scale_1_5_changes_neither_cells_nor_position() {
    for anchor in ANCHORS {
        for n in COUNTS {
            for rtl in [false, true] {
                let (a, pa) = plan_at(cells(n), rtl, 1, SCREEN, Some(anchor), 120);
                let (b, pb) = plan_at(cells(n), rtl, 1, SCREEN, Some(anchor), 180);
                let ctx = format!("{n} cells, rtl {rtl}, anchor {anchor:?}");
                assert_eq!(a.cells, b.cells, "{ctx}");
                assert_eq!((a.width, a.height), (b.width, b.height), "{ctx}");
                assert_eq!((pa.x, pa.y), (pb.x, pb.y), "{ctx}");
                assert_eq!(pa.destination, pb.destination, "{ctx}");
                let (w, h) = pa.destination;
                assert_eq!(
                    pb.buffer,
                    ((w * 3).div_ceil(2), (h * 3).div_ceil(2)),
                    "{ctx}: buffer ceil(w * 1.5) x ceil(h * 1.5)"
                );
            }
        }
    }
}

#[test]
fn ac4_everything_fits_equals_plan_rings() {
    for anchor in [(640.0, 400.0), (400.0, 500.0)] {
        for n in COUNTS {
            for rtl in [false, true] {
                for min_rings in [1, 2] {
                    let ctx =
                        format!("{n} cells, rtl {rtl}, min_rings {min_rings}, anchor {anchor:?}");
                    let (plan, pl) = plan_at(cells(n), rtl, min_rings, SCREEN, Some(anchor), 120);
                    let want = plan_rings(cells(n), rtl, min_rings);
                    assert_eq!(ids(&plan), ids(&want), "{ctx}: slots");
                    for (a, b) in plan.cells.iter().zip(&want.cells) {
                        assert!(
                            (a.x - b.x).abs() <= EPS && (a.y - b.y).abs() <= EPS,
                            "{ctx}: {} at ({}, {}), plan_rings ({}, {})",
                            a.cell.id,
                            a.x,
                            a.y,
                            b.x,
                            b.y
                        );
                    }
                    assert!(
                        (plan.width - want.width).abs() <= EPS
                            && (plan.height - want.height).abs() <= EPS,
                        "{ctx}: size"
                    );
                    let x = (anchor.0 - want.layout.origin.0).round() as i32;
                    let y = (anchor.1 - want.layout.origin.1).round() as i32;
                    assert_eq!((pl.x, pl.y), (x, y), "{ctx}: position");
                }
            }
        }
    }
}

#[test]
fn ac4_two_ring_hive_at_the_middle_is_444_219() {
    for n in [18, 12] {
        for rtl in [false, true] {
            for scale in [120, 180] {
                let (_, pl) = plan_at(cells(n), rtl, 1, SCREEN, Some((640.0, 400.0)), scale);
                assert_eq!(
                    (pl.x, pl.y),
                    (444, 219),
                    "{n} cells, rtl {rtl}, scale {scale}"
                );
                assert_eq!(pl.destination, (393, 362), "{n} cells, rtl {rtl}");
            }
        }
    }
    let (_, pl) = plan_at(cells(18), false, 1, SCREEN, Some((640.0, 400.0)), 180);
    assert_eq!(pl.buffer, (590, 543));
}

#[test]
fn ac4_two_ring_hive_at_400_500_is_204_319() {
    for n in COUNTS {
        for rtl in [false, true] {
            let min_rings = if n >= 12 { 1 } else { 2 };
            let (_, pl) = plan_at(cells(n), rtl, min_rings, SCREEN, Some((400.0, 500.0)), 120);
            assert_eq!((pl.x, pl.y), (204, 319), "{n} cells, rtl {rtl}");
        }
    }
}

#[test]
fn ac4_corner_shows_every_cell_it_can() {
    // At (0, 0) ring 1 keeps two slots and rings 2 to 4 keep 16: the oracle
    // fills 18, so a smaller `MAX_RINGS` shows fewer.
    let e = expect(cells(18), false, 1, (0.0, 0.0));
    assert_eq!(e.accepted[..3], [2, 4, 5]);
    let (plan, _) = plan_at(cells(18), false, 1, SCREEN, Some((0.0, 0.0)), 120);
    assert_eq!(plan.cells.len(), e.slots.len());
    assert!(plan.cells.iter().any(|p| hex_distance(p.axial) == 4));
}

#[test]
fn ac3_anchor_above_the_bottom_edge_keeps_only_slots_within_h_minus_my() {
    // Surviving mutant layout.rs:153:70 (`+` to `*` in the y upper bound of
    // the slot filter). A slot is kept only when its centre lies within
    // `(h - my) + EPS`, with `my = r + PAD`. At the bottom edge itself the
    // next row is far below that bound, so the AC4 anchors cannot tell
    // `(h - my) + EPS` from a bound a little under `h`. Anchored 70 to 110 px
    // above the bottom, the centre is not clamped and the row below it
    // (1.5 spacings lower) has its centre between `h - my` and `h`: its
    // hexagon leaves the screen, so those slots must be dropped.
    let (w, h) = (SCREEN.0 as f32, SCREEN.1 as f32);
    let my = radius() + PAD;
    let spacing = radius() + HEX_GAP / 3f32.sqrt();
    for y in [690.0, 700.0, 710.0, 720.0, 730.0] {
        for x in [100.0, 640.0, 1200.0] {
            let anchor = (x, y);
            // The case is the one the mutant gets wrong: the anchor needs no
            // clamp, and the row below lies between the bound and the edge.
            let below = y + 1.5 * spacing;
            assert!(
                y <= h - my && below > h - my + EPS && below < h,
                "case {anchor:?}"
            );
            for n in [18, 12] {
                for rtl in [false, true] {
                    for min_rings in [1, 2] {
                        check_case(n, rtl, min_rings, anchor, 120);
                    }
                    let (plan, pl) = plan_at(cells(n), rtl, 1, SCREEN, Some(anchor), 120);
                    for p in &plan.cells {
                        let sy = pl.y as f32 + p.y;
                        assert!(
                            sy <= h - my + 0.5 + EPS,
                            "{n} cells, rtl {rtl}, anchor {anchor:?}: slot centre y {sy} past h - my = {}",
                            h - my
                        );
                    }
                    // The kept rows stop at the anchor's row and the one
                    // above it reaches no further than the oracle's slots.
                    let e = expect(cells(n), rtl, 1, anchor);
                    assert!(
                        e.slots
                            .iter()
                            .all(|(_, a)| e.centre.1 + offset(*a, rtl).1 <= h - my + EPS),
                        "oracle {anchor:?}"
                    );
                    assert!(
                        plan.cells.len() <= e.slots.len()
                            && pl.x as f32 + plan.width <= w + 0.5 + EPS,
                        "{n} cells, rtl {rtl}, anchor {anchor:?}"
                    );
                }
            }
        }
    }
}
