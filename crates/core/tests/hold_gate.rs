//! S-019e (backlog/S-019-hive-at-pointer.md, AC17, AC19 and AC21, amended
//! 2026-10-21; manager decisions 9, 10 and 11 approved as proposed): the
//! long press gate. The hive appears when the hold threshold has passed with
//! the key still held, not at the press.
//!
//! Contract (written by the tester from the acceptance criteria; do not edit):
//! - The gate is pure and lives in `sbarhive_core::hold`, so the daemon (the
//!   service, AC17: not the UI), `hive_bench` (AC20, `measure.sh`) and a later
//!   GlobalShortcuts portal (AC21) share it. Its clock is injected: every call
//!   takes `now`, and it never reads the time itself.
//! - `pub enum Signal { Hold { at: Option<(f64, f64)> }, Release }`, with
//!   `Clone, Copy, Debug, PartialEq`: the only two inputs (AC17). `at` is the
//!   `--at X,Y` of the press (decision 1), in global logical coordinates.
//! - `pub enum Effect { Show { at: Option<(f64, f64)> }, Release }`, with
//!   `Clone, Copy, Debug, PartialEq`. `Show` asks for one drop show in hold
//!   mode (AC12) at the position of the first press (decision 9). `Release`
//!   passes a key release on to a hive this gate has shown and whose key has
//!   not been released yet (the hive then closes, AC12).
//! - `pub struct Gate`, `Gate::new(hold_ms)` where `hold_ms` is a closure
//!   `FnMut() -> u64` (`Send + 'static` is enough for these tests) that the
//!   gate calls at every press that starts a timer, so a changed `hold_ms` in
//!   `config.toml` applies at the next press without a restart (AC18). The
//!   daemon passes one that reads `Config::load()`.
//! - `Gate::signal(&mut self, s: Signal, now: Instant) -> Option<Effect>`:
//!   the single entry point for both inputs, from any source (AC21). It
//!   never returns `Effect::Show` (no show at a press or at a release, AC17).
//! - `Gate::tick(&mut self, now: Instant) -> Option<Effect>`: the timer.
//!   Returns `Effect::Show` once when `now` is at least `hold_ms` after the
//!   press that started the timer and no release came in between.
//! - `Gate::live_timers(&self) -> usize`: 1 while a threshold timer runs,
//!   else 0 (AC19).
//! - `hold::from_request(&Request) -> Option<Signal>`: the IPC side.
//!   `Request::Drop { hold: true, at, .. }` is `Hold { at }`,
//!   `Request::DropRelease` is `Release`, anything else (a plain
//!   `Request::Drop` included: an immediate show, S-014) is `None`.
//! - `hold::from_portal(signal: &str) -> Option<Signal>`: the GlobalShortcuts
//!   side (AC21, the portal itself is a later story). `"Activated"` is
//!   `Hold { at: None }`, `"Deactivated"` is `Release`, anything else `None`.
//! - The daemon may add more (a `deadline()` for its timer, say); these tests
//!   do not need it.

use sbarhive_core::config::Config;
use sbarhive_core::hold::{self, Effect, Gate, Signal};
use sbarhive_core::ipc::Request;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

fn ms(t0: Instant, n: u64) -> Instant {
    t0 + Duration::from_millis(n)
}

fn hold() -> Signal {
    Signal::Hold { at: None }
}

/// A gate with a fixed threshold.
fn gate(hold_ms: u64) -> Gate {
    Gate::new(move || hold_ms)
}

/// One step of a scenario: a signal at a time, in ms from the start.
#[derive(Clone, Copy, Debug)]
enum Step {
    Hold(u64),
    HoldAt(u64, f64, f64),
    Release(u64),
}

impl Step {
    fn time(self) -> u64 {
        match self {
            Step::Hold(t) | Step::HoldAt(t, _, _) | Step::Release(t) => t,
        }
    }
}

/// What a scenario produced: every effect with the ms it came at, and the
/// live timer count after each signal.
#[derive(Debug, Default, PartialEq)]
struct Trace {
    effects: Vec<(u64, Effect)>,
    timers_after: Vec<(u64, usize)>,
}

