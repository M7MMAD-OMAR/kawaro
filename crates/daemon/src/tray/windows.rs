//! The Windows tray stub (S-045 AC4): it compiles and does nothing. The
//! notification area icon comes in S-046. Left out of the mutants run.

use crate::state::Shared;
use sbarhive_core::i18n::Lang;
use sbarhive_core::queue::Queue;
use std::sync::Arc;

/// No icon yet, so there is never a handle.
pub type TrayHandle = Option<()>;

pub fn start(_lang: Lang, _queue: Arc<Queue>, _state: Shared) -> TrayHandle {
    None
}

pub fn set_active(_tray: &TrayHandle, _active: Vec<(String, f32)>) {}

pub fn set_lang(_lang: Lang) {}
