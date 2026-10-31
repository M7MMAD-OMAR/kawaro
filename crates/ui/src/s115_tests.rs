//! S-115 (backlog/S-115-x11-drop-cell-and-docs.md) AC1 to AC4 and AC6: the
//! drop hive on X11, on Slint's headless testing platform with a fake
//! pointer. No display is needed; the live checks are `tests/x11/drop.sh`.
//!
//! Hooks these tests pin (Linux only):
//! - `x11::drop_placement(at: Option<(f64, f64)>, screen: (u32, u32),
//!   cell: (u32, u32)) -> Placement`: the choice of the drop hive's place on
//!   X11. It ignores `at`: the answer is `overlay::place(screen, cell, 120)`.
//! - `x11::window_spec_for(drop: bool) -> WindowSpec`: `true` is a drop show
//!   (no decorations, always on top, opaque), `false` is exactly
//!   `x11_window_spec()` (S-113, unchanged).
//! - `Hive::present_drop_at` on `Session::X11` places the window with
//!   `drop_placement`; `Hive::x11_position()` then reports that position.
//! - `dnd/x11.rs`, registered in `dnd.rs` as `pub mod x11` (Linux only),
//!   mirrors `dnd/windows.rs` with `pub fn window_event(&WindowEvent)`
//!   (called by `FocusWatcher::window_event`) and `pub fn hive_closed()`.
//!   The pointer is the one of `Hive::set_session` (`PointerSource`, root
//!   pixels); the hive surface origin is `Hive::x11_position()`, scale 1,
//!   so the surface local cursor is pointer minus position. The dragged
//!   files of several `HoveredFile` events reach `on_drop_files` once (one
//!   `drop cells:` line), by the time `TICK_MS` has elapsed after the last
//!   hover event, not once per event. A `TICK_MS` timer moves the highlight.

use super::*;
use crate::drop_track::TICK_MS;
use crate::overlay::place;
use crate::session::Session;
use crate::x11::{Pointer, PointerSource, WindowSpec, drop_placement, window_spec_for};
use i_slint_backend_testing::mock_elapsed_time;
use slint::winit_030::winit::event::WindowEvent;

const ROOT: (u32, u32) = (1280, 800);
/// A pointer far from every cell: the top left corner of the root.
const FAR: (i32, i32) = (0, 0);

struct Fake(Rc<Cell<(i32, i32)>>);

impl PointerSource for Fake {
    fn pointer(&self) -> Option<Pointer> {
        let (x, y) = self.0.get();
        Some(Pointer {
            x,
            y,
            root_w: ROOT.0,
            root_h: ROOT.1,
        })
    }
}

#[derive(Default)]
struct Log {
    asked: RefCell<Vec<Vec<PathBuf>>>,
    chosen: RefCell<Vec<String>>,
    dismissed: Cell<u32>,
}

fn cells() -> Vec<HiveCell> {
    ["webp", "jpg", "gray"]
        .iter()
        .map(|id| HiveCell {
            id: (*id).into(),
            code: id.to_uppercase(),
            label: format!("label {id}"),
            icon: String::new(),
            ring: 1,
            enabled: true,
            reason: String::new(),
        })
        .collect()
}

fn files(n: usize) -> Vec<PathBuf> {
    (0..n)
        .map(|i| PathBuf::from(format!("/tmp/s115/{i}.png")))
        .collect()
}

struct Rig {
    hive: Hive,
    log: Rc<Log>,
    pointer: Rc<Cell<(i32, i32)>>,
}

fn rig(vars: &[(&str, &str)]) -> Rig {
    i_slint_backend_testing::init_no_event_loop();
    let hive = test_env::build(vars, Lang::En);
    let pointer = Rc::new(Cell::new(FAR));
    hive.set_session(Session::X11, Box::new(Fake(pointer.clone())));
    let log = Rc::new(Log::default());
    let l = log.clone();
    hive.on_drop_files(move |f| {
        let n = f.len().to_string();
        l.asked.borrow_mut().push(f);
        (n, cells())
    });
    let l = log.clone();
    hive.on_choose(move |id| l.chosen.borrow_mut().push(id));
    let l = log.clone();
    hive.on_dismiss(move || l.dismissed.set(l.dismissed.get() + 1));
    Rig { hive, log, pointer }
}

fn drop_rig(vars: &[(&str, &str)]) -> Rig {
    let r = rig(vars);
    r.hive
        .present_drop_at("", None, Some((400.0, 500.0)))
        .unwrap();
    r
}

fn turn(ms: u64) {
    mock_elapsed_time(Duration::from_millis(ms));
}

fn hover(n: usize) {
    for f in files(n) {
        dnd::x11::window_event(&WindowEvent::HoveredFile(f));
    }
    turn(u64::from(TICK_MS) * 2);
}

/// Pointer (root pixels) over the centre of cell `k` of the current plan.
fn over_cell(r: &Rig, k: usize) -> (i32, i32) {
    let (px, py) = r.hive.x11_position().expect("an X11 position");
    let plan = r.hive.inner.plan.borrow();
    let c = &plan.as_ref().expect("a plan").cells[k];
    (px + c.x.round() as i32, py + c.y.round() as i32)
}

fn over_centre(r: &Rig) -> (i32, i32) {
    let (px, py) = r.hive.x11_position().unwrap();
    let plan = r.hive.inner.plan.borrow();
    let o = plan.as_ref().unwrap().layout.origin;
    (px + o.0.round() as i32, py + o.1.round() as i32)
}

