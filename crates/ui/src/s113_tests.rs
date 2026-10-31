//! S-113 (backlog/S-113-x11-hive-window.md) AC1 to AC6: the hive window at
//! the pointer on X11, decided by pure functions. No display is needed; the
//! real `XQueryPointer` call is S-114.
//!
//! Hooks these tests pin (all `pub(crate)`, Linux only, modules registered
//! in `lib.rs`):
//! - `session.rs`: `enum Session { Wayland, X11, Windows, None }` with
//!   `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, and
//!   `fn detect(wayland_display: Option<&str>, display: Option<&str>,
//!   is_windows: bool) -> Session` (an empty value counts as unset).
//! - `x11.rs`, pure, plus `x11_pointer.rs` for the live adapter (S-114):
//!   - `struct Pointer { pub x: i32, pub y: i32, pub root_w: u32,
//!     pub root_h: u32 }` with `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`;
//!   - `trait PointerSource { fn pointer(&self) -> Option<Pointer>; }`
//!     (no `Send` bound);
//!   - `fn plan_for_pointer(cells: Vec<HiveCell>, rtl: bool, min_rings: i32,
//!     pointer: Pointer) -> (Plan, Placement)`: the pointer clamped to the
//!     root, then `layout::plan_at` at scale 120, no second copy of the maths;
//!   - `struct PointerLog` (`Default`) holding the warned flag, and
//!     `fn plan_with(source: &dyn PointerSource, fallback_root: (u32, u32),
//!     cells: Vec<HiveCell>, rtl: bool, min_rings: i32, log: &mut PointerLog,
//!     out: &mut dyn FnMut(&str)) -> (Plan, Placement)`: `plan_for_pointer`
//!     on `Some`; on `None` `plan_at` with no anchor on `fallback_root`, and
//!     `out("hive pointer: unavailable")` the first time only;
//!   - `struct WindowSpec { pub decorations: bool, pub always_on_top: bool,
//!     pub transparent: bool }` (`Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`)
//!     and `fn x11_window_spec() -> WindowSpec`;
//!   - `fn session_choice_line(session: Session) -> Option<String>`: `Some`
//!     of `overlay::choice_line(Surface::Window, "not a Wayland session")`
//!     for every session but `Wayland`, `None` for `Wayland`.
//! - `overlay.rs`: `fn next_line(previous: Option<&str>, line: &str) ->
//!   Option<String>`: `Some(line)` when it differs from `previous`.
//! - `hive.slint`: `in property <bool> x11-plate: false;` painting an opaque
//!   `Tokens.surface-0` rectangle under everything, and nothing when false.
//! - `Hive` (lib.rs):
//!   - `fn set_session(&self, session: Session, source: Box<dyn
//!     PointerSource>)` replaces the session read from the environment;
//!   - `fn x11_position(&self) -> Option<(i32, i32)>`: the position set by the
//!     last show on `Session::X11`, `None` after a show on any other session;
//!   - `show` on `X11` calls `plan_with` once (so the source once), sets
//!     the window position before `show()` and `x11-plate` to true; on
//!     `Wayland` and `Windows` it does neither.

use super::*;
use crate::layout::plan_at;
use crate::overlay::{Placement, Surface, choice_line, next_line};
use crate::session::{Session, detect};
use crate::x11::{
    Pointer, PointerLog, PointerSource, plan_for_pointer, plan_with, session_choice_line,
    x11_window_spec,
};
use slint::PlatformError;
use slint::platform::software_renderer::{
    MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType, TargetPixel,
};
use slint::platform::{Platform, WindowAdapter};
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

fn ptr(x: i32, y: i32) -> Pointer {
    Pointer {
        x,
        y,
        root_w: ROOT.0,
        root_h: ROOT.1,
    }
}

/// Everything of a plan the tests compare (`Plan` has no `PartialEq`).
type Shape = (Vec<(String, i32, i32)>, i32, i32, (u32, u32), (i32, i32));

