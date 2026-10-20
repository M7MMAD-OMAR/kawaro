//! S-014a: pure logic of dropping files on the hive (backlog/S-014-drop-hive.md).
//! AC2: picking a cell from a drop point. AC3: the hive mode state machine.
//!
//! Contract these tests fix, in `crate::dnd`:
//! - `pub fn pick(plan: &Plan, x: f64, y: f64) -> Option<usize>`: surface
//!   local drop point (wl_fixed as f64) to an index into `plan.cells`.
//! - `pub enum State { Hidden, Normal, Waiting, Over(usize), Running(usize) }`
//!   with `Clone, Copy, Debug, PartialEq, Eq`.
//! - `pub enum Event { Show { drop: bool }, Enter, Motion(Option<usize>),
//!   Leave, Drop, Cancel, Escape, FocusLost, ClickOutside, Tick }`.
//! - `pub struct Machine` with `new() -> Machine` (Hidden), `state() -> State`
//!   and `handle(&mut self, event: Event, now: Instant) -> State`. The clock
//!   is injected through `now`; the machine never reads the time itself.

use crate::HiveCell;
use crate::dnd::{self, Event, Machine, State};
use crate::layout::{Plan, plan};
use std::time::{Duration, Instant};

fn cell(id: &str, ring: u8) -> HiveCell {
    HiveCell {
        id: id.into(),
        code: id.to_uppercase(),
        label: format!("label {id}"),
        icon: String::new(),
        ring,
    }
}

/// A full two-ring hive (18 cells) and a one-ring hive of three cells, which
/// spreads its cells over alternating slots.
fn hives(rtl: bool) -> Vec<(&'static str, Plan)> {
    let mut full: Vec<HiveCell> = (0..6).map(|i| cell(&format!("a{i}"), 1)).collect();
    full.extend((0..12).map(|i| cell(&format!("b{i}"), 2)));
    let short = vec![cell("webp", 1), cell("png", 1), cell("pdf", 1)];
    vec![("full", plan(full, rtl)), ("short", plan(short, rtl))]
}

/// Click picking as the window does it: -1 means no action.
fn click(p: &Plan, x: f32, y: f32) -> Option<usize> {
    let i = p.pick(x, y);
    (i >= 0).then_some(i as usize)
}

/// Six points 1 px inside each edge of a pointy-top cell, on the edge normals.
fn inset_points(p: &Plan, cx: f32, cy: f32) -> Vec<(f32, f32)> {
    let apothem = p.layout.radius * 3f32.sqrt() / 2.0;
    (0..6)
        .map(|k| {
            let a = (k as f32 * 60.0).to_radians();
            (
                cx + (apothem - 1.0) * a.cos(),
                cy + (apothem - 1.0) * a.sin(),
            )
        })
        .collect()
}

fn ac2_check(rtl: bool) {
    for (name, p) in hives(rtl) {
        assert!(!p.cells.is_empty());
        for (i, c) in p.cells.iter().enumerate() {
            assert_eq!(
                dnd::pick(&p, c.x as f64, c.y as f64),
                Some(i),
                "{name} rtl={rtl}: centre of cell {i}"
            );
            assert_eq!(click(&p, c.x, c.y), Some(i));
            for (k, (x, y)) in inset_points(&p, c.x, c.y).into_iter().enumerate() {
                let drop = dnd::pick(&p, x as f64, y as f64);
                assert_eq!(drop, Some(i), "{name} rtl={rtl}: cell {i} edge {k}");
                assert_eq!(drop, click(&p, x, y), "{name} rtl={rtl}: same as click");
            }
        }
    }
}

#[test]
fn ac2_drop_pick_equals_click_pick_on_every_cell_in_english() {
    ac2_check(false);
}

#[test]
fn ac2_drop_pick_equals_click_pick_on_every_cell_in_mirrored_arabic() {
    ac2_check(true);
}