#[test]
fn ac1_the_drop_placement_ignores_the_pointer() {
    let cell = (392, 361);
    let want = place(ROOT, cell, 120);
    for at in [
        None,
        Some((400.0, 500.0)),
        Some((0.0, 0.0)),
        Some((1279.0, 799.0)),
    ] {
        assert_eq!(drop_placement(at, ROOT, cell), want, "at {at:?}");
    }
    assert_eq!(
        drop_placement(Some((7.0, 9.0)), ROOT, cell),
        drop_placement(None, ROOT, cell)
    );
}

#[test]
fn ac1_present_drop_at_centres_the_window_with_and_without_at() {
    let mut seen = Vec::new();
    for at in [Some((400.0, 500.0)), None, Some((1279.0, 799.0))] {
        let r = rig(&[]);
        r.hive.present_drop_at("", None, at).unwrap();
        let (w, h) = {
            let ui = r.hive.inner.ui.borrow();
            (ui.get_win_w().round() as u32, ui.get_win_h().round() as u32)
        };
        let p = place(ROOT, (w, h), 120);
        assert_eq!(r.hive.x11_position(), Some((p.x, p.y)), "at {at:?}");
        seen.push(r.hive.x11_position());
    }
    seen.dedup();
    assert_eq!(
        seen.len(),
        1,
        "one centred position for every `at`: {seen:?}"
    );
}

#[test]
fn ac2_the_drop_window_spec_is_always_on_top_and_the_normal_spec_is_unchanged() {
    assert_eq!(
        window_spec_for(true),
        WindowSpec {
            decorations: false,
            always_on_top: true,
            transparent: false
        }
    );
    assert_eq!(window_spec_for(false), crate::x11::x11_window_spec());
}

#[test]
fn ac3_three_hover_events_ask_for_the_cells_once_and_show_the_count() {
    let r = drop_rig(&[]);
    hover(3);
    assert_eq!(
        r.log.asked.borrow().as_slice(),
        [files(3)],
        "one request with the three files (one `drop cells:` line)"
    );
    let ui = r.hive.inner.ui.borrow();
    assert_eq!(ui.get_cells().row_count(), 3);
    assert!(
        ui.get_centre_text().contains('3'),
        "{}",
        ui.get_centre_text()
    );
}

#[test]
fn ac3_hover_cancelled_closes_the_hive_like_leave() {
    let r = drop_rig(&[]);
    hover(2);
    dnd::x11::window_event(&WindowEvent::HoveredFileCancelled);
    turn(120);
    assert!(!r.hive.inner.visible());
    assert_eq!(r.log.dismissed.get(), 1);
    assert!(r.log.chosen.borrow().is_empty());
}

#[test]
fn ac3_a_drop_over_cell_k_runs_cell_k_once() {
    for (k, id) in ["webp", "jpg", "gray"].iter().enumerate() {
        let r = drop_rig(&[]);
        hover(3);
        r.pointer.set(over_cell(&r, k));
        dnd::x11::window_event(&WindowEvent::DroppedFile(files(3)[0].clone()));
        turn(120);
        assert_eq!(
            r.log.chosen.borrow().as_slice(),
            [(*id).to_string()],
            "cell {k}"
        );
        assert!(!r.hive.inner.visible(), "cell {k} closes the hive");
    }
}

#[test]
fn ac3_a_drop_over_the_centre_or_a_gap_runs_nothing_and_closes_within_500_ms() {
    for spot in ["centre", "corner"] {
        let r = drop_rig(&[]);
        hover(3);
        r.pointer.set(if spot == "centre" {
            over_centre(&r)
        } else {
            FAR
        });
        dnd::x11::window_event(&WindowEvent::DroppedFile(files(3)[0].clone()));
        turn(500);
        assert!(r.log.chosen.borrow().is_empty(), "{spot}: no action");
        assert!(!r.hive.inner.visible(), "{spot}: closed within 500 ms");
    }
}

#[test]
fn ac4_a_tick_with_the_pointer_on_cell_k_highlights_k() {
    let r = drop_rig(&[]);
    hover(1);
    assert_eq!(r.hive.inner.ui.borrow().get_highlight(), -1);
    r.pointer.set(over_cell(&r, 2));
    turn(u64::from(TICK_MS));
    assert_eq!(r.hive.inner.ui.borrow().get_highlight(), 2);
    r.pointer.set(over_cell(&r, 0));
    turn(u64::from(TICK_MS));
    assert_eq!(r.hive.inner.ui.borrow().get_highlight(), 0);
}

#[test]
fn ac4_with_reduced_motion_the_highlight_changes_at_once() {
    let r = drop_rig(&[("KAWARO_REDUCED_MOTION", "1")]);
    assert!(r.hive.inner.reduced_motion);
    hover(1);
    r.pointer.set(over_cell(&r, 1));
    turn(u64::from(TICK_MS));
    assert_eq!(r.hive.inner.ui.borrow().get_highlight(), 1);
}

#[test]
fn ac6_a_dropped_file_on_a_normal_hive_runs_nothing() {
    let r = rig(&[]);
    r.hive.present_files("centre", &[], cells(), None).unwrap();
    let k = over_cell(&r, 1);
    r.pointer.set(k);
    hover(3);
    dnd::x11::window_event(&WindowEvent::DroppedFile(files(3)[0].clone()));
    turn(120);
    assert!(r.log.chosen.borrow().is_empty(), "no action from a drop");
    assert!(r.log.asked.borrow().is_empty(), "no drop request either");
    assert!(r.hive.inner.visible(), "a normal hive stays open");
}