impl Trace {
    fn shows(&self) -> Vec<u64> {
        self.effects
            .iter()
            .filter(|(_, e)| matches!(e, Effect::Show { .. }))
            .map(|(t, _)| *t)
            .collect()
    }

    fn releases(&self) -> Vec<u64> {
        self.effects
            .iter()
            .filter(|(_, e)| *e == Effect::Release)
            .map(|(t, _)| *t)
            .collect()
    }
}

/// Runs the steps on `g`, ticking every millisecond from 0 to `until`, and
/// turns each step into a signal with `to_signal` (the source under test).
/// At a millisecond with a step, the signal goes first, then the tick.
fn run_with(g: &mut Gate, steps: &[Step], until: u64, to_signal: &dyn Fn(Step) -> Signal) -> Trace {
    let t0 = Instant::now();
    let mut trace = Trace::default();
    for now in 0..=until {
        for s in steps.iter().filter(|s| s.time() == now) {
            let effect = g.signal(to_signal(*s), ms(t0, now));
            assert!(
                !matches!(effect, Some(Effect::Show { .. })),
                "AC17: a signal never shows the hive itself ({s:?} gave {effect:?})"
            );
            if let Some(e) = effect {
                trace.effects.push((now, e));
            }
            trace.timers_after.push((now, g.live_timers()));
        }
        if let Some(e) = g.tick(ms(t0, now)) {
            assert!(
                matches!(e, Effect::Show { .. }),
                "only a show comes from the timer, got {e:?} at {now} ms"
            );
            trace.effects.push((now, e));
        }
    }
    trace
}

/// The IPC source's signal for a step: plain values, as AC17 states them.
fn direct(s: Step) -> Signal {
    match s {
        Step::Hold(_) => hold(),
        Step::HoldAt(_, x, y) => Signal::Hold { at: Some((x, y)) },
        Step::Release(_) => Signal::Release,
    }
}

fn run(steps: &[Step], until: u64) -> Trace {
    run_with(&mut gate(400), steps, until, &direct)
}

// AC17: the show at the threshold, not at the press.

#[test]
fn ac17_no_show_at_399_ms_and_one_show_at_400_ms() {
    let t0 = Instant::now();
    let mut g = gate(400);
    assert_eq!(g.live_timers(), 0);
    assert_eq!(g.signal(hold(), t0), None, "no show at the press");
    assert_eq!(g.live_timers(), 1, "the press starts one timer");
    assert_eq!(g.tick(ms(t0, 0)), None);
    assert_eq!(g.tick(ms(t0, 200)), None);
    assert_eq!(g.tick(ms(t0, 399)), None, "zero show requests at 399 ms");
    assert_eq!(
        g.tick(ms(t0, 400)),
        Some(Effect::Show { at: None }),
        "one show request at 400 ms"
    );
    assert_eq!(g.live_timers(), 0, "the timer is spent");
    assert_eq!(g.tick(ms(t0, 401)), None, "only once");
    assert_eq!(g.tick(ms(t0, 5_000)), None, "only once");
}

#[test]
fn ac17_a_late_tick_still_shows_once() {
    // The daemon's timer may wake late under load: one show, not zero.
    let t0 = Instant::now();
    let mut g = gate(400);
    g.signal(hold(), t0);
    assert_eq!(g.tick(ms(t0, 437)), Some(Effect::Show { at: None }));
    assert_eq!(g.tick(ms(t0, 900)), None);
}

#[test]
fn ac17_the_threshold_is_hold_ms() {
    for hold_ms in [100, 250, 400, 1000, 2000] {
        let t = run_with(&mut gate(hold_ms), &[Step::Hold(0)], 2500, &direct);
        assert_eq!(t.shows(), [hold_ms], "hold_ms {hold_ms}");
    }
}

#[test]
fn ac17_neither_the_press_nor_the_release_shows_the_hive() {
    // run_with asserts that no signal returns Show; a release well after
    // the show gives the release effect only.
    let t = run(&[Step::Hold(0), Step::Release(900)], 1500);
    assert_eq!(t.shows(), [400]);
    assert_eq!(t.releases(), [900]);
}