fn shape(p: &(crate::layout::Plan, Placement)) -> Shape {
    let cells =
        p.0.cells
            .iter()
            .map(|c| {
                (
                    c.cell.id.clone(),
                    (c.x * 100.0).round() as i32,
                    (c.y * 100.0).round() as i32,
                )
            })
            .collect();
    (
        cells,
        (p.0.width * 100.0).round() as i32,
        (p.0.height * 100.0).round() as i32,
        p.1.destination,
        (p.1.x, p.1.y),
    )
}

// AC1

#[test]
fn ac1_session_detect_table_of_8_rows() {
    use Session::{None as Nope, Wayland, Windows, X11};
    let rows: [(Option<&str>, Option<&str>, bool, Session); 8] = [
        (Some("wayland-0"), Some(":0"), false, Wayland),
        (Some("wayland-0"), None, false, Wayland),
        (None, Some(":0"), false, X11),
        (Some(""), Some(":0"), false, X11),
        (Some(""), Some(""), false, Nope),
        (None, None, false, Nope),
        (None, None, true, Windows),
        (None, Some(":0"), true, X11),
    ];
    for (w, d, win, want) in rows {
        assert_eq!(
            detect(w, d, win),
            want,
            "wayland {w:?} display {d:?} win {win}"
        );
    }
}

// AC2

#[test]
fn ac2_pointer_400_500_puts_the_centre_cell_centre_there() {
    let (plan, place) = plan_for_pointer(cells(6), false, 1, ptr(400, 500));
    let cx = place.x as f32 + plan.layout.origin.0;
    let cy = place.y as f32 + plan.layout.origin.1;
    assert!((cx - 400.0).abs() <= 1.0, "centre x {cx}");
    assert!((cy - 500.0).abs() <= 1.0, "centre y {cy}");
}

#[test]
fn ac2_every_pointer_keeps_the_box_on_the_root_and_matches_plan_at() {
    for (x, y) in [(0, 0), (1279, 799), (100, 700), (1200, 80), (400, 500)] {
        let got = plan_for_pointer(cells(12), false, 1, ptr(x, y));
        let want = plan_at(cells(12), false, 1, ROOT, Some((x as f32, y as f32)), 120);
        let p = got.1;
        assert!(
            p.x >= 0 && p.y >= 0,
            "({x}, {y}): box at ({}, {})",
            p.x,
            p.y
        );
        assert!(
            // plan_at rounds the position to the nearest pixel and the width
            // up, so the box may end 1 px past the root at an exact corner;
            // that is plan_at's existing behaviour (anchor_tests pin it).
            p.x as i64 + p.destination.0 as i64 <= i64::from(ROOT.0) + 1
                && p.y as i64 + p.destination.1 as i64 <= i64::from(ROOT.1) + 1,
            "({x}, {y}): box {p:?} leaves the root"
        );
        assert_eq!(
            got.0.cells.len(),
            want.0.cells.len(),
            "({x}, {y}) cell count"
        );
        assert_eq!(shape(&got), shape(&want), "({x}, {y}) equals plan_at");
    }
}

#[test]
fn ac2_a_pointer_outside_the_root_is_clamped_not_a_panic() {
    let outside = plan_for_pointer(cells(12), false, 1, ptr(-5, 900));
    let corner = plan_for_pointer(cells(12), false, 1, ptr(0, 799));
    assert_eq!(shape(&outside), shape(&corner));
}

// AC3

struct Fake {
    at: Option<Pointer>,
    calls: Rc<Cell<u32>>,
}

impl PointerSource for Fake {
    fn pointer(&self) -> Option<Pointer> {
        self.calls.set(self.calls.get() + 1);
        self.at
    }
}

fn fake(at: Option<Pointer>) -> (Fake, Rc<Cell<u32>>) {
    let calls = Rc::new(Cell::new(0));
    (
        Fake {
            at,
            calls: calls.clone(),
        },
        calls,
    )
}

