//! The hive window on X11 (S-113): where it opens and how it looks, decided
//! by pure functions. The real pointer query is `x11_pointer.rs` (S-114).

use crate::HiveCell;
use crate::layout::{Plan, plan_at};
use crate::overlay::{Placement, Surface, choice_line};
use crate::session::Session;

/// The pointer on the root window and the root's size, in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pointer {
    pub x: i32,
    pub y: i32,
    pub root_w: u32,
    pub root_h: u32,
}

/// Where the pointer is, or `None` when it cannot be read.
pub trait PointerSource {
    fn pointer(&self) -> Option<Pointer>;
}

/// The hive at `pointer`, clamped to the root, by `plan_at` at scale 120.
pub fn plan_for_pointer(
    cells: Vec<HiveCell>,
    rtl: bool,
    min_rings: i32,
    pointer: Pointer,
) -> (Plan, Placement) {
    let clamp = |v: i32, size: u32| v.clamp(0, (size as i32 - 1).max(0));
    let anchor = (
        clamp(pointer.x, pointer.root_w) as f32,
        clamp(pointer.y, pointer.root_h) as f32,
    );
    plan_at(
        cells,
        rtl,
        min_rings,
        (pointer.root_w, pointer.root_h),
        Some(anchor),
        120,
    )
}

/// Whether the unavailable pointer line was already printed.
#[derive(Default)]
pub struct PointerLog {
    warned: bool,
}

/// [`plan_for_pointer`] on the source's pointer. Without one the hive is
/// centred on `fallback_root`, and `out` gets `hive pointer: unavailable`
/// the first time only.
pub fn plan_with(
    source: &dyn PointerSource,
    fallback_root: (u32, u32),
    cells: Vec<HiveCell>,
    rtl: bool,
    min_rings: i32,
    log: &mut PointerLog,
    out: &mut dyn FnMut(&str),
) -> (Plan, Placement) {
    match source.pointer() {
        Some(p) => plan_for_pointer(cells, rtl, min_rings, p),
        None => {
            if !log.warned {
                log.warned = true;
                out("hive pointer: unavailable");
            }
            plan_at(cells, rtl, min_rings, fallback_root, None, 120)
        }
    }
}

/// How the X11 hive window is made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSpec {
    pub decorations: bool,
    pub always_on_top: bool,
    pub transparent: bool,
}

/// Plain, on top and opaque: an X11 session has no compositor to count on.
pub fn x11_window_spec() -> WindowSpec {
    WindowSpec {
        decorations: false,
        always_on_top: true,
        transparent: false,
    }
}

/// The window of a show: a drop show and a normal show are made the same
/// way, plain, on top and opaque (S-115 AC2). The drop hive must stay above
/// the file manager window that was raised by the click starting the drag.
pub fn window_spec_for(_drop: bool) -> WindowSpec {
    x11_window_spec()
}

/// Where the drop hive opens (S-115 AC1): the root centre, whatever `at` is.
/// A drag gives no pointer position before it enters, so it is not used.
pub fn drop_placement(_at: Option<(f64, f64)>, screen: (u32, u32), cell: (u32, u32)) -> Placement {
    crate::overlay::place(screen, cell, 120)
}

/// The surface choice line of a session that is not Wayland.
pub fn session_choice_line(session: Session) -> Option<String> {
    (session != Session::Wayland).then(|| choice_line(Surface::Window, "not a Wayland session"))
}

/// The window path's surface line, in screen pixels.
pub fn surface_line(x: i32, y: i32, w: u32, h: u32) -> String {
    format!("hive surface: window {x} {y} {w} {h}")
}

/// What the first frames of an X11 hive do about its position (S-116).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceStep {
    /// Ask the window manager to move the window here.
    Move((i32, i32)),
    /// The move was asked for and has not happened yet.
    Wait,
    /// Print the surface line with this position.
    Report((i32, i32)),
}

/// A window manager may place a new window itself, so the planned position
/// is asked for once more after the window is mapped.
pub struct PlaceCheck {
    target: (i32, i32),
    requested: bool,
}

impl PlaceCheck {
    pub fn new(target: (i32, i32)) -> Self {
        Self {
            target,
            requested: false,
        }
    }

    /// The position read on a frame.
    pub fn seen(&mut self, actual: (i32, i32)) -> PlaceStep {
        if actual == self.target {
            PlaceStep::Report(actual)
        } else if self.requested {
            PlaceStep::Wait
        } else {
            self.requested = true;
            PlaceStep::Move(self.target)
        }
    }

    /// The position the window manager gave in a move event.
    pub fn moved(&mut self, actual: (i32, i32)) -> PlaceStep {
        if self.requested || actual == self.target {
            PlaceStep::Report(actual)
        } else {
            PlaceStep::Wait
        }
    }
}
