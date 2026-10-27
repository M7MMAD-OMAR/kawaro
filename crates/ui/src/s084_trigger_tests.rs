//! S-084 (backlog/S-084-windows-drag-trigger.md) AC2 and AC3 with the
//! manager decisions D3, D7 and D8: `drop_hive_allowed` takes the trigger
//! state of the chosen mechanism instead of a left button flag, `Source`
//! grows the tray variant, and a menu invocation always shows the hive. The
//! chosen trigger is the right-button drag (D3), so the state callers pass is
//! the drag button; the swapped-primary-buttons mapping stays in the caller.
//! Pure logic, tested on Linux.

use crate::drop_track::{Source, drop_hive_allowed};

#[test]
fn ac3_the_right_drag_trigger_is_a_real_drag_only_while_it_is_down() {
    // The chosen trigger (D3): right button down during the drag. The rule
    // accepts it as a real drag, and shows nothing without it.
    assert!(drop_hive_allowed(Source::Shortcut, true));
    assert!(!drop_hive_allowed(Source::Shortcut, false));
}

#[test]
fn ac3_the_tray_trigger_is_the_hover() {
    // D7: the tray variant's trigger state is the hover over the tray icon
    // while a drag is in progress.
    assert!(drop_hive_allowed(Source::Tray, true));
    assert!(!drop_hive_allowed(Source::Tray, false));
}

#[test]
fn ac3_a_menu_invocation_always_shows_the_hive() {
    // A menu invocation is not a drag path at all: it shows the hive with
    // the trigger up and with it down.
    assert!(drop_hive_allowed(Source::Menu, true));
    assert!(drop_hive_allowed(Source::Menu, false));
}
