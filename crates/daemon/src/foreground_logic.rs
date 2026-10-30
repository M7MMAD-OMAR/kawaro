//! The pure part of giving a window the foreground (S-106 D1): the order of
//! the steps and when to stop. Std only, so a test can include this file by
//! path. The calls themselves are in `shell_open.rs`.

/// One way to bring a window forward, mildest first.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Step {
    /// A plain `SetForegroundWindow`.
    Plain,
    /// Restore, then the topmost toggle.
    Topmost,
    /// Flash the taskbar button; the user brings the window forward.
    Flash,
}

/// Tries the steps in order and stops at the first that returns true.
/// Returns the step that worked, or `Flash` last. At most three calls.
pub fn foreground_ladder(mut try_step: impl FnMut(Step) -> bool) -> Step {
    for step in [Step::Plain, Step::Topmost] {
        if try_step(step) {
            return step;
        }
    }
    try_step(Step::Flash);
    Step::Flash
}
