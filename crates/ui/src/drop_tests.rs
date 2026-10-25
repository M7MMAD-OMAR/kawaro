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
//!
//! S-019e (backlog/S-019-hive-at-pointer.md, AC12 as amended 2026-10-21, and
//! the manager's decision 3): `Event::Show` gains `hold: bool` (a drop show
//! made by `kawaro drop --hold` after its threshold, AC17) and a new
//! `Event::Release` (the key of that show was released). The S-014 literals
//! above take `hold: false`; their assertions are unchanged.
//!
//! S-021a (backlog/S-021-drop-only-on-drag.md, AC1 and AC9): on the layer
//! path a drop show checks for a real file drag before it is revealed.
//! `Event::Show` gains `layer: bool` (the show is on the layer path, a hidden
//! catcher); the literals above take `layer: false` and keep their
//! assertions (the window path). New in the contract: `State::Probing` (a
//! layer drop show bound but not revealed), `Event::PointerEnter` (a
//! `wl_pointer.enter` on the catcher: no drag) and `Event::Reject` (the drag
//! has no `text/uri-list` or its files have no action). `Event::Enter` stays
//! a unit variant and means an enter with one cell or more. `ENTER_TIMEOUT`
//! (10 s) is the cap of a catcher that gets no event.

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
    let s = m.handle(
        Event::Show {
            drop,
            hold: false,
            layer: false,
        },
        t0,
    );
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
        m.handle(
            Event::Show {
                drop: true,
                hold: false,
                layer: false,
            },
            at(t0, 9.5)
        ),
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

// S-019e, AC12 and the manager's decision 3: the key release in a hold show.

fn shown_hold() -> (Machine, Instant) {
    let t0 = Instant::now();
    let mut m = Machine::new();
    let s = m.handle(
        Event::Show {
            drop: true,
            hold: true,
            layer: false,
        },
        t0,
    );
    assert_eq!(s, State::Waiting);
    (m, t0)
}

#[test]
fn ac12_release_hides_a_waiting_hold_show() {
    let (mut m, t0) = shown_hold();
    assert_eq!(m.handle(Event::Release, at(t0, 0.5)), State::Hidden);
    let (mut m, t0) = shown_hold();
    assert_eq!(m.handle(Event::Enter, at(t0, 0.3)), State::Waiting);
    assert_eq!(m.handle(Event::Release, at(t0, 0.5)), State::Hidden);
}

#[test]
fn ac12_release_over_a_cell_hides_and_runs_nothing() {
    let (mut m, t0) = shown_hold();
    m.handle(Event::Enter, at(t0, 0.3));
    assert_eq!(
        m.handle(Event::Motion(Some(2)), at(t0, 0.4)),
        State::Over(2)
    );
    assert_eq!(m.handle(Event::Release, at(t0, 0.5)), State::Hidden);
    // A drop that arrives after it changes nothing (no Running).
    assert_eq!(m.handle(Event::Drop, at(t0, 0.6)), State::Hidden);
}

#[test]
fn ac12_release_after_the_drop_has_no_effect() {
    let (mut m, t0) = shown_hold();
    m.handle(Event::Enter, at(t0, 0.3));
    m.handle(Event::Motion(Some(1)), at(t0, 0.4));
    assert_eq!(m.handle(Event::Drop, at(t0, 0.5)), State::Running(1));
    assert_eq!(m.handle(Event::Release, at(t0, 0.6)), State::Running(1));
}

#[test]
fn ac12_a_show_without_hold_ignores_release() {
    // The plain drop (a key press binding, `SUPER+Z`) and the normal hive.
    let (mut m, t0) = shown(true);
    assert_eq!(m.handle(Event::Release, at(t0, 0.5)), State::Waiting);
    m.handle(Event::Enter, at(t0, 0.6));
    m.handle(Event::Motion(Some(3)), at(t0, 0.7));
    assert_eq!(m.handle(Event::Release, at(t0, 0.8)), State::Over(3));
    let (mut m, t0) = shown(false);
    assert_eq!(m.handle(Event::Release, at(t0, 0.5)), State::Normal);
}

#[test]
fn ac12_the_machine_keeps_no_earlier_release() {
    // The 300 ms buffer of decision 3 is gone (amendment of 2026-10-21): a
    // release while hidden is forgotten, the next hold show waits.
    let t0 = Instant::now();
    let mut m = Machine::new();
    assert_eq!(m.handle(Event::Release, t0), State::Hidden);
    let s = m.handle(
        Event::Show {
            drop: true,
            hold: true,
            layer: false,
        },
        at(t0, 0.05),
    );
    assert_eq!(s, State::Waiting);
    assert_eq!(m.handle(Event::Tick, at(t0, 0.1)), State::Waiting);
    assert_eq!(m.handle(Event::Tick, at(t0, 0.5)), State::Waiting);
}

