//! S-102 (backlog/S-102-alt-doctor-and-docs.md), AC5: an Alt chord with
//! another key (Alt+a, Alt+Tab, Alt+F4) never shows a hive. Hyprland does not
//! fire the release bind when another key came between, so the gate must end
//! a pending press by itself when any other key goes down.
//!
//! Contract (written by the tester from the acceptance criteria; do not edit):
//! - `Signal::OtherKey`: a key other than the drop key went down (the daemon
//!   sends it for any other key signal while a press may be pending).
//! - Gate in Timing (press seen, hive not shown): `signal(OtherKey, now)`
//!   returns `None`, the gate is idle (`is_idle`), `live_timers()` is 0, a
//!   later `tick` returns `None` (no `Effect::Show`), and `take_outcome()`
//!   gives `Some(Outcome::Chord)` once.
//! - Gate Idle: `OtherKey` changes nothing (no effect, still idle, no
//!   outcome).
//! - Gate with a shown hive (the threshold passed): `OtherKey` changes
//!   nothing; the hive stays until the release (Esc with Alt held and other
//!   keys over a shown hive must not close or re-arm it).
//! - A `Release` after the cancel gives no effect and the gate stays idle.
//! - A new press after the cancel behaves as a fresh press: it shows at its
//!   own press + hold_ms.
//! - `Outcome::Chord.log_line()` is `hold: chord`; `doctor_line` prints
//!   `last hold: chord <n> ms ago` for it.

use kawaro_core::hold::{self, Effect, Gate, Outcome, Signal};
use std::time::{Duration, Instant};

const HOLD: u64 = 400;
const MAX: u64 = 10_000;

fn ms(t0: Instant, n: u64) -> Instant {
    t0 + Duration::from_millis(n)
}

fn press() -> Signal {
    Signal::Hold { at: None }
}

fn gate() -> Gate {
    Gate::with_max(|| HOLD, || MAX)
}

#[test]
fn ac5_another_key_during_the_wait_ends_the_press_and_nothing_shows() {
    let t0 = Instant::now();
    let mut g = gate();
    g.signal(press(), t0);
    assert_eq!(g.signal(Signal::OtherKey, ms(t0, 120)), None);
    assert!(g.is_idle(), "the gate is idle after the chord key");
    assert_eq!(g.live_timers(), 0);
    assert_eq!(g.tick(ms(t0, HOLD)), None, "no show at hold_ms");
    assert_eq!(g.tick(ms(t0, HOLD + 5_000)), None, "no late show either");
    assert_eq!(g.take_outcome(), Some(Outcome::Chord));
    assert_eq!(g.take_outcome(), None, "the outcome is taken once");
}

#[test]
fn ac5_the_chord_key_just_before_the_threshold_still_cancels() {
    let t0 = Instant::now();
    let mut g = gate();
    g.signal(press(), t0);
    assert_eq!(g.signal(Signal::OtherKey, ms(t0, HOLD - 1)), None);
    assert!(g.is_idle());
    assert_eq!(g.tick(ms(t0, HOLD)), None);
}

#[test]
fn ac5_a_release_after_the_cancel_gives_no_effect_and_stays_idle() {
    let t0 = Instant::now();
    let mut g = gate();
    g.signal(press(), t0);
    g.signal(Signal::OtherKey, ms(t0, 100));
    let _ = g.take_outcome();
    assert_eq!(g.signal(Signal::Release, ms(t0, 900)), None);
    assert!(g.is_idle());
    assert_eq!(g.live_timers(), 0);
}

#[test]
fn ac5_a_new_press_after_the_cancel_shows_at_its_own_threshold() {
    let t0 = Instant::now();
    let mut g = gate();
    g.signal(press(), t0);
    g.signal(Signal::OtherKey, ms(t0, 100));
    let _ = g.take_outcome();
    g.signal(press(), ms(t0, 2_000));
    assert_eq!(g.tick(ms(t0, 2_000 + HOLD - 1)), None);
    assert!(matches!(
        g.tick(ms(t0, 2_000 + HOLD)),
        Some(Effect::Show { .. })
    ));
}

#[test]
fn ac5_another_key_on_an_idle_gate_changes_nothing() {
    let t0 = Instant::now();
    let mut g = gate();
    assert_eq!(g.signal(Signal::OtherKey, t0), None);
    assert!(g.is_idle());
    assert_eq!(g.live_timers(), 0);
    assert_eq!(g.take_outcome(), None);
}

#[test]
fn ac5_another_key_over_a_shown_hive_leaves_it_up_until_the_release() {
    let t0 = Instant::now();
    let mut g = gate();
    g.signal(press(), t0);
    assert!(matches!(g.tick(ms(t0, HOLD)), Some(Effect::Show { .. })));
    assert_eq!(g.signal(Signal::OtherKey, ms(t0, HOLD + 50)), None);
    assert!(!g.is_idle(), "the shown hive keeps the gate active");
    assert_eq!(g.tick(ms(t0, HOLD + 100)), None);
    assert_eq!(
        g.signal(Signal::Release, ms(t0, HOLD + 200)),
        Some(Effect::Release)
    );
}

#[test]
fn ac5_the_chord_outcome_has_a_log_line_and_a_doctor_line() {
    assert_eq!(Outcome::Chord.log_line(), "hold: chord");
    let line = hold::doctor_line("Alt", true, true, Some((Outcome::Chord, 12)));
    assert_eq!(
        line,
        "drop key: Alt bind present, daemon up, last hold: chord 12 ms ago"
    );
}
