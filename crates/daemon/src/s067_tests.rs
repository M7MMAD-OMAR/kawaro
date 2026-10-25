//! S-067 AC5 (backlog/S-067-review-fixes-update-drop-notify.md): the pure
//! halves of the Windows shortcut drop gate, built and mutated on Linux.
//!
//! Contract (tester):
//! - `crate::shell_logic::primary_button_vk(swapped: bool) -> i32`: the
//!   virtual key of the primary mouse button, `0x01` (VK_LBUTTON) normally
//!   and `0x02` (VK_RBUTTON) when the buttons are swapped
//!   (`GetSystemMetrics(SM_SWAPBUTTON)` is read in `shell.rs`).
//! - `crate::server::drop_refused(is_drop: bool, primary_down: bool) ->
//!   bool`, not behind `cfg(windows)`: a drop request with the primary
//!   button up is refused; everything else passes.

use crate::server::drop_refused;
use crate::shell_logic::primary_button_vk;

#[test]
fn ac5_primary_button_vk_follows_the_swap_setting() {
    assert_eq!(primary_button_vk(false), 0x01, "VK_LBUTTON");
    assert_eq!(primary_button_vk(true), 0x02, "VK_RBUTTON");
}

#[test]
fn ac5_drop_refused_only_for_a_drop_without_the_primary_button() {
    assert!(drop_refused(true, false), "a shortcut drop with no drag");
    assert!(!drop_refused(true, true), "a drop during a drag");
    assert!(!drop_refused(false, false), "not a drop");
    assert!(!drop_refused(false, true), "not a drop, button held");
}