#[test]
fn ac12_the_10_s_enter_timeout_stays_in_a_hold_show_from_the_show() {
    let (mut m, t0) = shown_hold();
    assert_eq!(m.handle(Event::Tick, at(t0, 9.9)), State::Waiting);
    assert_eq!(m.handle(Event::Tick, at(t0, 10.1)), State::Hidden);
}

#[test]
fn ac12_decision3_leaving_a_hold_show_keeps_it_while_the_key_is_held() {
    // Manager decision 3: the drag leaving the hive does not close it while
    // the key is held; the drop or the key release closes it.
    let (mut m, t0) = shown_hold();
    m.handle(Event::Enter, at(t0, 0.3));
    m.handle(Event::Motion(Some(0)), at(t0, 0.4));
    assert_eq!(m.handle(Event::Leave, at(t0, 0.5)), State::Waiting);
    // No enter timeout after the first enter, even out of the hive.
    assert_eq!(m.handle(Event::Tick, at(t0, 12.0)), State::Waiting);
    // Back in and dropped on a cell: it runs.
    assert_eq!(m.handle(Event::Enter, at(t0, 12.5)), State::Waiting);
    assert_eq!(
        m.handle(Event::Motion(Some(4)), at(t0, 12.6)),
        State::Over(4)
    );
    assert_eq!(m.handle(Event::Drop, at(t0, 12.7)), State::Running(4));
    // Left, then the key released: closed.
    let (mut m, t0) = shown_hold();
    m.handle(Event::Enter, at(t0, 0.3));
    assert_eq!(m.handle(Event::Leave, at(t0, 0.5)), State::Waiting);
    assert_eq!(m.handle(Event::Release, at(t0, 0.9)), State::Hidden);
}

#[test]
fn ac12_decision3_leave_still_closes_a_show_without_hold() {
    let (mut m, t0) = entered();
    assert_eq!(m.handle(Event::Leave, at(t0, 2.0)), State::Hidden);
}

#[test]
fn ac12_a_cancelled_drag_and_escape_still_close_a_hold_show() {
    let (mut m, t0) = shown_hold();
    m.handle(Event::Enter, at(t0, 0.3));
    assert_eq!(m.handle(Event::Cancel, at(t0, 0.5)), State::Hidden);
    let (mut m, t0) = shown_hold();
    assert_eq!(m.handle(Event::Escape, at(t0, 0.5)), State::Hidden);
}

// ------------------------------------------------------------------------
// S-021a, AC1: the probe of a drop show on the layer path.

fn layer_show(hold: bool) -> (Machine, Instant) {
    let t0 = Instant::now();
    let mut m = Machine::new();
    let s = m.handle(
        Event::Show {
            drop: true,
            hold,
            layer: true,
        },
        t0,
    );
    assert_eq!(s, State::Probing, "a layer drop show probes, hold={hold}");
    assert_eq!(m.state(), State::Probing);
    (m, t0)
}

#[test]
fn ac1_a_layer_drop_show_probes_with_and_without_hold() {
    layer_show(false);
    layer_show(true);
}

#[test]
fn ac1_a_normal_show_on_the_layer_path_does_not_probe() {
    let mut m = Machine::new();
    let s = m.handle(
        Event::Show {
            drop: false,
            hold: false,
            layer: true,
        },
        Instant::now(),
    );
    assert_eq!(s, State::Normal);
}

#[test]
fn ac1_enter_with_cells_reveals_and_the_drag_then_runs_as_today() {
    for hold in [false, true] {
        let (mut m, t0) = layer_show(hold);
        assert_eq!(m.handle(Event::Enter, at(t0, 0.5)), State::Waiting);
        assert_eq!(
            m.handle(Event::Motion(Some(2)), at(t0, 0.6)),
            State::Over(2)
        );
        // After the enter the cap no longer applies.
        assert_eq!(m.handle(Event::Tick, at(t0, 10.1)), State::Over(2));
        assert_eq!(m.handle(Event::Drop, at(t0, 10.2)), State::Running(2));
    }
}

#[test]
fn ac1_reject_hides_a_probing_show() {
    for hold in [false, true] {
        let (mut m, t0) = layer_show(hold);
        assert_eq!(m.handle(Event::Reject, at(t0, 0.5)), State::Hidden);
    }
}

