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

    /// The shown hive closed with no drag while the key is still held (the
    /// pointer moved before the drag began, S-095 AC2): the gate shows it
    /// again at `now`, so a drag that starts later still finds a hive. The
    /// original limit stays. Returns false when no hive was shown.
    pub fn rearm(&mut self, now: Instant) -> bool {
        if let Phase::Shown { limit, at } = self.phase {
            self.phase = Phase::Timing {
                due: now,
                limit,
                at,
            };
            self.outcome = Some(Outcome::NoDrag);
            true
        } else {
            false
        }
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
