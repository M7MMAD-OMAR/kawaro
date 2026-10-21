//! The long press gate (S-019e): the drop hive shows when a held key has
//! stayed down for `hold_ms`, not at the press. Pure, with an injected
//! clock: every call takes `now`, the gate never reads the time.
//!
//! Two inputs, a press ([`Signal::Hold`]) and a release
//! ([`Signal::Release`]), from any source: `sbarhive drop --hold` and
//! `--release` over IPC ([`from_request`]), or a GlobalShortcuts portal's
//! `Activated` and `Deactivated` ([`from_portal`]). One more, by manager
//! decision 14: [`Gate::hive_closed`], so a hive closed without its key
//! release (a drop, the 10 s timeout, Esc, a click) never swallows the next
//! press.

use crate::ipc::Request;
use std::time::{Duration, Instant};

/// A gate input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Signal {
    /// The key went down (or repeats). `at` is the pointer position of the
    /// press in global logical coordinates, when the caller knows it.
    Hold { at: Option<(f64, f64)> },
    /// The key went up.
    Release,
}

/// What the gate asks for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Effect {
    /// Show the drop hive in hold mode, at the first press's position.
    Show { at: Option<(f64, f64)> },
    /// Pass the key release to the hive this gate showed.
    Release,
}

#[derive(Clone, Copy, Debug)]
enum Phase {
    Idle,
    Timing {
        due: Instant,
        at: Option<(f64, f64)>,
    },
    Shown,
}

/// The gate. `hold_ms` is called at every press that starts a timer, so a
/// changed setting applies at the next press.
pub struct Gate {
    hold_ms: Box<dyn FnMut() -> u64 + Send>,
    phase: Phase,
}

impl Gate {
    pub fn new(hold_ms: impl FnMut() -> u64 + Send + 'static) -> Gate {
        Gate {
            hold_ms: Box::new(hold_ms),
            phase: Phase::Idle,
        }
    }

    /// The single entry point for both inputs. Never returns
    /// [`Effect::Show`]: the hive shows from [`Gate::tick`] only.
    pub fn signal(&mut self, s: Signal, now: Instant) -> Option<Effect> {
        match (self.phase, s) {
            (Phase::Idle, Signal::Hold { at }) => {
                let due = now + Duration::from_millis((self.hold_ms)());
                self.phase = Phase::Timing { due, at };
                None
            }
            (Phase::Timing { .. }, Signal::Release) => {
                self.phase = Phase::Idle;
                None
            }
            (Phase::Shown, Signal::Release) => {
                self.phase = Phase::Idle;
                Some(Effect::Release)
            }
            _ => None,
        }
    }

    /// The timer: the show, once, when the threshold has passed with the
    /// key still held.
    pub fn tick(&mut self, now: Instant) -> Option<Effect> {
        match self.phase {
            Phase::Timing { due, at } if now >= due => {
                self.phase = Phase::Shown;
                Some(Effect::Show { at })
            }
            _ => None,
        }
    }

    /// 1 while a threshold timer runs, else 0.
    pub fn live_timers(&self) -> usize {
        usize::from(matches!(self.phase, Phase::Timing { .. }))
    }

    /// Manager decision 14: the hive this gate showed has closed for any
    /// reason, so the gate forgets it and the next press starts a timer. A
    /// running threshold timer is left alone: it belongs to a newer press.
    pub fn hive_closed(&mut self) {
        if matches!(self.phase, Phase::Shown) {
            self.phase = Phase::Idle;
        }
    }
}

/// The IPC side: `sbarhive drop --hold` is a press, `drop --release` a
/// release. A plain `drop` is the immediate show, not the gate.
pub fn from_request(r: &Request) -> Option<Signal> {
    match r {
        Request::Drop { hold: true, at, .. } => Some(Signal::Hold { at: *at }),
        Request::DropRelease => Some(Signal::Release),
        _ => None,
    }
}

/// The GlobalShortcuts portal side (a later story): `Activated` is a press,
/// `Deactivated` a release.
pub fn from_portal(signal: &str) -> Option<Signal> {
    match signal {
        "Activated" => Some(Signal::Hold { at: None }),
        "Deactivated" => Some(Signal::Release),
        _ => None,
    }
}
