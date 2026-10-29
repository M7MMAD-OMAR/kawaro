//! The long press gate (S-019e): the drop hive shows when a held key has
//! stayed down for `hold_ms`, not at the press. Pure, with an injected
//! clock: every call takes `now`, the gate never reads the time.
//!
//! Two inputs, a press ([`Signal::Hold`]) and a release
//! ([`Signal::Release`]), from any source: `kawaro drop --hold` and
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

/// Why a hold ended without a hive (S-095 AC6). Each has one log line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Outcome {
    /// The key came up before `hold_ms`.
    Tap,
    /// The hive showed and closed with no drag.
    NoDrag,
    /// A drag came with no file that has an action.
    NoActionFile,
    /// The press had no show within `hold_max_ms`.
    Timeout,
    /// A shown hive got no release within `hold_max_ms`.
    ReleaseLost,
    /// The press never reached a running daemon.
    DaemonDown,
}

impl Outcome {
    fn reason(self) -> &'static str {
        match self {
            Outcome::Tap => "tap",
            Outcome::NoDrag => "no-drag",
            Outcome::NoActionFile => "no-action-file",
            Outcome::Timeout => "timeout",
            Outcome::ReleaseLost => "release-lost",
            Outcome::DaemonDown => "daemon-down",
        }
    }

    /// `hold: <reason>`.
    pub fn log_line(self) -> String {
        format!("hold: {}", self.reason())
    }
}

/// The `kawaro doctor` line for the drop key (S-095 AC6).
pub fn doctor_line(
    key: &str,
    bind_present: bool,
    daemon_up: bool,
    last: Option<(Outcome, u64)>,
) -> String {
    let last = match last {
        Some((o, n)) => format!("{} {n} ms ago", o.reason()),
        None => "none".to_string(),
    };
    format!(
        "drop key: {key} bind {}, daemon {}, last hold: {last}",
        if bind_present { "present" } else { "missing" },
        if daemon_up { "up" } else { "down" },
    )
}

/// Why a shown hive closed without a key release (S-095).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hide {
    /// The pointer moved before any drag: a late drag may still come.
    Pointer,
    /// Anything else (Esc, the enter cap, a click): the hive stays closed.
    Other,
}

/// Most re-shows of one press, so continuous pointer motion with the key
/// held (an Alt window move) makes at most `1 + REARM_MAX` shows.
pub const REARM_MAX: u32 = 4;

/// A tick later than this after the threshold is a stalled clock, not a
/// hold: nothing shows from it.
const STALE: Duration = Duration::from_millis(1000);

#[derive(Clone, Copy, Debug)]
enum Phase {
    Idle,
    Timing {
        due: Instant,
        limit: Instant,
        at: Option<(f64, f64)>,
    },
    Shown {
        limit: Instant,
        at: Option<(f64, f64)>,
    },
}

/// The gate. `hold_ms` is called at every press that starts a timer, so a
/// changed setting applies at the next press.
pub struct Gate {
    hold_ms: Box<dyn FnMut() -> u64 + Send>,
    hold_max_ms: Box<dyn FnMut() -> u64 + Send>,
    phase: Phase,
    outcome: Option<Outcome>,
    rearms: u32,
}

impl Gate {
    pub fn new(hold_ms: impl FnMut() -> u64 + Send + 'static) -> Gate {
        Gate::with_max(hold_ms, || 10_000)
    }

    /// A gate whose every wait ends `hold_max_ms` after it began (AC4).
    pub fn with_max(
        hold_ms: impl FnMut() -> u64 + Send + 'static,
        hold_max_ms: impl FnMut() -> u64 + Send + 'static,
    ) -> Gate {
        Gate {
            hold_ms: Box::new(hold_ms),
            hold_max_ms: Box::new(hold_max_ms),
            phase: Phase::Idle,
            outcome: None,
            rearms: 0,
        }
    }

    /// The phase is Idle.
    pub fn is_idle(&self) -> bool {
        matches!(self.phase, Phase::Idle)
    }

    /// The outcome of the last hold that ended without a hive, once.
    pub fn take_outcome(&mut self) -> Option<Outcome> {
        self.outcome.take()
    }