#[test]
fn ac2_drop_pick_follows_the_mirror() {
    // The first ring-1 cell is top right in English and top left in Arabic,
    // so the same window point picks it in one layout only.
    let ltr = plan(vec![cell("a", 1), cell("b", 2)], false);
    let rtl = plan(vec![cell("a", 1), cell("b", 2)], true);
    let (x, y) = (ltr.cells[0].x as f64, ltr.cells[0].y as f64);
    assert_eq!(dnd::pick(&ltr, x, y), Some(0));
    assert_ne!(dnd::pick(&rtl, x, y), Some(0));
    let (x, y) = (rtl.cells[0].x as f64, rtl.cells[0].y as f64);
    assert_eq!(dnd::pick(&rtl, x, y), Some(0));
}

#[test]
fn ac2_transparent_corners_and_the_centre_cell_give_no_action() {
    for rtl in [false, true] {
        for (name, p) in hives(rtl) {
            let (w, h) = (p.width as f64, p.height as f64);
            for (x, y) in [
                (0.0, 0.0),
                (w - 1.0, 0.0),
                (0.0, h - 1.0),
                (w - 1.0, h - 1.0),
            ] {
                assert_eq!(
                    dnd::pick(&p, x, y),
                    None,
                    "{name} rtl={rtl}: corner {x},{y}"
                );
            }
            let (ox, oy) = p.layout.origin;
            assert_eq!(
                dnd::pick(&p, ox as f64, oy as f64),
                None,
                "{name} rtl={rtl}: centre"
            );
            assert_eq!(click(&p, ox, oy), None);
        }
    }
}

// AC3: the mode state machine.

fn shown(drop: bool) -> (Machine, Instant) {
    let t0 = Instant::now();
    let mut m = Machine::new();
    assert_eq!(m.state(), State::Hidden);
    let s = m.handle(Event::Show { drop }, t0);
    assert_eq!(s, if drop { State::Waiting } else { State::Normal });
    assert_eq!(m.state(), s);
    (m, t0)
}

fn at(t0: Instant, secs: f64) -> Instant {
    t0 + Duration::from_secs_f64(secs)
}

/// Shown for a drop, then the drag entered at 1 s.
fn entered() -> (Machine, Instant) {
    let (mut m, t0) = shown(true);
    assert_eq!(m.handle(Event::Enter, at(t0, 1.0)), State::Waiting);
    (m, t0)
}

/// Shown for a drop, entered, and hovering cell 2.
fn over() -> (Machine, Instant) {
    let (mut m, t0) = entered();
    assert_eq!(
        m.handle(Event::Motion(Some(2)), at(t0, 1.5)),
        State::Over(2)
    );
    (m, t0)
}

#[test]
fn ac3_motion_moves_between_cells_and_back_to_waiting() {
    let (mut m, t0) = over();
    assert_eq!(
        m.handle(Event::Motion(Some(5)), at(t0, 2.0)),
        State::Over(5)
    );
    assert_eq!(m.handle(Event::Motion(None), at(t0, 2.1)), State::Waiting);
    assert_eq!(
        m.handle(Event::Motion(Some(0)), at(t0, 2.2)),
        State::Over(0)
    );
}

#[test]
fn ac3_drop_over_a_cell_runs_that_cell() {
    let (mut m, t0) = over();
    assert_eq!(m.handle(Event::Drop, at(t0, 2.0)), State::Running(2));
    assert_eq!(m.state(), State::Running(2));
}

#[test]
fn ac3_drop_outside_every_cell_closes() {
    let (mut m, t0) = entered();
    assert_eq!(m.handle(Event::Motion(None), at(t0, 1.5)), State::Waiting);
    assert_eq!(m.handle(Event::Drop, at(t0, 2.0)), State::Hidden);
}

#[test]
fn ac3_leave_after_enter_closes_while_waiting() {
    let (mut m, t0) = entered();
    assert_eq!(m.handle(Event::Leave, at(t0, 2.0)), State::Hidden);
}

#[test]
fn ac3_leave_closes_while_over_a_cell() {
    let (mut m, t0) = over();
    assert_eq!(m.handle(Event::Leave, at(t0, 2.0)), State::Hidden);
}

