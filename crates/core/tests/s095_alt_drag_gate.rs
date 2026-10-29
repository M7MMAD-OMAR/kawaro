//! S-095 (backlog/S-095-alt-drag-reliability.md), AC4, AC5, AC6: the hold
//! gate is bounded, never ends stuck, and every non show outcome has a
//! fixed log line and a doctor line.
//!
//! Contract (written by the tester from the acceptance criteria; do not edit):
//! - `Gate::new(hold_ms)` keeps its signature; its `hold_max_ms` is 10000.
//!   `Gate::with_max(hold_ms, hold_max_ms)` takes two `FnMut() -> u64 + Send
//!   + 'static` closures, both read at the press that starts the timer.
//! - `Gate::is_idle(&self) -> bool`: the phase is Idle.
//! - `Config::hold_max_ms`: an unsigned integer (`u64::from` takes it),
//!   default 10000, saved under the key `hold_max_ms`.
//! - `hold::Outcome` (Clone, Copy, Debug, PartialEq): `Tap`, `NoDrag`,
//!   `NoActionFile`, `Timeout`, `ReleaseLost`, `DaemonDown`;
//!   `Outcome::log_line(self) -> String` gives `hold: <reason>` with the
//!   reason in {tap, no-drag, no-action-file, timeout, release-lost,
//!   daemon-down}.
//! - `hold::doctor_line(key: &str, bind_present: bool, daemon_up: bool,
//!   last: Option<(Outcome, u64)>) -> String` gives
//!   `drop key: <key> bind <present|missing>, daemon <up|down>, last hold:
//!   <reason> <n> ms ago`; with `last` None the tail is `last hold: none`.
//! - The limit counts from the press for a gate still timing, so a press
//!   with no drag and no release is idle at press + hold_max_ms. For a
//!   shown hive with no release the gate is idle (and the tick answers
//!   `Effect::Release`, to close the hive) at the show + hold_max_ms.
//!   Before the limit minus 1 ms nothing changes.

use kawaro_core::config::Config;
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
fn ac4_press_with_no_release_is_active_at_limit_minus_1_ms_and_idle_at_the_limit() {
    let t0 = Instant::now();
    let mut g = gate();
    g.signal(press(), t0);
    // The threshold passes with the key still held: the gate shows once.
    assert!(matches!(g.tick(ms(t0, HOLD)), Some(Effect::Show { .. })));
    g.hive_closed();
    // A press whose hive never came: a second gate, never ticked until late.
    let mut g2 = gate();
    g2.signal(press(), t0);
    g2.tick(ms(t0, MAX - 1));
    assert!(!g2.is_idle(), "still active at the limit minus 1 ms");
    g2.tick(ms(t0, MAX));
    assert!(g2.is_idle(), "idle at the limit");
    assert_eq!(g2.live_timers(), 0);
}

#[test]
fn ac4_shown_hive_with_no_release_closes_at_the_limit_after_the_show() {
    let t0 = Instant::now();
    let mut g = gate();
    g.signal(press(), t0);
    assert!(matches!(g.tick(ms(t0, HOLD)), Some(Effect::Show { .. })));
    assert_eq!(g.tick(ms(t0, HOLD + MAX - 1)), None);
    assert!(!g.is_idle(), "still active at the limit minus 1 ms");
    assert_eq!(g.tick(ms(t0, HOLD + MAX)), Some(Effect::Release));
    assert!(g.is_idle(), "idle at the limit");
    assert_eq!(g.live_timers(), 0);
}

#[test]
fn ac4_a_second_press_works_after_either_limit() {
    let t0 = Instant::now();
    // After the limit of a press with no drag.
    let mut g = gate();
    g.signal(press(), t0);
    g.tick(ms(t0, MAX));
    assert!(g.is_idle());
    g.signal(press(), ms(t0, MAX + 10));
    assert_eq!(g.live_timers(), 1, "the second press starts a timer");
    assert!(matches!(
        g.tick(ms(t0, MAX + 10 + HOLD)),
        Some(Effect::Show { .. })
    ));
    // After the limit of a shown hive.
    let mut h = gate();
    h.signal(press(), t0);
    h.tick(ms(t0, HOLD));
    h.tick(ms(t0, HOLD + MAX));
    assert!(h.is_idle());
    h.signal(press(), ms(t0, HOLD + MAX + 10));
    assert_eq!(h.live_timers(), 1);
}

#[test]
fn ac4_the_limit_is_hold_max_ms_from_the_config_default_10000() {
    assert_eq!(u64::from(Config::default().hold_max_ms), 10_000);
}

/// A small seeded generator (no dependency): xorshift64.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

#[test]
fn ac5_100_random_sequences_end_idle_with_no_timers() {
    for seed in 1..=100u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let t0 = Instant::now();
        let mut now = 0u64;
        let mut g = gate();
        for _ in 0..40 {
            now += rng.next() % 700;
            match rng.next() % 5 {
                0 => {
                    g.signal(press(), ms(t0, now));
                }
                1 => {
                    g.signal(Signal::Release, ms(t0, now));
                }
                2 => g.hive_closed(), // drag, drop, Esc: the hive closed
                _ => {
                    g.tick(ms(t0, now));
                }
            }
        }
        // No release ever comes after the last step: only the bounds close it.
        g.tick(ms(t0, now + HOLD + MAX + 1));
        g.tick(ms(t0, now + 2 * (HOLD + MAX) + 2));
        assert!(g.is_idle(), "seed {seed}: the gate must end Idle");
        assert_eq!(g.live_timers(), 0, "seed {seed}: no timer left");
    }
}

#[test]
fn ac6_every_outcome_writes_its_exact_log_line() {
    let want = [
        (Outcome::Tap, "hold: tap"),
        (Outcome::NoDrag, "hold: no-drag"),
        (Outcome::NoActionFile, "hold: no-action-file"),
        (Outcome::Timeout, "hold: timeout"),
        (Outcome::ReleaseLost, "hold: release-lost"),
        (Outcome::DaemonDown, "hold: daemon-down"),
    ];
    for (o, line) in want {
        assert_eq!(o.log_line(), line);
    }
}

#[test]
fn ac6_doctor_line_per_state() {
    assert_eq!(
        hold::doctor_line("ALT_L", true, true, Some((Outcome::Tap, 1200))),
        "drop key: ALT_L bind present, daemon up, last hold: tap 1200 ms ago"
    );
    assert_eq!(
        hold::doctor_line("ALT_L", false, true, Some((Outcome::NoDrag, 35))),
        "drop key: ALT_L bind missing, daemon up, last hold: no-drag 35 ms ago"
    );
    assert_eq!(
        hold::doctor_line("SUPER+Z", true, false, Some((Outcome::DaemonDown, 9000))),
        "drop key: SUPER+Z bind present, daemon down, last hold: daemon-down 9000 ms ago"
    );
    assert_eq!(
        hold::doctor_line("ALT_L", true, true, None),
        "drop key: ALT_L bind present, daemon up, last hold: none"
    );
}