#[test]
fn ac17_decision9_the_first_press_position_is_kept_until_the_show() {
    // Key repeat sends more presses with newer positions; the hive shows
    // where the first press was (decision 9).
    let t = run(
        &[
            Step::HoldAt(0, 1198.0, 1800.0),
            Step::HoldAt(30, 1300.0, 1700.0),
            Step::HoldAt(60, -5.5, 2.25),
        ],
        1000,
    );
    assert_eq!(
        t.effects,
        [(
            400,
            Effect::Show {
                at: Some((1198.0, 1800.0))
            }
        )]
    );
}

#[test]
fn ac17_a_press_without_a_position_shows_without_one() {
    let t = run(&[Step::Hold(0)], 600);
    assert_eq!(t.effects, [(400, Effect::Show { at: None })]);
}

// AC19: the edge cases, with the injected clock.

#[test]
fn ac19_release_before_the_threshold_cancels_the_timer() {
    let t = run(&[Step::Hold(0), Step::Release(200)], 1000);
    assert_eq!(
        t.shows(),
        Vec::<u64>::new(),
        "zero show requests up to 1000 ms"
    );
    assert_eq!(t.timers_after, [(0, 1), (200, 0)], "the timer is cancelled");
    assert_eq!(
        t.releases(),
        Vec::<u64>::new(),
        "nothing shown, nothing to close"
    );
}

#[test]
fn ac19_five_presses_in_100_ms_keep_one_timer_from_the_first() {
    let steps = [
        Step::Hold(0),
        Step::Hold(25),
        Step::Hold(50),
        Step::Hold(75),
        Step::Hold(100),
    ];
    let t = run(&steps, 1000);
    assert_eq!(
        t.timers_after,
        [(0, 1), (25, 1), (50, 1), (75, 1), (100, 1)],
        "one live timer"
    );
    assert_eq!(
        t.shows(),
        [400],
        "one show, 400 ms after the first press (key repeat does not delay it)"
    );
}

#[test]
fn ac19_release_without_a_press_changes_nothing() {
    let t0 = Instant::now();
    let mut g = gate(400);
    assert_eq!(g.signal(Signal::Release, t0), None);
    assert_eq!(g.live_timers(), 0);
    assert_eq!(g.tick(ms(t0, 1000)), None, "no show");
    // The state is unchanged: a press afterwards behaves as the first one.
    let t = run_with(
        &mut g,
        &[Step::Release(0), Step::Hold(10), Step::Release(1000)],
        1200,
        &direct,
    );
    assert_eq!(t.shows(), [410]);
    assert_eq!(t.releases(), [1000]);
}

#[test]
fn ac19_a_press_while_the_hive_from_an_unreleased_press_shows_adds_nothing() {
    let steps = [
        Step::Hold(0),
        Step::Hold(500),
        Step::Hold(530),
        Step::Hold(900),
    ];
    let t = run(&steps, 2000);
    assert_eq!(t.shows(), [400], "one show request in all");
    assert_eq!(t.releases(), Vec::<u64>::new(), "no extra dismiss");
    assert_eq!(
        t.timers_after,
        [(0, 1), (500, 0), (530, 0), (900, 0)],
        "no new timer while the hive shows"
    );
}

#[test]
fn ac19_a_cancelled_timer_is_not_inherited() {
    let t = run(&[Step::Hold(0), Step::Release(200), Step::Hold(250)], 1200);
    assert_eq!(t.shows(), [650], "the show at 650 ms, not 400 ms");
}

#[test]
fn ac19_two_full_cycles_show_400_ms_after_each_press() {
    let steps = [
        Step::Hold(0),
        Step::Release(700),
        Step::Hold(1000),
        Step::Release(1600),
    ];
    let t = run(&steps, 2000);
    assert_eq!(t.shows(), [400, 1400], "two cycles of two");
    assert_eq!(t.releases(), [700, 1600], "each release closes its hive");
    assert_eq!(t.timers_after, [(0, 1), (700, 0), (1000, 1), (1600, 0)]);
}