#[test]
fn ac1_pointer_enter_hides_a_probing_show() {
    for hold in [false, true] {
        let (mut m, t0) = layer_show(hold);
        assert_eq!(m.handle(Event::PointerEnter, at(t0, 0.3)), State::Hidden);
    }
}

#[test]
fn ac1_release_hides_a_probing_hold_show_only() {
    let (mut m, t0) = layer_show(true);
    assert_eq!(m.handle(Event::Release, at(t0, 0.5)), State::Hidden);
    // A show without hold (`kawaro drop`, `SUPER+Z`) ignores the release.
    let (mut m, t0) = layer_show(false);
    assert_eq!(m.handle(Event::Release, at(t0, 0.5)), State::Probing);
}

#[test]
fn ac1_escape_and_cancel_hide_a_probing_show() {
    for hold in [false, true] {
        let (mut m, t0) = layer_show(hold);
        assert_eq!(m.handle(Event::Escape, at(t0, 0.5)), State::Hidden);
        let (mut m, t0) = layer_show(hold);
        assert_eq!(m.handle(Event::Cancel, at(t0, 0.5)), State::Hidden);
    }
}

#[test]
fn ac1_the_cap_keeps_probing_at_9_9_seconds_and_hides_at_10_1() {
    for hold in [false, true] {
        let (mut m, t0) = layer_show(hold);
        assert_eq!(m.handle(Event::Tick, at(t0, 9.9)), State::Probing);
        assert_eq!(m.handle(Event::Tick, at(t0, 10.1)), State::Hidden);
    }
}

#[test]
fn ac1_focus_loss_and_click_outside_keep_probing() {
    let (mut m, t0) = layer_show(false);
    assert_eq!(m.handle(Event::FocusLost, at(t0, 0.2)), State::Probing);
    assert_eq!(m.handle(Event::ClickOutside, at(t0, 0.3)), State::Probing);
    // Still probing, so the cap still counts from the show.
    assert_eq!(m.handle(Event::Tick, at(t0, 10.1)), State::Hidden);
}

#[test]
fn ac1_the_window_path_still_waits_and_ignores_pointer_enter() {
    let (mut m, t0) = shown(true);
    assert_eq!(m.handle(Event::PointerEnter, at(t0, 0.3)), State::Waiting);
    assert_eq!(m.handle(Event::Enter, at(t0, 0.5)), State::Waiting);
    assert_eq!(m.handle(Event::PointerEnter, at(t0, 0.6)), State::Waiting);
    assert_eq!(
        m.handle(Event::Motion(Some(1)), at(t0, 0.7)),
        State::Over(1)
    );
    assert_eq!(m.handle(Event::PointerEnter, at(t0, 0.8)), State::Over(1));
    let (mut m, t0) = shown(false);
    assert_eq!(m.handle(Event::PointerEnter, at(t0, 0.3)), State::Normal);
}

// S-021a, AC9: shows in a row on one machine, each with its own cap.

#[test]
fn ac9_two_hides_without_a_drag_then_a_held_drag_reveals_and_runs() {
    let t0 = Instant::now();
    let show = |hold| Event::Show {
        drop: true,
        hold,
        layer: true,
    };
    let mut m = Machine::new();
    // 1: no drag, the pointer enters the catcher.
    assert_eq!(m.handle(show(false), t0), State::Probing);
    assert_eq!(m.handle(Event::PointerEnter, at(t0, 0.3)), State::Hidden);
    // 2: a hold show, the key released with the pointer still.
    assert_eq!(m.handle(show(true), at(t0, 1.0)), State::Probing);
    assert_eq!(m.handle(Event::Release, at(t0, 1.5)), State::Hidden);
    // A release after the hide changes nothing.
    assert_eq!(m.handle(Event::Release, at(t0, 1.6)), State::Hidden);
    // 3: a hold show with a PNG drag: revealed, then dropped on a cell.
    assert_eq!(m.handle(show(true), at(t0, 5.0)), State::Probing);
    // Its cap counts from its own show, not from the first one.
    assert_eq!(m.handle(Event::Tick, at(t0, 14.9)), State::Probing);
    assert_eq!(m.handle(Event::Enter, at(t0, 14.95)), State::Waiting);
    assert_eq!(
        m.handle(Event::Motion(Some(0)), at(t0, 15.0)),
        State::Over(0)
    );
    assert_eq!(m.handle(Event::Drop, at(t0, 15.1)), State::Running(0));
    assert_eq!(m.handle(Event::Release, at(t0, 15.2)), State::Running(0));
}
