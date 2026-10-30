//! S-105 (backlog/S-105-hyprland-0.57-no-at-focus.md) AC1 and AC2: the pure
//! decision to remap a drop show's catcher at the map time enter position.
//!
//! Hooks these tests pin (in `layer.rs`):
//! - `pub(crate) fn remap_at(drop: bool, had_at: bool, revealed: bool,
//!   remapped: bool, map_enter: Option<(f64, f64)>, origin: (i32, i32))
//!   -> Option<(f64, f64)>`: `Some(enter + origin)` only for a drop show with
//!   no `--at`, not revealed, not yet remapped, with a map time enter.
//! - `ProbeState`, `Probe`, `ProbeState::new`-free use through `Default`,
//!   and `ProbeState::{enter, motion, leave}` become `pub(crate)` so this
//!   file can drive them; `Probe` keeps `Debug` and `PartialEq`.

use crate::layer::{Probe, ProbeState, remap_at};
use std::time::Duration;

const ORIGIN: (i32, i32) = (1920, 0);
const ENTER: Option<(f64, f64)> = Some((40.0, 40.0));

#[test]
fn ac1_a_drop_show_without_at_remaps_at_the_enter_plus_the_origin() {
    assert_eq!(
        remap_at(true, false, false, false, ENTER, ORIGIN),
        Some((1960.0, 40.0))
    );
    assert_eq!(
        remap_at(true, false, false, false, ENTER, (0, 0)),
        Some((40.0, 40.0))
    );
    assert_eq!(
        remap_at(true, false, false, false, Some((5.5, 6.25)), (-1920, 10)),
        Some((-1914.5, 16.25))
    );
}

#[test]
fn ac1_every_other_input_gives_none() {
    // not a drop show
    assert_eq!(remap_at(false, false, false, false, ENTER, ORIGIN), None);
    // a show with --at
    assert_eq!(remap_at(true, true, false, false, ENTER, ORIGIN), None);
    // already revealed
    assert_eq!(remap_at(true, false, true, false, ENTER, ORIGIN), None);
    // already remapped
    assert_eq!(remap_at(true, false, false, true, ENTER, ORIGIN), None);
    // no map time enter
    assert_eq!(remap_at(true, false, false, false, None, ORIGIN), None);
}

#[test]
fn ac1_the_full_table_has_one_some_in_thirty_two_combinations() {
    let mut somes = 0;
    for bits in 0..32u32 {
        let (drop, had_at, revealed, remapped, enter) = (
            bits & 1 != 0,
            bits & 2 != 0,
            bits & 4 != 0,
            bits & 8 != 0,
            bits & 16 != 0,
        );
        let r = remap_at(
            drop,
            had_at,
            revealed,
            remapped,
            enter.then_some((1.0, 2.0)),
            ORIGIN,
        );
        if r.is_some() {
            somes += 1;
            assert!(drop && !had_at && !revealed && !remapped && enter);
        }
    }
    assert_eq!(somes, 1);
}

/// What the adapter does with the probe events of one show: a remap on a map
/// time enter, at most once.
fn drive(probes: &[Probe], remapped: &mut bool) -> Option<(f64, f64)> {
    let mut out = None;
    for p in probes {
        let enter = match p {
            Probe::MapEnter(x, y) => Some((*x, *y)),
            _ => None,
        };
        if let Some(pt) = remap_at(true, false, false, *remapped, enter, ORIGIN) {
            *remapped = true;
            out = Some(pt);
        }
    }
    out
}

#[test]
fn ac2_enter_at_zero_remaps_once_and_a_second_enter_does_not() {
    let mut st = ProbeState::default();
    let mut remapped = false;
    let first = st.enter(Some(Duration::from_millis(0)), (40.0, 40.0));
    assert_eq!(drive(&first, &mut remapped), Some((1960.0, 40.0)));
    assert!(remapped);
    // A forced second map time enter on the same show returns None.
    assert_eq!(remap_at(true, false, false, remapped, ENTER, ORIGIN), None);
    // And a second enter after a leave is a real pointer, never a remap.
    st.leave();
    let second = st.enter(Some(Duration::from_millis(10)), (40.0, 40.0));
    assert_eq!(drive(&second, &mut remapped), None);
}

#[test]
fn ac2_an_enter_later_than_250_ms_is_a_pointer_enter_and_no_remap() {
    let mut st = ProbeState::default();
    let mut remapped = false;
    let late = st.enter(Some(Duration::from_millis(300)), (40.0, 40.0));
    assert_eq!(late[0], Probe::PointerEnter);
    assert_eq!(drive(&late, &mut remapped), None);
    assert!(!remapped);
}

#[test]
fn ac2_a_drag_enter_with_no_pointer_enter_never_remaps() {
    // A drag holds the pointer: the catcher sees motion and positions only,
    // no wl_pointer.enter, so no map time enter exists.
    let mut st = ProbeState::default();
    let mut remapped = false;
    for pos in [(10.0, 10.0), (60.0, 60.0), (200.0, 90.0)] {
        assert_eq!(drive(&st.motion(pos), &mut remapped), None);
    }
    assert!(!remapped);
}

#[test]
fn ac2_a_move_after_the_remap_does_not_remap_again() {
    let mut st = ProbeState::default();
    let mut remapped = false;
    let first = st.enter(Some(Duration::from_millis(0)), (40.0, 40.0));
    assert!(drive(&first, &mut remapped).is_some());
    assert_eq!(drive(&st.motion((60.0, 40.0)), &mut remapped), None);
}
