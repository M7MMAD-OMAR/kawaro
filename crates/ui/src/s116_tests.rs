//! S-116 (backlog/S-116-x11-placement-applied.md): the X11 hive window ends
//! up where the plan put it, and the log line says where it really is.
//!
//! Hooks these tests pin (Linux only):
//! - `x11::surface_line(x, y, w, h) -> String`: `hive surface: window X Y W H`.
//! - `Hive::x11_surface_line(&self) -> Option<String>`: the line for the
//!   last show on X11 (planned position, window size), `None` elsewhere.
//! - `x11::PlaceCheck`: what the first frames of an X11 hive do with the
//!   position the window manager gave (`new(target)`, then `seen(actual)`
//!   returns `Move(target)` once when the position differs, `Report` when it
//!   equals the target or after the move was already requested, never
//!   `Move` twice).

use super::*;
use crate::layout::plan_at;
use crate::session::Session;
use crate::x11::{PlaceCheck, PlaceStep, Pointer, PointerSource, surface_line};
use std::cell::Cell;

const ROOT: (u32, u32) = (1280, 800);

fn cells(n: usize) -> Vec<HiveCell> {
    (0..n)
        .map(|i| HiveCell {
            id: format!("c{i}"),
            code: format!("C{i}"),
            label: format!("label {i}"),
            icon: "image".into(),
            ring: if i < 6 { 1 } else { 2 },
            enabled: true,
            reason: String::new(),
        })
        .collect()
}

struct Fake(Pointer, Rc<Cell<u32>>);

impl PointerSource for Fake {
    fn pointer(&self) -> Option<Pointer> {
        self.1.set(self.1.get() + 1);
        Some(self.0)
    }
}

fn shown(x: i32, y: i32, session: Session) -> Hive {
    i_slint_backend_testing::init_no_event_loop();
    let hive = test_env::build(&[], Lang::En);
    let p = Pointer {
        x,
        y,
        root_w: ROOT.0,
        root_h: ROOT.1,
    };
    hive.set_session(session, Box::new(Fake(p, Rc::new(Cell::new(0)))));
    hive.present_files("centre", &[], cells(6), None).unwrap();
    hive
}

#[test]
fn the_window_position_equals_the_plan_at_three_pointers() {
    let mut seen = Vec::new();
    for (x, y) in [(400, 500), (0, 0), (1279, 799)] {
        let hive = shown(x, y, Session::X11);
        let want = hive.x11_position().expect("a position on X11");
        let got = hive.inner.ui.borrow().window().position();
        assert_eq!((got.x, got.y), want, "pointer ({x}, {y})");
        let plan = plan_at(
            cells(6),
            false,
            1,
            ROOT,
            Some((x as f32, y as f32)),
            120,
        )
        .1;
        assert_eq!(want, (plan.x, plan.y), "pointer ({x}, {y}) is the plan");
        seen.push(want);
    }
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), 3, "three different positions: {seen:?}");
}

#[test]
fn the_log_line_reports_the_planned_position_and_the_window_size() {
    for (x, y) in [(400, 500), (0, 0), (1279, 799)] {
        let hive = shown(x, y, Session::X11);
        let (px, py) = hive.x11_position().unwrap();
        let (w, h) = {
            let ui = hive.inner.ui.borrow();
            (ui.get_win_w().round() as u32, ui.get_win_h().round() as u32)
        };
        assert_eq!(
            hive.x11_surface_line(),
            Some(surface_line(px, py, w, h)),
            "pointer ({x}, {y})"
        );
    }
    assert_eq!(
        surface_line(444, 220, 392, 361),
        "hive surface: window 444 220 392 361"
    );
}

#[test]
fn no_surface_line_off_x11() {
    for session in [Session::Wayland, Session::Windows] {
        assert_eq!(shown(400, 500, session).x11_surface_line(), None);
    }
}

#[test]
fn the_first_frames_move_the_window_once_when_the_wm_placed_it_elsewhere() {
    let mut c = PlaceCheck::new((100, 200));
    assert_eq!(c.seen((444, 220)), PlaceStep::Move((100, 200)));
    // the move is requested, the wm has not answered yet: wait, do not spam
    assert_eq!(c.seen((444, 220)), PlaceStep::Wait);
    // the wm moved it: report the real position
    assert_eq!(c.seen((100, 200)), PlaceStep::Report((100, 200)));
}

#[test]
fn a_window_already_at_the_target_is_reported_without_a_move() {
    let mut c = PlaceCheck::new((100, 200));
    assert_eq!(c.seen((100, 200)), PlaceStep::Report((100, 200)));
}

#[test]
fn a_moved_event_after_the_request_reports_what_the_wm_decided() {
    let mut c = PlaceCheck::new((100, 200));
    assert_eq!(c.seen((444, 220)), PlaceStep::Move((100, 200)));
    assert_eq!(c.moved((120, 230)), PlaceStep::Report((120, 230)));
}