#[test]
fn ac3_a_source_with_a_pointer_gives_the_ac2_plan_and_no_line() {
    let (src, _) = fake(Some(ptr(400, 500)));
    let mut log = PointerLog::default();
    let mut lines: Vec<String> = Vec::new();
    let got = plan_with(&src, ROOT, cells(6), false, 1, &mut log, &mut |l| {
        lines.push(l.to_string())
    });
    assert_eq!(
        shape(&got),
        shape(&plan_for_pointer(cells(6), false, 1, ptr(400, 500)))
    );
    assert!(lines.is_empty(), "{lines:?}");
}

#[test]
fn ac3_no_pointer_centres_the_hive_and_logs_one_line_for_five_shows() {
    let (src, _) = fake(None);
    let mut log = PointerLog::default();
    let mut lines: Vec<String> = Vec::new();
    let want = plan_at(cells(6), false, 1, ROOT, None, 120);
    for i in 0..5 {
        let got = plan_with(&src, ROOT, cells(6), false, 1, &mut log, &mut |l| {
            lines.push(l.to_string())
        });
        assert_eq!(shape(&got), shape(&want), "show {i}");
        assert_eq!(lines, ["hive pointer: unavailable"], "after show {i}");
    }
}

// AC4

#[test]
fn ac4_a_non_wayland_session_has_the_window_choice_line() {
    let want = "hive surface choice: window, not a Wayland session";
    for s in [Session::X11, Session::Windows, Session::None] {
        assert_eq!(session_choice_line(s).as_deref(), Some(want), "{s:?}");
    }
    assert_eq!(
        session_choice_line(Session::X11),
        Some(choice_line(Surface::Window, "not a Wayland session"))
    );
    assert_eq!(session_choice_line(Session::Wayland), None);
}

#[test]
fn ac4_20_identical_lines_print_once_and_a_changed_reason_prints_again() {
    let line = "hive surface choice: window, not a Wayland session";
    let mut previous: Option<String> = None;
    let mut printed = 0;
    for _ in 0..20 {
        if let Some(l) = next_line(previous.as_deref(), line) {
            assert_eq!(l, line);
            printed += 1;
            previous = Some(l);
        }
    }
    assert_eq!(printed, 1);
    let other = "hive surface choice: window, screen reader running";
    assert_eq!(
        next_line(previous.as_deref(), other).as_deref(),
        Some(other)
    );
}

// AC5

#[test]
fn ac5_the_x11_window_spec_is_plain_and_on_top() {
    let spec = x11_window_spec();
    assert!(!spec.decorations, "no decorations");
    assert!(spec.always_on_top, "always on top");
    assert!(!spec.transparent, "not transparent");
}

thread_local! {
    static WINDOW: RefCell<Option<Rc<MinimalSoftwareWindow>>> = const { RefCell::new(None) };
}

struct SoftPlatform;

impl Platform for SoftPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        let w = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        WINDOW.with(|c| *c.borrow_mut() = Some(w.clone()));
        Ok(w)
    }
}

#[derive(Clone, Copy, Default)]
struct Px {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl TargetPixel for Px {
    fn blend(&mut self, c: PremultipliedRgbaColor) {
        let keep = u16::from(255 - c.alpha);
        let mix = |d: u8, s: u8| (u16::from(d) * keep / 255) as u8 + s;
        self.r = mix(self.r, c.red);
        self.g = mix(self.g, c.green);
        self.b = mix(self.b, c.blue);
        self.a = mix(self.a, c.alpha);
    }
    fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
    fn background() -> Self {
        Self::default()
    }
}

/// Shows the hive on `session` with a pointer at (400, 500) of the 1280 x
/// 800 root, renders it in `scheme`, and returns the pixels (w, h, RGBA).
fn render(scheme: &str, session: Session) -> (u32, u32, Vec<u8>) {
    slint::platform::set_platform(Box::new(SoftPlatform)).ok();
    let hive = test_env::build(&[("KAWARO_COLOR_SCHEME", scheme)], Lang::En);
    let (src, _) = fake(Some(ptr(400, 500)));
    hive.set_session(session, Box::new(src));
    hive.present_files("centre", &[], cells(6), None).unwrap();
    hive.inner.ui.borrow().set_opened(true);
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(500));
    let win = WINDOW
        .with(|c| c.borrow().clone())
        .expect("the hive made a window");
    let (w, h) = {
        let ui = hive.inner.ui.borrow();
        (ui.get_win_w() as u32, ui.get_win_h() as u32)
    };
    win.set_size(slint::PhysicalSize::new(w, h));
    win.dispatch_event(slint::platform::WindowEvent::ScaleFactorChanged { scale_factor: 1.0 });
    let mut buf = vec![Px::default(); (w * h) as usize];
    win.request_redraw();
    assert!(
        win.draw_if_needed(|r| {
            r.render(&mut buf, w as usize);
        }),
        "the hive window drew nothing"
    );
    (
        w,
        h,
        buf.iter().flat_map(|p| [p.r, p.g, p.b, p.a]).collect(),
    )
}