#[test]
fn ac19_the_release_closes_only_once() {
    let t = run(
        &[
            Step::Hold(0),
            Step::Release(600),
            Step::Release(650),
            Step::Release(700),
        ],
        1000,
    );
    assert_eq!(t.releases(), [600]);
}

// AC18: hold_ms is read at every press, not once.

#[test]
fn ac18_hold_ms_is_read_at_every_press() {
    let value = Arc::new(AtomicU64::new(400));
    let reads = Arc::new(AtomicUsize::new(0));
    let (v, r) = (value.clone(), reads.clone());
    let mut g = Gate::new(move || {
        r.fetch_add(1, Ordering::SeqCst);
        v.load(Ordering::SeqCst)
    });
    let t = run_with(&mut g, &[Step::Hold(0), Step::Release(500)], 600, &direct);
    assert_eq!(t.shows(), [400]);
    let first = reads.load(Ordering::SeqCst);
    assert!(first >= 1, "read at the first press");
    value.store(1000, Ordering::SeqCst);
    let t = run_with(&mut g, &[Step::Hold(0), Step::Release(1500)], 1600, &direct);
    assert_eq!(t.shows(), [1000], "the new value applies at the next press");
    assert!(reads.load(Ordering::SeqCst) > first, "read again");
}

/// A private config dir, removed when dropped.
struct ConfigFile(PathBuf);

