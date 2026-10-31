//! S-110 (backlog/S-110-open-at-position.md) AC5 and AC6: the pure decision
//! that places the hive at show time.
//!
//! Hooks these tests pin (in `layer.rs`, replacing the inline `at` test of
//! `wait_for_anchor` and the `drop_mode && anchor.is_none()` flags of
//! `decide`, so the maths stays in `layout::plan_at`):
//! - `pub(crate) enum Start { Place((f32, f32)), WaitForPointer, Window }`
//!   with `Debug`, `Clone`, `Copy`, `PartialEq`.
//! - `pub(crate) struct ShowPlan { pub start: Start, pub drop_probe: bool,
//!   pub hold: bool, pub may_move: bool }` with `Debug`, `Clone`, `Copy`,
//!   `PartialEq`.
//! - `pub(crate) fn show_plan(drop: bool, hold: bool, at: Option<(f64, f64)>,
//!   layer: bool) -> ShowPlan`:
//!   - layer path, `at = Some((x, y))`: `Start::Place((x as f32, y as f32))`
//!     at once (the anchor goes to `plan_at`, no centred first frame);
//!   - layer path, `at = None`: `Start::WaitForPointer` (the existing wait
//!     of `ANCHOR_WAIT`, then centred);
//!   - window path (`layer = false`): `Start::Window`, and `at` is ignored;
//!   - `drop_probe = drop && layer`, `hold = hold && drop`,
//!     `may_move = drop && layer && at.is_none()`.
//!
//! The S-105 `remap_at` results stay as they are (`s105_tests.rs`).

use crate::layer::{ShowPlan, Start, show_plan};

const AT: Option<(f64, f64)> = Some((400.0, 500.0));

fn plan(start: Start, drop_probe: bool, hold: bool, may_move: bool) -> ShowPlan {
    ShowPlan {
        start,
        drop_probe,
        hold,
        may_move,
    }
}

#[test]
fn ac5_a_normal_show_with_at_places_the_anchor_at_once_with_no_modes() {
    assert_eq!(
        show_plan(false, false, AT, true),
        plan(Start::Place((400.0, 500.0)), false, false, false)
    );
}

#[test]
fn ac5_a_normal_show_without_at_waits_for_the_pointer_with_no_modes() {
    assert_eq!(
        show_plan(false, false, None, true),
        plan(Start::WaitForPointer, false, false, false)
    );
}

#[test]
fn ac5_the_anchor_keeps_negative_and_fractional_values_as_f32() {
    assert_eq!(
        show_plan(false, false, Some((-1280.5, 300.25)), true).start,
        Start::Place((-1280.5, 300.25))
    );
}

#[test]
fn ac5_drop_shows_keep_today_s_behaviour() {
    assert_eq!(
        show_plan(true, false, AT, true),
        plan(Start::Place((400.0, 500.0)), true, false, false)
    );
    assert_eq!(
        show_plan(true, false, None, true),
        plan(Start::WaitForPointer, true, false, true)
    );
    assert_eq!(
        show_plan(true, true, AT, true),
        plan(Start::Place((400.0, 500.0)), true, true, false)
    );
    assert_eq!(
        show_plan(true, true, None, true),
        plan(Start::WaitForPointer, true, true, true)
    );
}

#[test]
fn ac5_a_normal_show_with_at_sets_no_probe_hold_or_later_move() {
    let p = show_plan(false, false, AT, true);
    assert!(!p.drop_probe && !p.hold && !p.may_move, "{p:?}");
}

#[test]
fn ac6_the_window_path_ignores_at_in_every_combination() {
    for drop in [false, true] {
        for hold in [false, true] {
            let without = show_plan(drop, hold, None, false);
            assert_eq!(without.start, Start::Window);
            assert!(!without.may_move && !without.drop_probe, "{without:?}");
            for at in [AT, Some((-1280.5, 300.25)), Some((0.0, 0.0))] {
                assert_eq!(show_plan(drop, hold, at, false), without);
            }
        }
    }
}

#[test]
fn ac5_the_full_table_has_exactly_one_place_for_each_layer_show_with_at() {
    let mut places = 0;
    for bits in 0..16u32 {
        let (drop, hold, has_at, layer) =
            (bits & 1 != 0, bits & 2 != 0, bits & 4 != 0, bits & 8 != 0);
        let p = show_plan(drop, hold, has_at.then_some((1.0, 2.0)), layer);
        let placed = matches!(p.start, Start::Place(_));
        assert_eq!(placed, has_at && layer, "{bits:04b}: {p:?}");
        places += usize::from(placed);
    }
    assert_eq!(places, 4);
}