    /// The single entry point for both inputs. Never returns
    /// [`Effect::Show`]: the hive shows from [`Gate::tick`] only.
    pub fn signal(&mut self, s: Signal, now: Instant) -> Option<Effect> {
        match (self.phase, s) {
            (Phase::Idle, Signal::Hold { at }) => {
                let due = now + Duration::from_millis((self.hold_ms)());
                let limit = now + Duration::from_millis((self.hold_max_ms)());
                self.phase = Phase::Timing { due, limit, at };
                self.rearms = 0;
                None
            }
            (Phase::Timing { .. }, Signal::Release) => {
                self.phase = Phase::Idle;
                self.outcome = Some(Outcome::Tap);
                None
            }
            (Phase::Shown { .. }, Signal::Release) => {
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
            Phase::Timing { limit, .. } if now >= limit => {
                self.phase = Phase::Idle;
                self.outcome = Some(Outcome::Timeout);
                None
            }
            Phase::Timing { due, at, .. } if now >= due && now - due <= STALE => {
                let limit = now + Duration::from_millis((self.hold_max_ms)());
                self.phase = Phase::Shown { limit, at };
                Some(Effect::Show { at })
            }
            Phase::Shown { limit, .. } if now >= limit => {
                self.phase = Phase::Idle;
                self.outcome = Some(Outcome::ReleaseLost);
                Some(Effect::Release)
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
        if matches!(self.phase, Phase::Shown { .. }) {
            self.phase = Phase::Idle;
        }
    }

    /// The shown hive closed with no drag while the key may still be held
    /// (S-095 AC2). Only a [`Hide::Pointer`] close shows it again at `now`,
    /// so a drag that starts later finds a hive, at most [`REARM_MAX`] times
    /// per press; the original limit stays. Any other close, or the last
    /// re-show used, ends the hold with [`Outcome::NoDrag`] for a pointer
    /// close. Returns true when the hive is to be shown again.
    pub fn rearm(&mut self, why: Hide, now: Instant) -> bool {
        let Phase::Shown { limit, at } = self.phase else {
            return false;
        };
        if why == Hide::Pointer && self.rearms < REARM_MAX {
            self.rearms += 1;
            self.phase = Phase::Timing {
                due: now,
                limit,
                at,
            };
            return true;
        }
        if why == Hide::Pointer {
            self.outcome = Some(Outcome::NoDrag);
        }
        false
    }
}

/// The IPC side: `kawaro drop --hold` is a press, `drop --release` a
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

#[cfg(test)]
mod rearm_tests {
    use super::*;

    fn shown() -> (Gate, Instant) {
        let t0 = Instant::now();
        let mut g = Gate::new(|| 400);
        g.signal(Signal::Hold { at: None }, t0);
        g.tick(t0 + Duration::from_millis(400));
        (g, t0 + Duration::from_millis(400))
    }

    #[test]
    fn a_pointer_close_shows_again_and_logs_nothing() {
        let (mut g, t) = shown();
        assert!(g.rearm(Hide::Pointer, t));
        assert_eq!(g.take_outcome(), None);
        assert!(matches!(g.tick(t), Some(Effect::Show { .. })));
    }

    #[test]
    fn any_other_close_stays_closed() {
        let (mut g, t) = shown();
        assert!(!g.rearm(Hide::Other, t));
        assert_eq!(g.take_outcome(), None);
        assert_eq!(g.tick(t), None);
    }

    #[test]
    fn re_shows_per_press_are_bounded_then_no_drag_is_logged() {
        let (mut g, mut t) = shown();
        for _ in 0..REARM_MAX {
            assert!(g.rearm(Hide::Pointer, t));
            assert!(matches!(g.tick(t), Some(Effect::Show { .. })));
            t += Duration::from_millis(10);
        }
        assert!(!g.rearm(Hide::Pointer, t));
        assert_eq!(g.take_outcome(), Some(Outcome::NoDrag));
    }

    #[test]
    fn a_new_press_resets_the_count() {
        let (mut g, t) = shown();
        for _ in 0..REARM_MAX {
            g.rearm(Hide::Pointer, t);
            g.tick(t);
        }
        g.signal(Signal::Release, t);
        g.signal(Signal::Hold { at: None }, t);
        g.tick(t + Duration::from_millis(400));
        assert!(g.rearm(Hide::Pointer, t));
    }
}