/// `surface-0` of `scheme` read from design/tokens.json.
fn surface_0(scheme: &str) -> [u8; 3] {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../design/tokens.json");
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let hex = json["color"][scheme]["surface-0"].as_str().unwrap();
    let v = |i: usize| u8::from_str_radix(&hex[1 + 2 * i..3 + 2 * i], 16).unwrap();
    [v(0), v(1), v(2)]
}

fn corner_pixels(shot: &(u32, u32, Vec<u8>)) -> Vec<[u8; 4]> {
    let at = |x: u32, y: u32| {
        let i = ((y * shot.0 + x) * 4) as usize;
        [shot.2[i], shot.2[i + 1], shot.2[i + 2], shot.2[i + 3]]
    };
    vec![
        at(0, 0),
        at(shot.0 - 1, 0),
        at(0, shot.1 - 1),
        at(shot.0 - 1, shot.1 - 1),
    ]
}

#[test]
fn ac5_the_x11_hive_corner_pixel_is_surface_0_in_both_schemes() {
    for scheme in ["light", "dark"] {
        let shot = render(scheme, Session::X11);
        let [r, g, b] = surface_0(scheme);
        for (i, p) in corner_pixels(&shot).into_iter().enumerate() {
            assert_eq!(p, [r, g, b, 255], "{scheme} corner {i}");
        }
    }
}

#[test]
fn ac5_the_plate_is_absent_off_x11() {
    for session in [Session::Wayland, Session::Windows] {
        let shot = render("light", session);
        for (i, p) in corner_pixels(&shot).into_iter().enumerate() {
            assert_eq!(p[3], 0, "{session:?} corner {i} has alpha {}", p[3]);
        }
    }
}

// AC6

fn shown_on(session: Session, at: Option<Pointer>) -> (Hive, Rc<Cell<u32>>) {
    slint::platform::set_platform(Box::new(SoftPlatform)).ok();
    let hive = test_env::build(&[], Lang::En);
    let (src, calls) = fake(at);
    hive.set_session(session, Box::new(src));
    hive.present_files("centre", &[], cells(6), None).unwrap();
    (hive, calls)
}

#[test]
fn ac6_show_on_x11_asks_the_source_once_and_sets_the_position() {
    let (hive, calls) = shown_on(Session::X11, Some(ptr(400, 500)));
    assert_eq!(calls.get(), 1);
    let want = plan_for_pointer(cells(6), false, 1, ptr(400, 500)).1;
    assert_eq!(hive.x11_position(), Some((want.x, want.y)));
    assert!(hive.inner.ui.borrow().get_x11_plate());
}

#[test]
fn ac6_show_on_wayland_and_windows_never_asks_the_source() {
    for session in [Session::Wayland, Session::Windows] {
        let (hive, calls) = shown_on(session, Some(ptr(400, 500)));
        assert_eq!(calls.get(), 0, "{session:?}");
        assert_eq!(hive.x11_position(), None, "{session:?}");
        assert!(!hive.inner.ui.borrow().get_x11_plate(), "{session:?}");
    }
}