impl ConfigFile {
    fn new(test: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("sbarhive-s019e-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir.join("config.toml"))
    }

    fn write(&self, text: &str) {
        std::fs::write(&self.0, text).unwrap();
    }
}

impl Drop for ConfigFile {
    fn drop(&mut self) {
        if let Some(dir) = self.0.parent() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

#[test]
fn ac18_a_changed_config_file_applies_at_the_next_press_without_a_restart() {
    let file = ConfigFile::new("reload");
    file.write("language = \"en\"\n");
    let path = file.0.clone();
    let mut g = Gate::new(move || u64::from(Config::load_from(&path).0.hold_ms));
    // Absent key: 400.
    let t = run_with(&mut g, &[Step::Hold(0), Step::Release(1500)], 1600, &direct);
    assert_eq!(t.shows(), [400]);
    file.write("language = \"en\"\nhold_ms = 1000\n");
    let t = run_with(&mut g, &[Step::Hold(0), Step::Release(1500)], 1600, &direct);
    assert_eq!(t.shows(), [1000]);
    file.write("hold_ms = 150\n");
    let t = run_with(&mut g, &[Step::Hold(0), Step::Release(1500)], 1600, &direct);
    assert_eq!(t.shows(), [150]);
    // Out of range: the default (AC18, decision 10).
    file.write("hold_ms = 0\n");
    let t = run_with(&mut g, &[Step::Hold(0), Step::Release(1500)], 1600, &direct);
    assert_eq!(t.shows(), [400]);
}

// AC21: one entry point for both sources.

fn drop_req(hold: bool, at: Option<(f64, f64)>) -> Request {
    Request::Drop {
        token: None,
        at,
        hold,
    }
}

#[test]
fn ac21_ipc_requests_map_to_hold_and_release() {
    assert_eq!(hold::from_request(&drop_req(true, None)), Some(hold()));
    assert_eq!(
        hold::from_request(&drop_req(true, Some((400.0, 500.0)))),
        Some(Signal::Hold {
            at: Some((400.0, 500.0))
        })
    );
    assert_eq!(
        hold::from_request(&Request::DropRelease),
        Some(Signal::Release)
    );
    // A plain drop is the immediate show (AC11), not the gate.
    assert_eq!(hold::from_request(&drop_req(false, None)), None);
    assert_eq!(hold::from_request(&drop_req(false, Some((1.0, 2.0)))), None);
    assert_eq!(hold::from_request(&Request::Ping), None);
    assert_eq!(
        hold::from_request(&Request::Open {
            files: Vec::new(),
            token: None
        }),
        None
    );
}

#[test]
fn ac21_portal_signals_map_to_hold_and_release() {
    assert_eq!(hold::from_portal("Activated"), Some(hold()));
    assert_eq!(hold::from_portal("Deactivated"), Some(Signal::Release));
    for other in ["", "activated", "ShortcutsChanged", "Activated "] {
        assert_eq!(hold::from_portal(other), None, "{other:?}");
    }
}

/// The IPC source: each step as the request the CLI sends, mapped back.
fn via_ipc(s: Step) -> Signal {
    let r = match s {
        Step::Hold(_) => drop_req(true, None),
        Step::HoldAt(..) => unreachable!("the portal carries no position"),
        Step::Release(_) => Request::DropRelease,
    };
    hold::from_request(&r).expect("a gate signal")
}

/// The portal source: each step as the GlobalShortcuts signal name.
fn via_portal(s: Step) -> Signal {
    let name = match s {
        Step::Hold(_) => "Activated",
        Step::HoldAt(..) => unreachable!("the portal carries no position"),
        Step::Release(_) => "Deactivated",
    };
    hold::from_portal(name).expect("a gate signal")
}

#[test]
fn ac21_both_sources_give_the_same_trace_in_every_ac19_case() {
    let cases: [(&str, Vec<Step>, u64, Vec<u64>); 6] = [
        (
            "release before the threshold",
            vec![Step::Hold(0), Step::Release(200)],
            1000,
            vec![],
        ),
        (
            "five presses in 100 ms",
            vec![
                Step::Hold(0),
                Step::Hold(25),
                Step::Hold(50),
                Step::Hold(75),
                Step::Hold(100),
            ],
            1000,
            vec![400],
        ),
        (
            "release without a press",
            vec![Step::Release(0)],
            1000,
            vec![],
        ),
        (
            "press while shown",
            vec![Step::Hold(0), Step::Hold(500), Step::Hold(900)],
            1500,
            vec![400],
        ),
        (
            "cancelled timer not inherited",
            vec![Step::Hold(0), Step::Release(200), Step::Hold(250)],
            1200,
            vec![650],
        ),
        (
            "two full cycles",
            vec![
                Step::Hold(0),
                Step::Release(700),
                Step::Hold(1000),
                Step::Release(1600),
            ],
            2000,
            vec![400, 1400],
        ),
    ];
    for (name, steps, until, shows) in cases {
        let ipc = run_with(&mut gate(400), &steps, until, &via_ipc);
        let portal = run_with(&mut gate(400), &steps, until, &via_portal);
        let plain = run_with(&mut gate(400), &steps, until, &direct);
        assert_eq!(ipc.shows(), shows, "{name}: IPC");
        assert_eq!(ipc, portal, "{name}: the portal path is the IPC path");
        assert_eq!(ipc, plain, "{name}: one entry point");
    }
}

#[test]
fn ac21_a_press_from_one_source_and_the_release_from_the_other() {
    // One gate, no second path and no second timer: a portal press and an
    // IPC release (or the other way round) are the same press and release.
    let t0 = Instant::now();
    let mut g = gate(400);
    g.signal(hold::from_portal("Activated").unwrap(), t0);
    assert_eq!(
        g.signal(
            hold::from_request(&drop_req(true, None)).unwrap(),
            ms(t0, 50)
        ),
        None
    );
    assert_eq!(g.live_timers(), 1, "one timer for both sources");
    assert_eq!(g.tick(ms(t0, 400)), Some(Effect::Show { at: None }));
    assert_eq!(
        g.signal(
            hold::from_request(&Request::DropRelease).unwrap(),
            ms(t0, 600)
        ),
        Some(Effect::Release)
    );
    g.signal(
        hold::from_request(&drop_req(true, None)).unwrap(),
        ms(t0, 1000),
    );
    assert_eq!(
        g.signal(hold::from_portal("Deactivated").unwrap(), ms(t0, 1100)),
        None,
        "released before its threshold"
    );
    assert_eq!(g.live_timers(), 0);
    assert_eq!(g.tick(ms(t0, 2000)), None);
}
