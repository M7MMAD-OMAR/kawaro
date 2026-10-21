//! Dropping dragged files on the hive (S-014): which cell a drop point hits,
//! and the hive mode state machine. Pure logic; the Wayland drop receiver
//! (S-014b) feeds it events and acts on the states it returns.

use crate::layout::Plan;
use std::time::{Duration, Instant};

/// How long a drop hive waits for the drag to enter it before closing.
pub const ENTER_TIMEOUT: Duration = Duration::from_secs(10);

/// The cell under a surface local drop point (`wl_fixed` as `f64`), with the
/// same hit test as a click. Transparent corners and the centre give `None`.
pub fn pick(plan: &Plan, x: f64, y: f64) -> Option<usize> {
    usize::try_from(plan.pick(x as f32, y as f32)).ok()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Hidden,
    /// Shown from the context menu or `sbarhive open`.
    Normal,
    /// Shown by `sbarhive drop`, no cell under the drag.
    Waiting,
    /// A drag hovers this cell.
    Over(usize),
    /// Files were dropped on this cell and its action runs.
    Running(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Show {
        drop: bool,
    },
    /// The drag entered the hive surface.
    Enter,
    /// The drag moved; the cell under it, if any.
    Motion(Option<usize>),
    /// The drag left the hive surface.
    Leave,
    Drop,
    /// The drag source cancelled the drag.
    Cancel,
    Escape,
    FocusLost,
    ClickOutside,
    /// Periodic clock tick, for the enter timeout.
    Tick,
}

#[derive(Debug)]
pub struct Machine {
    state: State,
    shown_at: Option<Instant>,
    entered: bool,
}

impl Default for Machine {
    fn default() -> Self {
        Self::new()
    }
}

impl Machine {
    pub fn new() -> Machine {
        Machine {
            state: State::Hidden,
            shown_at: None,
            entered: false,
        }
    }

    pub fn state(&self) -> State {
        self.state
    }

    /// Applies `event` at time `now` and returns the new state.
    pub fn handle(&mut self, event: Event, now: Instant) -> State {
        use State::*;
        self.state = match (self.state, event) {
            (_, Event::Show { drop }) => {
                self.shown_at = Some(now);
                self.entered = false;
                if drop { Waiting } else { Normal }
            }
            (Normal, Event::Escape | Event::FocusLost | Event::ClickOutside) => Hidden,
            (Waiting | Over(_), Event::Enter) => {
                self.entered = true;
                Waiting
            }
            (Waiting | Over(_), Event::Motion(Some(i))) => Over(i),
            (Waiting | Over(_), Event::Motion(None)) => Waiting,
            (Waiting | Over(_), Event::Leave) if self.entered => Hidden,
            (Over(i), Event::Drop) => Running(i),
            (Waiting, Event::Drop) => Hidden,
            (Waiting | Over(_), Event::Cancel | Event::Escape) => Hidden,
            (Waiting, Event::Tick) if !self.entered && self.timed_out(now) => Hidden,
            (s, _) => s,
        };
        self.state
    }

    fn timed_out(&self, now: Instant) -> bool {
        self.shown_at
            .is_some_and(|t| now.saturating_duration_since(t) >= ENTER_TIMEOUT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shown(drop: bool) -> (Machine, Instant) {
        let t0 = Instant::now();
        let mut m = Machine::default();
        m.handle(Event::Show { drop, hold: false }, t0);
        (m, t0)
    }

    #[test]
    fn leave_before_enter_keeps_the_drop_hive() {
        let (mut m, t0) = shown(true);
        assert_eq!(m.handle(Event::Leave, t0), State::Waiting);
    }

    #[test]
    fn click_outside_closes_the_normal_hive() {
        let (mut m, t0) = shown(false);
        assert_eq!(m.handle(Event::ClickOutside, t0), State::Hidden);
    }

    #[test]
    fn the_normal_hive_ignores_drag_events_and_the_timeout() {
        let (mut m, t0) = shown(false);
        for e in [
            Event::Enter,
            Event::Motion(Some(1)),
            Event::Leave,
            Event::Drop,
            Event::Cancel,
        ] {
            assert_eq!(m.handle(e, t0), State::Normal, "{e:?}");
        }
        assert_eq!(m.handle(Event::Tick, t0 + ENTER_TIMEOUT * 2), State::Normal);
    }

    #[test]
    fn the_timeout_closes_at_exactly_ten_seconds() {
        let (mut m, t0) = shown(true);
        assert_eq!(m.handle(Event::Tick, t0 + ENTER_TIMEOUT), State::Hidden);
    }

    #[test]
    fn hidden_and_running_ignore_everything_but_show() {
        let t0 = Instant::now();
        let mut m = Machine::new();
        for e in [Event::Enter, Event::Escape, Event::Tick, Event::Drop] {
            assert_eq!(m.handle(e, t0), State::Hidden, "{e:?}");
        }
        m.handle(
            Event::Show {
                drop: true,
                hold: false,
            },
            t0,
        );
        m.handle(Event::Enter, t0);
        m.handle(Event::Motion(Some(3)), t0);
        assert_eq!(m.handle(Event::Drop, t0), State::Running(3));
        for e in [Event::Leave, Event::Escape, Event::Cancel, Event::FocusLost] {
            assert_eq!(m.handle(e, t0), State::Running(3), "{e:?}");
        }
        assert_eq!(
            m.handle(
                Event::Show {
                    drop: false,
                    hold: false,
                },
                t0
            ),
            State::Normal
        );
    }

    #[test]
    fn enter_over_a_cell_goes_back_to_waiting() {
        let (mut m, t0) = shown(true);
        m.handle(Event::Motion(Some(1)), t0);
        assert_eq!(m.handle(Event::Enter, t0), State::Waiting);
    }
}