#[test]
fn ac3_cancelled_drag_closes_in_every_drop_state() {
    let (mut m, t0) = shown(true);
    assert_eq!(m.handle(Event::Cancel, at(t0, 0.5)), State::Hidden);
    let (mut m, t0) = entered();
    assert_eq!(m.handle(Event::Cancel, at(t0, 2.0)), State::Hidden);
    let (mut m, t0) = over();
    assert_eq!(m.handle(Event::Cancel, at(t0, 2.0)), State::Hidden);
}

#[test]
fn ac3_escape_closes_in_every_drop_state() {
    let (mut m, t0) = shown(true);
    assert_eq!(m.handle(Event::Escape, at(t0, 0.5)), State::Hidden);
    let (mut m, t0) = entered();
    assert_eq!(m.handle(Event::Escape, at(t0, 2.0)), State::Hidden);
    let (mut m, t0) = over();
    assert_eq!(m.handle(Event::Escape, at(t0, 2.0)), State::Hidden);
}

#[test]
fn ac3_without_enter_the_hive_stays_at_9_9_seconds_and_closes_at_10_1() {
    let (mut m, t0) = shown(true);
    assert_eq!(m.handle(Event::Tick, at(t0, 9.9)), State::Waiting);
    assert_eq!(m.handle(Event::Tick, at(t0, 10.1)), State::Hidden);
}

#[test]
fn ac3_the_timeout_does_not_apply_after_enter() {
    let (mut m, t0) = shown(true);
    assert_eq!(m.handle(Event::Enter, at(t0, 5.0)), State::Waiting);
    assert_eq!(m.handle(Event::Tick, at(t0, 10.1)), State::Waiting);
    assert_eq!(
        m.handle(Event::Motion(Some(1)), at(t0, 11.0)),
        State::Over(1)
    );
    assert_eq!(m.handle(Event::Tick, at(t0, 30.0)), State::Over(1));
}

#[test]
fn ac3_a_second_drop_show_restarts_the_timeout() {
    let (mut m, t0) = shown(true);
    assert_eq!(m.handle(Event::Tick, at(t0, 9.0)), State::Waiting);
    // Closed by Esc, then shown again at 9.5 s: its 10 s count from 9.5 s.
    assert_eq!(m.handle(Event::Escape, at(t0, 9.2)), State::Hidden);
    assert_eq!(
        m.handle(Event::Show { drop: true }, at(t0, 9.5)),
        State::Waiting
    );
    assert_eq!(m.handle(Event::Tick, at(t0, 19.4)), State::Waiting);
    assert_eq!(m.handle(Event::Tick, at(t0, 19.6)), State::Hidden);
}

#[test]
fn ac3_focus_loss_and_click_outside_keep_the_hive_before_enter() {
    let (mut m, t0) = shown(true);
    assert_eq!(m.handle(Event::FocusLost, at(t0, 0.2)), State::Waiting);
    assert_eq!(m.handle(Event::ClickOutside, at(t0, 0.3)), State::Waiting);
}

#[test]
fn ac3_focus_loss_and_click_outside_keep_the_hive_after_enter() {
    let (mut m, t0) = entered();
    assert_eq!(m.handle(Event::FocusLost, at(t0, 1.2)), State::Waiting);
    assert_eq!(m.handle(Event::ClickOutside, at(t0, 1.3)), State::Waiting);
}

#[test]
fn ac3_focus_loss_and_click_outside_keep_the_hive_over_a_cell() {
    let (mut m, t0) = over();
    assert_eq!(m.handle(Event::FocusLost, at(t0, 2.0)), State::Over(2));
    assert_eq!(m.handle(Event::ClickOutside, at(t0, 2.1)), State::Over(2));
}

#[test]
fn ac3_focus_loss_still_closes_the_normal_hive() {
    let (mut m, t0) = shown(false);
    assert_eq!(m.handle(Event::FocusLost, at(t0, 0.5)), State::Hidden);
}

#[test]
fn ac3_escape_closes_the_normal_hive() {
    let (mut m, t0) = shown(false);
    assert_eq!(m.handle(Event::Escape, at(t0, 0.5)), State::Hidden);
}
