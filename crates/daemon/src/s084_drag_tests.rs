//! S-084 (backlog/S-084-windows-drag-trigger.md) AC2, AC3 and manager
//! decisions D3 and D8: the right-button drag trigger. A momentary press is
//! not a drag; the trigger moment is the drag start past the system drag
//! threshold (`SM_CXDRAG`, `SM_CYDRAG`), with no hold threshold at all:
//! `DragWatch::tick` takes no clock, so there is nothing to wait for. Pure
//! logic, tested on Linux; the Win32 side is `shell.rs`.

use crate::server::drop_refused;
use crate::shell_logic::{DragPoll, DragWatch, drag_button_vk, primary_button_vk};

/// The system drag threshold the tests move past (like `SM_CXDRAG` and
/// `SM_CYDRAG` on a default machine).
const THRESHOLD: (i32, i32) = (4, 4);

#[test]
fn ac3_a_momentary_right_press_never_shows_the_hive() {
    let mut w = DragWatch::default();
    assert_eq!(w.tick(true, (100, 100), THRESHOLD), DragPoll::Idle);
    assert_eq!(w.tick(false, (100, 100), THRESHOLD), DragPoll::PressOnly);
    // The report of a press that was never a drag is not repeated.
    assert_eq!(w.tick(false, (100, 100), THRESHOLD), DragPoll::Idle);
}

#[test]
fn ac3_movement_under_the_threshold_is_not_a_drag() {
    let mut w = DragWatch::default();
    assert_eq!(w.tick(true, (100, 100), THRESHOLD), DragPoll::Idle);
    assert_eq!(w.tick(true, (103, 100), THRESHOLD), DragPoll::Idle);
    assert_eq!(w.tick(true, (100, 103), THRESHOLD), DragPoll::Idle);
    assert_eq!(w.tick(false, (100, 103), THRESHOLD), DragPoll::PressOnly);
}

#[test]
fn ac3_the_drag_starts_past_the_threshold_and_reports_once() {
    let mut w = DragWatch::default();
    assert_eq!(w.tick(true, (100, 100), THRESHOLD), DragPoll::Idle);
    assert_eq!(w.tick(true, (104, 100), THRESHOLD), DragPoll::Started);
    assert_eq!(w.tick(true, (120, 100), THRESHOLD), DragPoll::Idle);
    assert_eq!(w.tick(true, (120, 130), THRESHOLD), DragPoll::Idle);
    assert_eq!(w.tick(false, (120, 130), THRESHOLD), DragPoll::Ended);
    assert_eq!(w.tick(false, (120, 130), THRESHOLD), DragPoll::Idle);
}

#[test]
fn ac3_vertical_movement_starts_the_drag_too() {
    let mut w = DragWatch::default();
    assert_eq!(w.tick(true, (100, 100), THRESHOLD), DragPoll::Idle);
    assert_eq!(w.tick(true, (100, 107), THRESHOLD), DragPoll::Started);
    assert_eq!(w.tick(false, (100, 107), THRESHOLD), DragPoll::Ended);
}

#[test]
fn ac3_the_watch_rearms_after_each_drag_and_each_press() {
    let mut w = DragWatch::default();
    // One drag.
    w.tick(true, (0, 0), THRESHOLD);
    assert_eq!(w.tick(true, (9, 9), THRESHOLD), DragPoll::Started);
    assert_eq!(w.tick(false, (9, 9), THRESHOLD), DragPoll::Ended);
    // One press that is not a drag.
    assert_eq!(w.tick(true, (0, 0), THRESHOLD), DragPoll::Idle);
    assert_eq!(w.tick(false, (0, 0), THRESHOLD), DragPoll::PressOnly);
    // A second drag works the same.
    assert_eq!(w.tick(true, (0, 0), THRESHOLD), DragPoll::Idle);
    assert_eq!(w.tick(true, (0, -5), THRESHOLD), DragPoll::Started);
    assert_eq!(w.tick(false, (0, -5), THRESHOLD), DragPoll::Ended);
}

#[test]
fn ac3_the_drag_button_is_the_secondary_button_with_the_swapped_mapping() {
    // The trigger is the button whose drag shows the shell's own
    // "Copy here / Move here / Create shortcuts here" menu: the secondary
    // button, VK_RBUTTON, or VK_LBUTTON when the buttons are swapped
    // (`SM_SWAPBUTTON`). The mapping stays in the caller, as today.
    assert_eq!(drag_button_vk(false), 0x02);
    assert_eq!(drag_button_vk(true), 0x01);
    for swapped in [false, true] {
        assert_eq!(
            drag_button_vk(swapped),
            primary_button_vk(!swapped),
            "swapped {swapped}: the drag button is the other button of the primary"
        );
    }
}

#[test]
fn ac2_the_s021_rule_accepts_the_right_drag_trigger_as_a_real_drag() {
    // The trigger state the caller passes is the chosen mechanism's: the
    // drag button down. Without it no drop request shows a hive.
    assert!(
        !drop_refused(true, true),
        "a drag in progress shows the hive"
    );
    assert!(drop_refused(true, false), "no drag, no hive");
    assert!(!drop_refused(false, false), "not a drop request");
}
