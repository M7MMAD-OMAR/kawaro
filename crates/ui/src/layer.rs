//! The hive on a zwlr_layer_shell_v1 overlay (S-018b, ADR 0004), after
//! `spikes/overlay/src/overlay.rs`. A Slint window adapter with the software
//! renderer, so no compositor tiles, frames, places or unfocuses the hive.
//!
//! Per show (`set_visible(true)`), destroyed again on hide:
//! - catcher: a layer surface on the overlay layer anchored to all edges,
//!   exclusive zone -1, keyboard exclusive (none in drop mode). Its buffer is
//!   one transparent pixel stretched by wp_viewporter, attached at the first
//!   configure so the pointer can enter it; a click on it, outside the hive,
//!   asks to close (never in drop mode). In drop mode its input region is
//!   its default one until the first position (or [`MOVE_WINDOW`] after the
//!   placement), then the hive rectangle, so
//!   it takes the drag (S-020: Hyprland sends no drag motion to a layer
//!   surface's subsurface; S-019 AC8).
//! - hive: a subsurface of the catcher, placed by [`layout::plan_at`] at
//!   the anchor, holding the Slint frame in a wl_shm buffer; no input in
//!   drop mode. A drop show attaches no hive buffer until [`probed`] reveals
//!   it for a drag of files with an action (S-021); its place, size and
//!   regions are sent at bind all the same.
//!
//! The anchor (S-019 AC5, AC8) is the first pointer enter or motion on the
//! catcher, or drag position in drop mode, waited for at most
//! [`ANCHOR_WAIT`] from the first configure; without one the hive is
//! centred. A drop show centred so moves once, at its first position within
//! [`MOVE_WINDOW`] of its placement; after that its catcher takes the hive
//! rectangle and the hive stays centred (manager decision 12). With
//! `kawaro drop --at X,Y` the output holding that point (xdg-output) is
//! chosen and the hive placed before the first commit.
//!
//! One wayland-client connection for the process, read on a thread; its
//! events are dispatched on the Slint thread and go to the adapter that is
//! shown. Decisions (surface, placement, keys) come from [`crate::overlay`].
//! A drop-mode show also binds the drop receiver (`receiver.rs`) on this
//! connection's display, to the catcher, offset by the hive position (S-020).

use crate::HiveCell;
use crate::dnd;
use crate::layout::{self, Plan};
use crate::overlay::{self, Surface};
use rustix::event::{PollFd, PollFlags, poll};
use slint::platform::software_renderer::{
    PremultipliedRgbaColor, RepaintBufferType, SoftwareRenderer, TargetPixel,
};
use slint::platform::{Key, PointerEventButton, Renderer, WindowAdapter, WindowEvent};
use slint::{LogicalPosition, LogicalSize, PhysicalSize, PlatformError, WindowSize};
use std::cell::{Cell, RefCell};
use std::io::Write;
use std::os::fd::AsFd;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use wayland_client::globals::{GlobalList, GlobalListContents, registry_queue_init};
use wayland_client::protocol::{
    wl_buffer::{self, WlBuffer},
    wl_callback::{self, WlCallback},
    wl_compositor::WlCompositor,
    wl_keyboard::{self, WlKeyboard},
    wl_output::{self, WlOutput},
    wl_pointer::{self, WlPointer},
    wl_region::WlRegion,
    wl_registry::WlRegistry,
    wl_seat::{self, WlSeat},
    wl_shm::{self, WlShm},
    wl_shm_pool::WlShmPool,
    wl_subcompositor::WlSubcompositor,
    wl_subsurface::WlSubsurface,
    wl_surface::{self, WlSurface},
};
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle, WEnum, delegate_noop};
use wayland_protocols::wp::cursor_shape::v1::client::{
    wp_cursor_shape_device_v1::{Shape, WpCursorShapeDeviceV1},
    wp_cursor_shape_manager_v1::WpCursorShapeManagerV1,
};
use wayland_protocols::wp::fractional_scale::v1::client::{
    wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
    wp_fractional_scale_v1::{self, WpFractionalScaleV1},
};
use wayland_protocols::wp::single_pixel_buffer::v1::client::wp_single_pixel_buffer_manager_v1::WpSinglePixelBufferManagerV1;
use wayland_protocols::wp::viewporter::client::{
    wp_viewport::WpViewport, wp_viewporter::WpViewporter,
};
use wayland_protocols::xdg::activation::v1::client::xdg_activation_v1::XdgActivationV1;
use wayland_protocols::xdg::xdg_output::zv1::client::{
    zxdg_output_manager_v1::ZxdgOutputManagerV1,
    zxdg_output_v1::{self, ZxdgOutputV1},
};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{self, Anchor, KeyboardInteractivity, ZwlrLayerSurfaceV1},
};

thread_local! {
    /// The overlay connection, once [`start`] chose the layer path.
    static SHARED: RefCell<Option<Shared>> = const { RefCell::new(None) };
    /// Interface names the compositor advertised at start.
    static GLOBALS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// The adapter whose surfaces are mapped; it gets the events.
    static OWNER: RefCell<Weak<LayerAdapter>> = const { RefCell::new(Weak::new()) };
    /// What the next show lays out, and whether it waits for a drag.
    static SHOW: RefCell<ShowRequest> = RefCell::new(ShowRequest::default());
    /// The surface choice and reason last printed (S-019 AC2).
    static LAST_CHOICE: RefCell<Option<(Surface, String)>> = const { RefCell::new(None) };
    /// The last pointer position seen on the catcher, in global logical
    /// coordinates, and the origin of the catcher's output (S-104). A
    /// re-show of a held key hole-punches there instead of at the stale
    /// position of the first press.
    static LAST_POINTER: Cell<Option<(f64, f64)>> = const { Cell::new(None) };
    static OUTPUT_ORIGIN: Cell<(i32, i32)> = const { Cell::new((0, 0)) };
    /// The [`after_catcher_ready`] hook, taken once.
    static AFTER_CATCHER: RefCell<Option<Box<dyn FnOnce()>>> = const { RefCell::new(None) };
    /// The shown hive's opening, run after its next drawn frame.
    static OPENER: RefCell<Option<Box<dyn FnOnce()>>> = const { RefCell::new(None) };
}

/// Measurement hook (S-021 AC8): runs `f` once, right after the next
/// show's catcher is ready, that is its first commit after the first
/// configure of its layer surface. A drop show draws nothing until a drag
/// enters, so the bench times this instead of a first frame.
#[doc(hidden)]
pub fn after_catcher_ready(f: impl FnOnce() + 'static) {
    AFTER_CATCHER.with(|h| *h.borrow_mut() = Some(Box::new(f)));
}

/// Runs the [`after_catcher_ready`] hook, if one is set.
fn catcher_ready() {
    if let Some(f) = AFTER_CATCHER.with(|h| h.borrow_mut().take()) {
        f();
    }
}

/// How long a show waits for the pointer before it centres the hive
/// (S-019 AC5, AC8).
const ANCHOR_WAIT: Duration = Duration::from_millis(50);

/// How long a centred drop show's catcher keeps its full input region
/// after the placement, waiting for a position; then it takes the hive
/// rectangle and the hive stays centred (S-019 manager decision 12).
const MOVE_WINDOW: Duration = Duration::from_millis(300);

/// Half the side of the square an unrevealed `--at` catcher leaves out of its
/// input region around the pointer (S-104). Hyprland 0.57 sends a
/// `wl_pointer.enter` to a layer surface that maps under a still pointer and
/// the surface then also takes the next button press; with the pointer in a
/// hole there is no such enter, and a press still reaches the file manager.
/// Smaller than any drag threshold, so a real drag leaves the hole first.
const PROBE_HOLE_HALF: i32 = 3;

/// A `wl_pointer.enter` this soon after the catcher mapped is the compositor
/// refocusing under a still pointer, not a pointer that moved (S-104).
const MAP_ENTER_WINDOW: Duration = Duration::from_millis(250);

/// How far the pointer must leave a map time enter before it counts as a
/// move, in surface pixels (fractional scale rounds 40 to 39.75).
const MOVE_TOLERANCE: f64 = 3.0;

/// The hole of an unrevealed `--at` catcher: the square around `anchor`,
/// cut to `rect` (x, y, w, h). None when it does not touch the rectangle.
fn probe_hole(rect: (i32, i32, i32, i32), anchor: (f32, f32)) -> Option<(i32, i32, i32, i32)> {
    let (rx, ry, rw, rh) = rect;
    let (ax, ay) = (anchor.0.round() as i32, anchor.1.round() as i32);
    let x0 = (ax - PROBE_HOLE_HALF).max(rx);
    let y0 = (ay - PROBE_HOLE_HALF).max(ry);
    let x1 = (ax + PROBE_HOLE_HALF).min(rx + rw);
    let y1 = (ay + PROBE_HOLE_HALF).min(ry + rh);
    (x1 > x0 && y1 > y0).then_some((x0, y0, x1 - x0, y1 - y0))
}

/// What the catcher's pointer events of one show have been (S-104): the
/// pure part of the `wl_pointer` enter and motion decisions.
#[derive(Debug, Default)]
pub(crate) struct ProbeState {
    entered_once: bool,
    baseline: Option<(f64, f64)>,
}

/// What the adapter is told about a pointer event on the catcher.
#[derive(Debug, PartialEq)]
pub(crate) enum Probe {
    /// The compositor's map time enter: placement only, no hide.
    MapEnter(f64, f64),
    /// A real pointer arrived: no drag holds it.
    PointerEnter,
    Position(f64, f64),
}

impl From<Probe> for Ev {
    fn from(p: Probe) -> Ev {
        match p {
            Probe::MapEnter(x, y) => Ev::MapEnter(x, y),
            Probe::PointerEnter => Ev::PointerEnter,
            Probe::Position(x, y) => Ev::Position(x, y),
        }
    }
}

/// Where a drop show without `--at` remaps its catcher (S-105): the map time
/// enter plus the output origin, once per show. Hyprland 0.57 enters the
/// catcher under a still pointer and keeps the focus there when the region
/// shrinks, so the catcher is replaced by the `--at` form, which has a hole.
pub(crate) fn remap_at(
    drop: bool,
    had_at: bool,
    revealed: bool,
    remapped: bool,
    map_enter: Option<(f64, f64)>,
    origin: (i32, i32),
) -> Option<(f64, f64)> {
    if !drop || had_at || revealed || remapped {
        return None;
    }
    let (x, y) = map_enter?;
    Some((x + f64::from(origin.0), y + f64::from(origin.1)))
}

impl ProbeState {
    /// `wl_pointer.enter` on the catcher `since_map` after it mapped.
    pub(crate) fn enter(&mut self, since_map: Option<Duration>, pos: (f64, f64)) -> Vec<Probe> {
        let map_time = since_map.is_some_and(|t| is_map_time_enter(t, self.entered_once));
        self.entered_once = true;
        if map_time {
            self.baseline = Some(pos);
            vec![Probe::MapEnter(pos.0, pos.1)]
        } else {
            self.baseline = None;
            vec![Probe::PointerEnter, Probe::Position(pos.0, pos.1)]
        }
    }

    /// Motion on the catcher.
    pub(crate) fn motion(&mut self, pos: (f64, f64)) -> Vec<Probe> {
        let mut out = Vec::new();
        if let Some(b) = self.baseline
            && moved_from(b, pos)
        {
            self.baseline = None;
            out.push(Probe::PointerEnter);
        }
        out.push(Probe::Position(pos.0, pos.1));
        out
    }

    /// `wl_pointer.leave` from the catcher: the next enter is a real one.
    pub(crate) fn leave(&mut self) {
        self.baseline = None;
    }
}

/// The hole of the catcher's input region: only while the hive is not
/// revealed, so a reveal takes the whole hive rectangle for the drag.
fn region_hole(
    revealed: bool,
    anchor: Option<(f32, f32)>,
    rect: (i32, i32, i32, i32),
) -> Option<(i32, i32, i32, i32)> {
    if revealed {
        return None;
    }
    probe_hole(rect, anchor?)
}

/// Remembers the pointer at catcher coordinates (x, y) in global ones.
fn note_pointer(x: f64, y: f64) {
    let (ox, oy) = OUTPUT_ORIGIN.with(Cell::get);
    LAST_POINTER.with(|p| p.set(Some((x + f64::from(ox), y + f64::from(oy)))));
}

/// The last pointer position the catcher saw before it hid, in global
/// logical coordinates (S-104). The pointer may have moved on since, so a
/// re-show's hole can miss it; that is harmless, the map time enter is
/// ignored anyway.
pub(crate) fn last_pointer() -> Option<(f64, f64)> {
    LAST_POINTER.with(Cell::get)
}

/// Whether an enter on the catcher is the compositor's map time refocus: the
/// first one, within [`MAP_ENTER_WINDOW`] of the map.
fn is_map_time_enter(since_map: Duration, entered_before: bool) -> bool {
    !entered_before && since_map <= MAP_ENTER_WINDOW
}

/// Whether the pointer left the position of a map time enter.
fn moved_from(baseline: (f64, f64), now: (f64, f64)) -> bool {
    (now.0 - baseline.0).hypot(now.1 - baseline.1) >= MOVE_TOLERANCE
}

/// The cells of the next show and its kind, so the hive can be laid out
/// again at the pointer (S-019c, S-019d).
#[derive(Default)]
pub(crate) struct ShowRequest {
    pub cells: Vec<HiveCell>,
    pub rtl: bool,
    pub min_rings: i32,
    /// A drop show (S-014): no keyboard, the catcher takes the drag.
    pub drop: bool,
    /// `kawaro drop --at X,Y`, global logical coordinates.
    pub at: Option<(f64, f64)>,
}

pub(crate) fn set_show(r: ShowRequest) {
    SHOW.with(|s| *s.borrow_mut() = r);
}

/// A position from the drop receiver in the anchor phase (catcher
/// coordinates).
pub(crate) fn position(x: f64, y: f64) {
    if let Some(a) = OWNER.with(|o| o.borrow().upgrade()) {
        a.handle(Ev::Position(x, y));
    }
}

/// The drop hive's plan for `cells` at the place and in the box it took at
/// the pointer; `None` off the layer path or before it is placed.
pub(crate) fn replan(cells: &[HiveCell]) -> Option<Plan> {
    let a = OWNER.with(|o| o.borrow().upgrade())?;
    if !a.decided.get() {
        return None;
    }
    let rtl = SHOW.with(|s| s.borrow().rtl);
    let (plan, _) = layout::plan_at(
        cells.to_vec(),
        rtl,
        a.rings.get(),
        a.screen.get(),
        a.anchor.get(),
        a.scale120.get(),
    );
    Some(plan)
}

/// Ends the probe of the shown drop show (S-021): `from` and `to` are the
/// mode machine's states before and after `event`. A reveal prints `hive
/// reveal: N`, with `cells` the cells the drag brought, and draws the hive;
/// a hide prints `hive probe: hidden (R)` before the catcher goes. Escape
/// and Cancel never reach a catcher before its enter, so print nothing.
pub(crate) fn probed(from: dnd::State, to: dnd::State, event: dnd::Event, cells: usize) {
    if from != dnd::State::Probing || to == from {
        return;
    }
    let reason = match (to, event) {
        (dnd::State::Waiting, _) => None,
        (_, dnd::Event::PointerEnter) => Some("pointer"),
        (_, dnd::Event::Reject) if crate::receiver::offered_uri_list() => Some("no-actions"),
        (_, dnd::Event::Reject) => Some("no-uri-list"),
        (_, dnd::Event::Release) => Some("release"),
        (_, dnd::Event::Tick) => Some("cap"),
        _ => return,
    };
    if std::env::var_os("KAWARO_DEBUG_FOCUS").is_some() {
        crate::stderr_line(&match reason {
            Some(r) => format!("hive probe: hidden ({r})"),
            None => format!("hive reveal: {cells}"),
        });
    }
    if reason.is_none()
        && let Some(a) = OWNER.with(|o| o.borrow().upgrade())
    {
        a.revealed.set(true);
        // The hole of the probe goes: the whole hive rectangle takes the drag.
        a.set_region();
        if let Some(s) = a.surfaces.borrow().as_ref() {
            s.catcher.commit();
        }
        a.request_redraw();
    }
}

/// Runs `f` once the shown hive has drawn a frame, so the opening
/// animation it starts runs from that frame (S-022 AC4): Slint animates a
/// property only from a value it has drawn. A hide drops it.
pub(crate) fn after_drawn(f: impl FnOnce() + 'static) {
    OPENER.with(|o| *o.borrow_mut() = Some(Box::new(f)));
}

/// The shown hive was replaced by a new show: a probing one prints `hive
/// probe: hidden (replaced)` (S-022), any other nothing.
pub(crate) fn replaced(reason: Option<&str>) {
    if let Some(r) = reason
        && std::env::var_os("KAWARO_DEBUG_FOCUS").is_some()
    {
        crate::stderr_line(&format!("hive probe: hidden ({r})"));
    }
}

/// The rings a drop hive at `anchor` sizes for: those 18 cells would use,
/// at least 2. Its cells come only with the drag, so the box is fixed for
/// the most a drag can bring and does not move or grow at enter (S-019d,
/// implementer decision).
fn drop_rings(rtl: bool, screen: (u32, u32), anchor: Option<(f32, f32)>) -> i32 {
    let cells = (0..18)
        .map(|i| HiveCell {
            id: String::new(),
            code: String::new(),
            label: String::new(),
            icon: String::new(),
            ring: if i < 6 { 1 } else { 2 },
            enabled: true,
            reason: String::new(),
        })
        .collect();
    let (plan, _) = layout::plan_at(cells, rtl, 2, screen, anchor, 120);
    plan.cells
        .iter()
        .map(|c| {
            c.axial
                .q
                .abs()
                .max(c.axial.r.abs())
                .max((c.axial.q + c.axial.r).abs())
        })
        .fold(2, i32::max)
}

/// An output with its place in the global logical space (xdg-output).
struct Output {
    name: u32,
    wl: WlOutput,
    xdg: Option<ZxdgOutputV1>,
    origin: (i32, i32),
    size: (i32, i32),
}

struct Shared {
    conn: Connection,
    queue: EventQueue<Wl>,
    wl: Wl,
    pending: Arc<AtomicBool>,
}

/// Bound globals, input devices and what the last dispatch saw.
struct Wl {
    qh: QueueHandle<Wl>,
    compositor: WlCompositor,
    subcompositor: WlSubcompositor,
    shm: WlShm,
    layer_shell: ZwlrLayerShellV1,
    viewporter: WpViewporter,
    single_pixel: WpSinglePixelBufferManagerV1,
    fractional: Option<WpFractionalScaleManagerV1>,
    activation: Option<XdgActivationV1>,
    cursor_shape: Option<WpCursorShapeManagerV1>,
    xdg_outputs: Option<ZxdgOutputManagerV1>,
    outputs: Vec<Output>,
    pointer: Option<WlPointer>,
    /// Sets the default arrow on each enter (S-018 AC16).
    cursor: Option<WpCursorShapeDeviceV1>,
    keyboard: Option<WlKeyboard>,
    catcher: Option<WlSurface>,
    hive: Option<WlSurface>,
    pointer_on_hive: bool,
    pointer_on_catcher: bool,
    /// When the catcher was created, and whether its pointer enter came
    /// already; the position of a map time enter (S-104).
    map_at: Option<Instant>,
    probe: ProbeState,
    shift: bool,
    events: Vec<Ev>,
    /// Buffers of the shown hive the compositor has not released yet,
    /// destroyed on release or, at the latest, on hide (S-018 AC18).
    buffers: Vec<WlBuffer>,
}

/// What the Wayland side saw, handed to the shown adapter after dispatching.
enum Ev {
    Configured(u32, u32),
    Closed,
    Scale(u32),
    FrameDone,
    Motion(f64, f64),
    /// The pointer or a drag on the catcher, in its coordinates.
    Position(f64, f64),
    /// `wl_pointer.enter` on the catcher: no drag holds the pointer.
    PointerEnter,
    /// The compositor's map time enter (S-104): the pointer is here but did
    /// not move. It positions a normal show; a drop show ignores it.
    MapEnter(f64, f64),
    Leave,
    Button {
        pressed: bool,
        on_hive: bool,
    },
    Key {
        code: u32,
        pressed: bool,
    },
    Focus(bool),
    Shift(bool),
}

/// Connects to the compositor and, when [`overlay::choose_surface`] says
/// `Layer`, keeps the connection for the hive. False keeps today's window.
pub(crate) fn start() -> bool {
    let Ok(conn) = Connection::connect_to_env() else {
        return false;
    };
    let Ok((globals, mut queue)) = registry_queue_init::<Wl>(&conn) else {
        return false;
    };
    let names: Vec<String> = globals
        .contents()
        .clone_list()
        .into_iter()
        .map(|g| g.interface)
        .collect();
    GLOBALS.with(|g| *g.borrow_mut() = names);
    // The overlay is possible here; a screen reader only makes some shows
    // windows, decided at every build (S-018 AC12). When it is not, no build
    // decides again, so this decision prints its choice line.
    let (surface, reason) = choice(false, false);
    if surface != Surface::Layer {
        report_choice(surface, reason);
        return false;
    }
    crate::a11y::watch();
    let Some(mut wl) = bind(&globals, &queue.handle()) else {
        return false;
    };
    // Seat capabilities, then the pointer and keyboard they bring.
    if queue.roundtrip(&mut wl).is_err() || queue.roundtrip(&mut wl).is_err() {
        return false;
    }
    let pending = Arc::new(AtomicBool::new(false));
    {
        let (conn, pending) = (conn.clone(), pending.clone());
        std::thread::spawn(move || reader(conn, pending));
    }
    SHARED.with(|s| {
        *s.borrow_mut() = Some(Shared {
            conn,
            queue,
            wl,
            pending,
        })
    });
    true
}

/// The surface decision for the advertised globals, `KAWARO_SURFACE`,
/// `hive_surface` in config.toml (read again at every build) and the
/// screen reader state (S-019 AC1).
fn choice(flag: bool, reader: bool) -> (Surface, String) {
    let forced = std::env::var("KAWARO_SURFACE").ok();
    let config = kawaro_core::config::Config::load().0.hive_surface;
    GLOBALS.with(|g| {
        let g = g.borrow();
        let names: Vec<&str> = g.iter().map(String::as_str).collect();
        overlay::surface_choice(&names, true, flag, reader, config, forced.as_deref())
    })
}

/// Prints the `hive surface choice:` line when the choice or its reason
/// differs from the last one printed (S-019 AC2), with no environment
/// variable.
fn report_choice(surface: Surface, reason: String) {
    LAST_CHOICE.with(|last| {
        let mut last = last.borrow_mut();
        if last.as_ref() != Some(&(surface, reason.clone())) {
            crate::stderr_line(&overlay::choice_line(surface, &reason));
            *last = Some((surface, reason));
        }
    });
}

/// The next hive build goes on the overlay (decided again at every build).
pub(crate) fn wanted() -> bool {
    if SHARED.with(|s| s.borrow().is_none()) {
        return false;
    }
    let (surface, reason) = choice(crate::a11y::screen_reader(), crate::a11y::reader());
    report_choice(surface, reason);
    surface == Surface::Layer
}

fn bind(globals: &GlobalList, qh: &QueueHandle<Wl>) -> Option<Wl> {
    let _seat: WlSeat = globals.bind(qh, 1..=7, ()).ok()?;
    let shapes = GLOBALS.with(|g| {
        let g = g.borrow();
        overlay::wants_cursor_shape(&g.iter().map(String::as_str).collect::<Vec<_>>())
    });
    let xdg_outputs: Option<ZxdgOutputManagerV1> = globals.bind(qh, 1..=3, ()).ok();
    let mut outputs = Vec::new();
    for g in globals.contents().clone_list() {
        if g.interface == "wl_output" {
            outputs.push(new_output(
                globals.registry(),
                g.name,
                g.version,
                xdg_outputs.as_ref(),
                qh,
            ));
        }
    }
    Some(Wl {
        xdg_outputs,
        outputs,
        pointer_on_catcher: false,
        map_at: None,
        probe: ProbeState::default(),
        qh: qh.clone(),
        compositor: globals.bind(qh, 4..=6, ()).ok()?,
        subcompositor: globals.bind(qh, 1..=1, ()).ok()?,
        shm: globals.bind(qh, 1..=1, ()).ok()?,
        layer_shell: globals.bind(qh, 1..=4, ()).ok()?,
        viewporter: globals.bind(qh, 1..=1, ()).ok()?,
        single_pixel: globals.bind(qh, 1..=1, ()).ok()?,
        fractional: globals.bind(qh, 1..=1, ()).ok(),
        activation: globals.bind(qh, 1..=1, ()).ok(),
        cursor_shape: shapes.then(|| globals.bind(qh, 1..=1, ()).ok()).flatten(),
        pointer: None,
        cursor: None,
        keyboard: None,
        catcher: None,
        hive: None,
        pointer_on_hive: false,
        shift: false,
        events: Vec::new(),
        buffers: Vec::new(),
    })
}

fn new_output(
    registry: &WlRegistry,
    name: u32,
    version: u32,
    manager: Option<&ZxdgOutputManagerV1>,
    qh: &QueueHandle<Wl>,
) -> Output {
    let wl: WlOutput = registry.bind(name, version.min(4), qh, ());
    let xdg = manager.map(|m| m.get_xdg_output(&wl, qh, name));
    Output {
        name,
        wl,
        xdg,
        origin: (0, 0),
        size: (0, 0),
    }
}

/// An output with its logical origin and size.
type Target = (WlOutput, (i32, i32), (u32, u32));

/// The output holding the global point `at`: the output, its origin and its
/// logical size.
fn output_at(at: (f64, f64)) -> Option<Target> {
    SHARED.with(|s| {
        let s = s.borrow();
        s.as_ref()?.wl.outputs.iter().find_map(|o| {
            let (x, y) = (at.0 - f64::from(o.origin.0), at.1 - f64::from(o.origin.1));
            let inside = o.size.0 > 0
                && o.size.1 > 0
                && (0.0..f64::from(o.size.0)).contains(&x)
                && (0.0..f64::from(o.size.1)).contains(&y);
            inside.then(|| (o.wl.clone(), o.origin, (o.size.0 as u32, o.size.1 as u32)))
        })
    })
}

/// Blocks on the socket off the Slint thread and wakes it to dispatch.
fn reader(conn: Connection, pending: Arc<AtomicBool>) {
    loop {
        let Some(guard) = conn.prepare_read() else {
            // Events wait in the queue for the Slint thread, maybe read by
            // the drop receiver's thread on this display (S-018d).
            if !pending.swap(true, Ordering::SeqCst)
                && slint::invoke_from_event_loop(dispatch).is_err()
            {
                return;
            }
            std::thread::sleep(Duration::from_millis(1));
            continue;
        };
        {
            let fd = guard.connection_fd();
            let mut fds = [PollFd::new(&fd, PollFlags::IN)];
            if poll(&mut fds, None).is_err() {
                return;
            }
        }
        if guard.read().is_err() {
            return;
        }
        if !pending.swap(true, Ordering::SeqCst) && slint::invoke_from_event_loop(dispatch).is_err()
        {
            return;
        }
    }
}

/// Dispatches what the reader read and hands it to the shown adapter.
fn dispatch() {
    let events = SHARED.with(|s| {
        let mut s = s.borrow_mut();
        let Some(Shared {
            queue, wl, pending, ..
        }) = s.as_mut()
        else {
            return Vec::new();
        };
        pending.store(false, Ordering::SeqCst);
        let _ = queue.dispatch_pending(wl);
        std::mem::take(&mut wl.events)
    });
    if let Some(a) = OWNER.with(|o| o.borrow().upgrade()) {
        for e in events {
            a.handle(e);
        }
    }
    flush();
}

fn flush() {
    let _ = SHARED.try_with(|s| {
        if let Some(s) = s.borrow().as_ref() {
            let _ = s.conn.flush();
        }
    });
}

/// wl_shm ARGB8888 is little endian: bytes B, G, R, A, premultiplied.
#[derive(Clone, Copy, Default)]
struct Bgra {
    b: u8,
    g: u8,
    r: u8,
    a: u8,
}

impl TargetPixel for Bgra {
    fn blend(&mut self, c: PremultipliedRgbaColor) {
        let keep = u16::from(255 - c.alpha);
        let mix = |d: u8, s: u8| (u16::from(d) * keep / 255) as u8 + s;
        self.r = mix(self.r, c.red);
        self.g = mix(self.g, c.green);
        self.b = mix(self.b, c.blue);
        self.a = mix(self.a, c.alpha);
    }
    fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { b, g, r, a: 255 }
    }
    fn background() -> Self {
        Self::default()
    }
}

struct Surfaces {
    catcher: WlSurface,
    layer: ZwlrLayerSurfaceV1,
    catcher_vp: WpViewport,
    hive: WlSurface,
    sub: WlSubsurface,
    hive_vp: WpViewport,
    frac: Option<WpFractionalScaleV1>,
    /// Output size from the last configure.
    configured: Option<(u32, u32)>,
    /// A drop show: the catcher's input region is the hive (S-020).
    drop_mode: bool,
}

pub(crate) struct LayerAdapter {
    me: Weak<LayerAdapter>,
    window: slint::Window,
    renderer: SoftwareRenderer,
    surfaces: RefCell<Option<Surfaces>>,
    /// Hive size in whole logical pixels (S-018 AC2).
    logical: Cell<(u32, u32)>,
    /// Scale in 120ths (fractional-scale-v1).
    scale120: Cell<u32>,
    pointer: Cell<(f32, f32)>,
    waiting_frame: Cell<bool>,
    redraw_wanted: Cell<bool>,
    render_scheduled: Cell<bool>,
    /// Activation token handed over with this build (S-013), used once.
    token: RefCell<Option<String>>,
    /// The token of the shown catcher, sent with its first frame: KWin
    /// ignores an activation of a layer surface that is not mapped yet.
    activation: RefCell<Option<String>>,
    /// The `hive surface` line of this placement was printed (S-018 AC11).
    reported: Cell<bool>,
    /// The hive of this show is placed (anchored or centred).
    decided: Cell<bool>,
    /// A drop show centred without a position: it moves once, at the first.
    may_move: Cell<bool>,
    /// The drop show's catcher takes only the hive rectangle (S-020 AC1).
    switched: Cell<bool>,
    /// The hive is drawn: always outside drop mode, after [`probed`] in it.
    revealed: Cell<bool>,
    /// The pointer entered the catcher of a drop show not revealed: no
    /// drag, so the show hides and is never placed after it (S-021 AC3).
    no_drag: Cell<bool>,
    /// The catcher of this show was already remapped at its map time enter
    /// (S-105).
    remapped: Cell<bool>,
    /// The anchor the hive was placed at, the output size and the rings
    /// its box is sized for.
    anchor: Cell<Option<(f32, f32)>>,
    screen: Cell<(u32, u32)>,
    rings: Cell<i32>,
    /// Subsurface position from the plan, and the last one sent.
    pos: Cell<Option<(i32, i32)>>,
    sent: Cell<Option<(i32, i32)>>,
    /// Ends the wait for the anchor.
    timer: slint::Timer,
}

impl LayerAdapter {
    pub(crate) fn create() -> Rc<dyn WindowAdapter> {
        let token = crate::PENDING_TOKEN.lock().ok().and_then(|mut t| t.take());
        Rc::new_cyclic(|me: &Weak<Self>| Self {
            me: me.clone(),
            window: slint::Window::new(me.clone() as Weak<dyn WindowAdapter>),
            renderer: SoftwareRenderer::new_with_repaint_buffer_type(RepaintBufferType::NewBuffer),
            surfaces: RefCell::new(None),
            logical: Cell::new((1, 1)),
            scale120: Cell::new(120),
            pointer: Cell::new((0.0, 0.0)),
            waiting_frame: Cell::new(false),
            redraw_wanted: Cell::new(false),
            render_scheduled: Cell::new(false),
            token: RefCell::new(token),
            activation: RefCell::new(None),
            reported: Cell::new(false),
            decided: Cell::new(false),
            may_move: Cell::new(false),
            switched: Cell::new(false),
            revealed: Cell::new(true),
            no_drag: Cell::new(false),
            remapped: Cell::new(false),
            anchor: Cell::new(None),
            screen: Cell::new((0, 0)),
            rings: Cell::new(1),
            pos: Cell::new(None),
            sent: Cell::new(None),
            timer: slint::Timer::default(),
        })
    }

    fn placement(&self, screen: (u32, u32)) -> overlay::Placement {
        let p = overlay::place(screen, self.logical.get(), self.scale120.get());
        match self.pos.get() {
            Some((x, y)) => overlay::Placement { x, y, ..p },
            None => p,
        }
    }

    /// Places the hive at `anchor` (or centred) on the configured output.
    fn decide(&self, anchor: Option<(f32, f32)>) {
        let Some(screen) = self.surfaces.borrow().as_ref().and_then(|s| s.configured) else {
            return;
        };
        if self.no_drag.get() {
            return;
        }
        let drop_mode = SHOW.with(|r| r.borrow().drop);
        self.timer.stop();
        self.decided.set(true);
        self.may_move.set(drop_mode && anchor.is_none());
        self.plan_for(anchor, screen);
        if drop_mode && anchor.is_some() {
            self.switched.set(true);
        }
        self.place();
        if let Some(s) = self.surfaces.borrow().as_ref() {
            s.catcher.commit();
        }
        self.render();
        if self.switched.get() {
            self.release_held();
        } else if self.may_move.get() {
            // Centred without a position: the full region lasts at most
            // MOVE_WINDOW from this placement, so a still pointer is never
            // held (S-020 AC4).
            let me = self.me.clone();
            self.timer
                .start(slint::TimerMode::SingleShot, MOVE_WINDOW, move || {
                    if let Some(a) = me.upgrade() {
                        a.keep_centred();
                    }
                });
        }
    }

    /// Ends the move window of a centred drop show: no move any more, and
    /// the catcher takes only the hive rectangle.
    fn keep_centred(&self) {
        if !self.may_move.replace(false) {
            return;
        }
        self.switched.set(true);
        self.set_region();
        if let Some(s) = self.surfaces.borrow().as_ref() {
            s.catcher.commit();
        }
        self.release_held();
    }

    /// The hive rectangle is on the wire; now the held drag enter.
    fn release_held(&self) {
        flush();
        crate::receiver::set_anchoring(false);
        crate::receiver::replay();
    }

    /// Lays the shown hive out at `anchor` on a `screen` sized output with
    /// `plan_at`, prints the anchor and cells lines, and takes its size and
    /// position.
    fn plan_for(&self, anchor: Option<(f32, f32)>, screen: (u32, u32)) {
        if std::env::var_os("KAWARO_DEBUG_FOCUS").is_some() {
            match anchor {
                Some((x, y)) => crate::stderr_line(&format!("hive anchor: {x} {y}")),
                None => crate::stderr_line("hive anchor: none"),
            }
        }
        let (plan, p) = SHOW.with(|r| {
            let r = r.borrow();
            let rings = if r.drop && anchor.is_some() {
                drop_rings(r.rtl, screen, anchor)
            } else {
                r.min_rings
            };
            self.rings.set(rings);
            layout::plan_at(
                r.cells.clone(),
                r.rtl,
                rings,
                screen,
                anchor,
                self.scale120.get(),
            )
        });
        if std::env::var_os("KAWARO_DEBUG_FOCUS").is_some() {
            let (cx, cy) = plan.layout.origin;
            crate::stderr_line(&format!("hive cells: {} {cx} {cy}", plan.cells.len()));
        }
        self.anchor.set(anchor);
        self.screen.set(screen);
        self.pos.set(Some((p.x, p.y)));
        self.reported.set(false);
        let (w, h) = (plan.width, plan.height);
        crate::set_shown_plan(plan);
        self.set_logical(w, h);
    }

    /// Gives the catcher its stretched transparent pixel.
    fn attach_pixel(&self, s: &Surfaces, sh: &mut Shared, (w, h): (u32, u32)) {
        let pixel = sh
            .wl
            .single_pixel
            .create_u32_rgba_buffer(0, 0, 0, 0, &sh.wl.qh, ());
        s.catcher_vp.set_destination(w as i32, h as i32);
        s.catcher.attach(Some(&pixel), 0, 0);
        s.catcher.damage_buffer(0, 0, i32::MAX, i32::MAX);
        sh.wl.buffers.push(pixel);
    }

    fn handle(&self, e: Ev) {
        let position = || {
            let (x, y) = self.pointer.get();
            LogicalPosition::new(x, y)
        };
        match e {
            Ev::Configured(w, h) => {
                let first = match self.surfaces.borrow_mut().as_mut() {
                    Some(s) => s.configured.replace((w, h)).is_none(),
                    None => return,
                };
                if !self.decided.get() && first {
                    self.wait_for_anchor((w, h));
                } else {
                    self.place();
                    self.render();
                }
                // Both branches committed the catcher and flushed.
                if first {
                    catcher_ready();
                }
            }
            Ev::MapEnter(x, y) => {
                note_pointer(x, y);
                let (drop, had_at) = SHOW.with(|r| {
                    let r = r.borrow();
                    (r.drop, r.at.is_some())
                });
                if !drop {
                    self.handle(Ev::Position(x, y));
                }
                let origin = OUTPUT_ORIGIN.with(Cell::get);
                let revealed = self.revealed.get();
                let enter = Some((x, y));
                if let Some(pt) =
                    remap_at(drop, had_at, revealed, self.remapped.get(), enter, origin)
                {
                    self.remapped.set(true);
                    let me = self.me.clone();
                    slint::Timer::single_shot(Duration::ZERO, move || {
                        if let Some(a) = me.upgrade() {
                            a.remap(pt);
                        }
                    });
                }
            }
            Ev::Position(x, y) => {
                note_pointer(x, y);
                let configured = self
                    .surfaces
                    .borrow()
                    .as_ref()
                    .is_some_and(|s| s.configured.is_some());
                if configured && (!self.decided.get() || self.may_move.replace(false)) {
                    self.decide(Some((x as f32, y as f32)));
                }
            }
            // A drop show's probe decides on it (S-021); this only forwards
            // it to the receiver's sink, on the next loop turn like its notes.
            // Not revealed, the show is not placed at this enter.
            Ev::PointerEnter => {
                if self.surfaces.borrow().as_ref().is_some_and(|s| s.drop_mode) {
                    if !self.revealed.get() {
                        self.no_drag.set(true);
                        self.timer.stop();
                    }
                    let note = || crate::on_note(crate::receiver::Note::PointerEnter);
                    let _ = slint::invoke_from_event_loop(note);
                }
            }
            Ev::Closed => self.window.dispatch_event(WindowEvent::CloseRequested),
            Ev::Scale(s) => {
                if s != self.scale120.get() && s > 0 {
                    self.scale120.set(s);
                    self.window.dispatch_event(WindowEvent::ScaleFactorChanged {
                        scale_factor: s as f32 / 120.0,
                    });
                    self.place();
                    self.request_redraw();
                }
            }
            Ev::FrameDone => {
                self.waiting_frame.set(false);
                if self.redraw_wanted.get() || self.window.has_active_animations() {
                    self.render();
                }
            }
            Ev::Motion(x, y) => {
                self.pointer.set((x as f32, y as f32));
                self.window.dispatch_event(WindowEvent::PointerMoved {
                    position: position(),
                });
            }
            Ev::Leave => self.window.dispatch_event(WindowEvent::PointerExited),
            Ev::Button {
                pressed,
                on_hive: true,
            } => {
                let button = PointerEventButton::Left;
                let position = position();
                self.window.dispatch_event(if pressed {
                    WindowEvent::PointerPressed { position, button }
                } else {
                    WindowEvent::PointerReleased { position, button }
                });
            }
            // A click on the catcher, outside the hive; in drop mode the
            // catcher may cover the hive, and a click never closes it
            // (S-019 AC8).
            Ev::Button {
                pressed: true,
                on_hive: false,
            } if !self.surfaces.borrow().as_ref().is_some_and(|s| s.drop_mode) => {
                self.window.dispatch_event(WindowEvent::CloseRequested)
            }
            Ev::Button { .. } => {}
            Ev::Key { code, pressed } => {
                if let Some(k) = overlay::key_for_evdev(code) {
                    self.key(k, pressed);
                }
            }
            Ev::Shift(down) => self.key(Key::Shift, down),
            Ev::Focus(f) => {
                // Observation hook for tests/focus, as on the window path.
                if std::env::var_os("KAWARO_DEBUG_FOCUS").is_some() {
                    crate::stderr_line(&format!("hive focus: {f}"));
                }
                // Focus loss does not close the hive on this path (S-018,
                // manager decision 1), so `focus_changed` is not told.
                self.window
                    .dispatch_event(WindowEvent::WindowActiveChanged(f));
            }
        }
    }

    fn key(&self, k: Key, pressed: bool) {
        let text: slint::SharedString = k.into();
        self.window.dispatch_event(if pressed {
            WindowEvent::KeyPressed { text }
        } else {
            WindowEvent::KeyReleased { text }
        });
    }

    /// Maps the catcher (a transparent pixel) so the pointer can enter it,
    /// and waits at most [`ANCHOR_WAIT`] for the anchor; `--at` places the
    /// hive at once.
    fn wait_for_anchor(&self, screen: (u32, u32)) {
        let at = SHOW.with(|r| r.borrow().at);
        if let Some((x, y)) = at {
            // No output geometry at map: the point as output coordinates.
            self.decide(Some((x as f32, y as f32)));
            return;
        }
        {
            let surfaces = self.surfaces.borrow();
            let Some(s) = surfaces.as_ref() else { return };
            SHARED.with(|sh| {
                if let Some(sh) = sh.borrow_mut().as_mut() {
                    self.attach_pixel(s, sh, screen);
                }
            });
            s.catcher.commit();
        }
        flush();
        let me = self.me.clone();
        self.timer
            .start(slint::TimerMode::SingleShot, ANCHOR_WAIT, move || {
                if let Some(a) = me.upgrade()
                    && !a.decided.get()
                {
                    a.decide(None);
                }
            });
    }

    /// Replaces the catcher by the `--at` form at `pt` (S-105). The token and
    /// the opening of the show survive; the receiver follows the new catcher.
    fn remap(&self, pt: (f64, f64)) {
        if self.surfaces.borrow().is_none() || self.revealed.get() || self.no_drag.get() {
            return;
        }
        let token = self.activation.borrow_mut().take();
        let opener = OPENER.with(|o| o.borrow_mut().take());
        self.unmap();
        *self.token.borrow_mut() = token;
        OPENER.with(|o| *o.borrow_mut() = opener);
        SHOW.with(|r| r.borrow_mut().at = Some(pt));
        self.map(true);
    }

    /// Creates the catcher layer surface and the hive subsurface.
    fn map(&self, remap: bool) {
        let (drop_mode, at) = SHOW.with(|r| {
            let r = r.borrow();
            (r.drop, r.at)
        });
        *self.activation.borrow_mut() = self.token.borrow_mut().take();
        self.reported.set(false);
        self.decided.set(false);
        self.may_move.set(false);
        self.switched.set(false);
        self.revealed.set(!drop_mode);
        self.no_drag.set(false);
        self.remapped.set(remap);
        self.anchor.set(None);
        self.pos.set(None);
        self.sent.set(None);
        // `--at`: the output holding the point, and the point on it.
        let target = at.and_then(output_at);
        OUTPUT_ORIGIN.with(|o| o.set(target.as_ref().map_or((0, 0), |t| t.1)));
        LAST_POINTER.with(|p| p.set(None));
        if let (Some((_, origin, _)), Some((x, y))) = (&target, at) {
            SHOW.with(|r| {
                r.borrow_mut().at = Some((x - f64::from(origin.0), y - f64::from(origin.1)));
            });
        }
        crate::receiver::set_anchoring(drop_mode && at.is_none());
        let made = SHARED.with(|s| {
            let mut s = s.borrow_mut();
            let wl = &mut s.as_mut()?.wl;
            let qh = &wl.qh;
            let catcher = wl.compositor.create_surface(qh, ());
            let layer = wl.layer_shell.get_layer_surface(
                &catcher,
                target.as_ref().map(|t| &t.0),
                Layer::Overlay,
                "kawaro".into(),
                qh,
                (),
            );
            layer.set_anchor(Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right);
            layer.set_size(0, 0);
            layer.set_exclusive_zone(-1);
            // Before the catcher's first commit and never after it: KWin
            // at focus stealing prevention level 4 gives the keyboard to a
            // layer surface that asks for it once mapped (S-027).
            layer.set_keyboard_interactivity(if drop_mode {
                KeyboardInteractivity::None
            } else {
                KeyboardInteractivity::Exclusive
            });
            let catcher_vp = wl.viewporter.get_viewport(&catcher, qh, ());
            let hive = wl.compositor.create_surface(qh, ());
            if drop_mode {
                // The catcher takes the drag over the hive (S-020).
                let r: WlRegion = wl.compositor.create_region(qh, ());
                hive.set_input_region(Some(&r));
                r.destroy();
            }
            let sub = wl.subcompositor.get_subsurface(&hive, &catcher, qh, ());
            let hive_vp = wl.viewporter.get_viewport(&hive, qh, ());
            let frac = wl
                .fractional
                .as_ref()
                .map(|f| f.get_fractional_scale(&hive, qh, ()));
            let ids = (
                s.as_ref()?.conn.backend().display_ptr() as usize,
                catcher.id().as_ptr() as usize,
            );
            let wl = &mut s.as_mut()?.wl;
            wl.catcher = Some(catcher.clone());
            wl.hive = Some(hive.clone());
            wl.pointer_on_hive = false;
            wl.pointer_on_catcher = false;
            wl.map_at = Some(Instant::now());
            wl.probe = ProbeState::default();
            Some((
                ids,
                Surfaces {
                    catcher,
                    layer,
                    catcher_vp,
                    hive,
                    sub,
                    hive_vp,
                    frac,
                    configured: None,
                    drop_mode,
                },
            ))
        });
        if let Some(((display, catcher), surfaces)) = made {
            *self.surfaces.borrow_mut() = Some(surfaces);
            OWNER.with(|o| *o.borrow_mut() = self.me.clone());
            // `--at` on a known output: placed, and in drop mode the catcher
            // takes only the hive, before its first commit (S-019 AC8).
            if let (Some((_, _, size)), Some(local)) = (target, SHOW.with(|r| r.borrow().at)) {
                self.decided.set(true);
                self.plan_for(Some((local.0 as f32, local.1 as f32)), size);
                self.switched.set(drop_mode);
                self.set_region();
            }
            if let Some(s) = self.surfaces.borrow().as_ref() {
                s.catcher.commit();
            }
            if drop_mode {
                // The receiver's data device lives on this connection, the
                // only one the compositor names the catcher to.
                flush();
                if remap {
                    crate::receiver::attach(display, catcher, crate::on_note);
                } else {
                    crate::attach_layer_receiver(display, catcher);
                }
            }
        }
        self.waiting_frame.set(false);
        flush();
    }

    fn unmap(&self) {
        // A show closed before its first frame keeps no token nor opening
        // for the next.
        self.activation.borrow_mut().take();
        let _ = OPENER.try_with(|o| o.borrow_mut().take());
        self.timer.stop();
        let Some(s) = self.surfaces.borrow_mut().take() else {
            return;
        };
        if let Some(f) = s.frac {
            f.destroy();
        }
        s.hive_vp.destroy();
        s.sub.destroy();
        s.hive.destroy();
        s.catcher_vp.destroy();
        s.layer.destroy();
        s.catcher.destroy();
        // try_with: a hive dropped at thread exit may outlive these.
        let _ = SHARED.try_with(|sh| {
            if let Some(sh) = sh.borrow_mut().as_mut() {
                sh.wl.catcher = None;
                sh.wl.hive = None;
                sh.wl.pointer_on_hive = false;
                sh.wl.pointer_on_catcher = false;
                sh.wl.map_at = None;
                sh.wl.probe = ProbeState::default();
                // The surfaces are gone, so no buffer is in use any more:
                // free them now rather than on a later release.
                for b in sh.wl.buffers.drain(..) {
                    b.destroy();
                }
            }
        });
        let _ = OWNER.try_with(|o| {
            if std::ptr::eq(o.borrow().as_ptr(), self) {
                *o.borrow_mut() = Weak::new();
            }
        });
        self.waiting_frame.set(false);
        self.redraw_wanted.set(false);
        flush();
    }

    /// Gives the catcher its stretched transparent pixel and puts the hive
    /// where it was placed; in drop mode, once switched, the catcher's input
    /// region follows the hive.
    fn place(&self) {
        if !self.decided.get() {
            return;
        }
        let surfaces = self.surfaces.borrow();
        let Some(s) = surfaces.as_ref() else { return };
        let Some((w, h)) = s.configured else { return };
        let p = self.placement((w, h));
        SHARED.with(|sh| {
            if let Some(sh) = sh.borrow_mut().as_mut() {
                self.attach_pixel(s, sh, (w, h));
            }
        });
        drop(surfaces);
        self.set_region();
        if self.sent.replace(Some((p.x, p.y))) != Some((p.x, p.y))
            && let Some(s) = self.surfaces.borrow().as_ref()
        {
            s.sub.set_position(p.x, p.y);
        }
    }

    /// In a switched drop show, the catcher takes the hive rectangle only
    /// and the receiver picks relative to it (S-020 AC1).
    fn set_region(&self) {
        if !self.switched.get() {
            return;
        }
        let surfaces = self.surfaces.borrow();
        let Some(s) = surfaces.as_ref() else { return };
        let screen = s.configured.unwrap_or(self.screen.get());
        let p = self.placement(screen);
        SHARED.with(|sh| {
            let mut sh = sh.borrow_mut();
            let Some(sh) = sh.as_mut() else { return };
            let (dw, dh) = p.destination;
            let r: WlRegion = sh.wl.compositor.create_region(&sh.wl.qh, ());
            r.add(p.x, p.y, dw as i32, dh as i32);
            // Not revealed and anchored at a known point: leave the pointer
            // out, so the surface never takes pointer focus (S-104).
            if let Some((hx, hy, hw, hh)) = region_hole(
                self.revealed.get(),
                self.anchor.get(),
                (p.x, p.y, dw as i32, dh as i32),
            ) {
                r.subtract(hx, hy, hw, hh);
            }
            s.catcher.set_input_region(Some(&r));
            r.destroy();
            // The drag arrives in catcher coordinates.
            crate::receiver::set_offset(f64::from(p.x), f64::from(p.y));
        });
    }

    /// Draws the Slint window into a new shm buffer on the hive subsurface.
    fn render(&self) {
        self.render_scheduled.set(false);
        if !self.decided.get() {
            return;
        }
        if self.waiting_frame.get() {
            self.redraw_wanted.set(true);
            return;
        }
        let surfaces = self.surfaces.borrow();
        let Some(s) = surfaces.as_ref() else { return };
        let Some(screen) = s.configured else { return };
        self.redraw_wanted.set(false);
        let p = self.placement(screen);
        if !self.revealed.get() {
            // A drop show not revealed: its size and the catcher's pixel
            // and region go out, no hive buffer (S-021 AC4).
            s.hive_vp
                .set_destination(p.destination.0 as i32, p.destination.1 as i32);
            s.catcher.commit();
            drop(surfaces);
            self.report(&p);
            flush();
            return;
        }
        let (pw, ph) = p.buffer;
        let mut pixels = vec![Bgra::default(); (pw * ph) as usize];
        self.renderer.render(&mut pixels, pw as usize);
        let bytes: Vec<u8> = pixels.iter().flat_map(|c| [c.b, c.g, c.r, c.a]).collect();
        let drawn = SHARED.with(|sh| {
            let mut sh = sh.borrow_mut();
            let sh = sh.as_mut()?;
            let buffer = shm_buffer(&sh.wl.shm, &sh.wl.qh, pw, ph, &bytes)?;
            s.hive.attach(Some(&buffer), 0, 0);
            s.hive.damage_buffer(0, 0, pw as i32, ph as i32);
            s.hive.set_buffer_scale(1);
            s.hive_vp
                .set_destination(p.destination.0 as i32, p.destination.1 as i32);
            s.hive.frame(&sh.wl.qh, ());
            s.hive.commit();
            // The subsurface is synchronised: its state applies with this.
            s.catcher.commit();
            // Now the catcher is mapped, so the token can activate it.
            if let (Some(a), Some(t)) = (&sh.wl.activation, self.activation.borrow_mut().take()) {
                a.activate(t, &s.catcher);
            }
            sh.wl.buffers.push(buffer);
            Some(())
        });
        drop(surfaces);
        if drawn.is_some() {
            self.report(&p);
            self.waiting_frame.set(true);
            flush();
            crate::frame_presented();
            if let Some(f) = OPENER.with(|o| o.borrow_mut().take()) {
                f();
            }
        }
    }

    /// The `hive surface` line, once per placement, with the hive's real
    /// size (not the initial 1 x 1).
    fn report(&self, p: &overlay::Placement) {
        if self.logical.get() != (1, 1) && !self.reported.replace(true) {
            crate::report_surface("layer", p.x, p.y, p.destination.0, p.destination.1);
        }
    }

    fn set_logical(&self, w: f32, h: f32) {
        let size = (w.ceil() as u32, h.ceil() as u32);
        if size == self.logical.get() || size.0 == 0 || size.1 == 0 {
            return;
        }
        self.logical.set(size);
        self.window.dispatch_event(WindowEvent::Resized {
            size: LogicalSize::new(size.0 as f32, size.1 as f32),
        });
        self.place();
        self.request_redraw();
    }
}

impl Drop for LayerAdapter {
    fn drop(&mut self) {
        self.unmap();
    }
}

impl WindowAdapter for LayerAdapter {
    fn window(&self) -> &slint::Window {
        &self.window
    }
    fn renderer(&self) -> &dyn Renderer {
        &self.renderer
    }
    fn size(&self) -> PhysicalSize {
        let (w, h) = self.placement((0, 0)).buffer;
        PhysicalSize::new(w, h)
    }
    fn set_size(&self, size: WindowSize) {
        let l = size.to_logical(self.scale120.get() as f32 / 120.0);
        self.set_logical(l.width, l.height);
    }
    fn set_visible(&self, visible: bool) -> Result<(), PlatformError> {
        if visible == self.surfaces.borrow().is_some() {
            return Ok(());
        }
        if visible {
            // Only one hive is shown at a time.
            if let Some(other) = OWNER.with(|o| o.borrow().upgrade()) {
                other.unmap();
            }
            self.map(false);
        } else {
            self.unmap();
        }
        Ok(())
    }
    fn request_redraw(&self) {
        if self.render_scheduled.replace(true) {
            return;
        }
        // Rendering here could run inside a Slint callback; defer.
        let me = self.me.clone();
        slint::Timer::single_shot(Duration::ZERO, move || {
            if let Some(a) = me.upgrade() {
                a.render();
            }
        });
    }
    fn update_window_properties(&self, p: slint::platform::WindowProperties<'_>) {
        let c = p.layout_constraints();
        self.set_logical(c.preferred.width, c.preferred.height);
    }
}

fn shm_buffer(shm: &WlShm, qh: &QueueHandle<Wl>, w: u32, h: u32, bytes: &[u8]) -> Option<WlBuffer> {
    let fd = rustix::fs::memfd_create("kawaro-hive", rustix::fs::MemfdFlags::CLOEXEC).ok()?;
    let mut file = std::fs::File::from(fd);
    file.write_all(bytes).ok()?;
    let pool: WlShmPool = shm.create_pool(file.as_fd(), bytes.len() as i32, qh, ());
    let b = pool.create_buffer(
        0,
        w as i32,
        h as i32,
        (w * 4) as i32,
        wl_shm::Format::Argb8888,
        qh,
        (),
    );
    pool.destroy();
    Some(b)
}

// Dispatch implementations.

impl Dispatch<WlRegistry, GlobalListContents> for Wl {
    fn event(
        s: &mut Self,
        registry: &WlRegistry,
        e: <WlRegistry as Proxy>::Event,
        _: &GlobalListContents,
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        // Outputs come and go after start (`--at` picks among them).
        use wayland_client::protocol::wl_registry::Event;
        match e {
            Event::Global {
                name,
                interface,
                version,
            } if interface == "wl_output" => {
                let o = new_output(registry, name, version, s.xdg_outputs.as_ref(), qh);
                s.outputs.push(o);
            }
            Event::GlobalRemove { name } => {
                if let Some(i) = s.outputs.iter().position(|o| o.name == name) {
                    let o = s.outputs.remove(i);
                    if let Some(x) = o.xdg {
                        x.destroy();
                    }
                    if o.wl.version() >= 3 {
                        o.wl.release();
                    }
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<WlOutput, ()> for Wl {
    fn event(
        _: &mut Self,
        _: &WlOutput,
        _: wl_output::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZxdgOutputV1, u32> for Wl {
    fn event(
        s: &mut Self,
        _: &ZxdgOutputV1,
        e: zxdg_output_v1::Event,
        name: &u32,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let Some(o) = s.outputs.iter_mut().find(|o| o.name == *name) else {
            return;
        };
        match e {
            zxdg_output_v1::Event::LogicalPosition { x, y } => o.origin = (x, y),
            zxdg_output_v1::Event::LogicalSize { width, height } => o.size = (width, height),
            _ => {}
        }
    }
}

delegate_noop!(Wl: ignore WlCompositor);
delegate_noop!(Wl: ignore WlSubcompositor);
delegate_noop!(Wl: ignore WlSubsurface);
delegate_noop!(Wl: ignore WlShm);
delegate_noop!(Wl: ignore WlShmPool);
delegate_noop!(Wl: ignore WlRegion);
delegate_noop!(Wl: ignore ZwlrLayerShellV1);
delegate_noop!(Wl: ignore WpViewporter);
delegate_noop!(Wl: ignore WpViewport);
delegate_noop!(Wl: ignore WpFractionalScaleManagerV1);
delegate_noop!(Wl: ignore WpSinglePixelBufferManagerV1);
delegate_noop!(Wl: ignore XdgActivationV1);
delegate_noop!(Wl: ignore WpCursorShapeManagerV1);
delegate_noop!(Wl: ignore WpCursorShapeDeviceV1);
delegate_noop!(Wl: ignore ZxdgOutputManagerV1);

impl Dispatch<WlBuffer, ()> for Wl {
    fn event(
        s: &mut Self,
        b: &WlBuffer,
        e: wl_buffer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_buffer::Event::Release = e {
            s.buffers.retain(|x| x != b);
            b.destroy();
        }
    }
}

impl Dispatch<WlCallback, ()> for Wl {
    fn event(
        s: &mut Self,
        _: &WlCallback,
        e: wl_callback::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_callback::Event::Done { .. } = e {
            s.events.push(Ev::FrameDone);
        }
    }
}

impl Dispatch<WlSurface, ()> for Wl {
    fn event(
        s: &mut Self,
        surface: &WlSurface,
        e: wl_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // Integer scale, used only without fractional-scale-v1.
        if let wl_surface::Event::PreferredBufferScale { factor } = e
            && s.fractional.is_none()
            && s.hive.as_ref() == Some(surface)
        {
            s.events.push(Ev::Scale(factor.max(1) as u32 * 120));
        }
    }
}

impl Dispatch<WpFractionalScaleV1, ()> for Wl {
    fn event(
        s: &mut Self,
        _: &WpFractionalScaleV1,
        e: wp_fractional_scale_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wp_fractional_scale_v1::Event::PreferredScale { scale } = e {
            s.events.push(Ev::Scale(scale));
        }
    }
}

impl Dispatch<ZwlrLayerSurfaceV1, ()> for Wl {
    fn event(
        s: &mut Self,
        l: &ZwlrLayerSurfaceV1,
        e: zwlr_layer_surface_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match e {
            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                l.ack_configure(serial);
                s.events.push(Ev::Configured(width, height));
            }
            zwlr_layer_surface_v1::Event::Closed => s.events.push(Ev::Closed),
            _ => {}
        }
    }
}

impl Dispatch<WlSeat, ()> for Wl {
    fn event(
        s: &mut Self,
        seat: &WlSeat,
        e: wl_seat::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities {
            capabilities: WEnum::Value(c),
        } = e
        {
            if c.contains(wl_seat::Capability::Pointer) && s.pointer.is_none() {
                let pointer = seat.get_pointer(qh, ());
                s.cursor = s
                    .cursor_shape
                    .as_ref()
                    .map(|m| m.get_pointer(&pointer, qh, ()));
                s.pointer = Some(pointer);
            }
            if c.contains(wl_seat::Capability::Keyboard) && s.keyboard.is_none() {
                s.keyboard = Some(seat.get_keyboard(qh, ()));
            }
        }
    }
}

impl Dispatch<WlPointer, ()> for Wl {
    fn event(
        s: &mut Self,
        _: &WlPointer,
        e: wl_pointer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match e {
            wl_pointer::Event::Enter {
                serial,
                surface,
                surface_x,
                surface_y,
            } => {
                let ours =
                    s.catcher.as_ref() == Some(&surface) || s.hive.as_ref() == Some(&surface);
                if let (true, Some(c)) = (ours, &s.cursor) {
                    c.set_shape(serial, Shape::Default);
                }
                s.pointer_on_hive = s.hive.as_ref() == Some(&surface);
                s.pointer_on_catcher = s.catcher.as_ref() == Some(&surface);
                if s.pointer_on_hive {
                    s.events.push(Ev::Motion(surface_x, surface_y));
                }
                // The enter first: a drop show not revealed is not placed
                // at this position (S-021 AC3).
                if s.pointer_on_catcher {
                    // Hyprland 0.57 enters a layer surface that maps under a
                    // still pointer: that is no pointer, so no hide (S-104).
                    let since = s.map_at.map(|t| t.elapsed());
                    for p in s.probe.enter(since, (surface_x, surface_y)) {
                        s.events.push(p.into());
                    }
                }
            }
            wl_pointer::Event::Leave { surface, .. } => {
                if s.hive.as_ref() == Some(&surface) {
                    s.pointer_on_hive = false;
                    s.events.push(Ev::Leave);
                }
                if s.catcher.as_ref() == Some(&surface) {
                    s.pointer_on_catcher = false;
                    s.probe.leave();
                }
            }
            wl_pointer::Event::Motion {
                surface_x,
                surface_y,
                ..
            } if s.pointer_on_hive => s.events.push(Ev::Motion(surface_x, surface_y)),
            wl_pointer::Event::Motion {
                surface_x,
                surface_y,
                ..
            } if s.pointer_on_catcher => {
                for p in s.probe.motion((surface_x, surface_y)) {
                    s.events.push(p.into());
                }
            }
            // BTN_LEFT
            wl_pointer::Event::Button {
                button: 0x110,
                state: WEnum::Value(st),
                ..
            } => s.events.push(Ev::Button {
                pressed: st == wl_pointer::ButtonState::Pressed,
                on_hive: s.pointer_on_hive,
            }),
            _ => {}
        }
    }
}

impl Dispatch<WlKeyboard, ()> for Wl {
    fn event(
        s: &mut Self,
        _: &WlKeyboard,
        e: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let ours = |s: &Self, surface: &WlSurface| {
            s.catcher.as_ref() == Some(surface) || s.hive.as_ref() == Some(surface)
        };
        match e {
            // The keymap fd is closed unread: the hive needs layout
            // independent keys only.
            wl_keyboard::Event::Enter { surface, .. } if ours(s, &surface) => {
                s.events.push(Ev::Focus(true))
            }
            wl_keyboard::Event::Leave { surface, .. } if ours(s, &surface) => {
                s.events.push(Ev::Focus(false))
            }
            wl_keyboard::Event::Key {
                key,
                state: WEnum::Value(st),
                ..
            } => s.events.push(Ev::Key {
                code: key,
                pressed: st == wl_keyboard::KeyState::Pressed,
            }),
            wl_keyboard::Event::Modifiers { mods_depressed, .. } => {
                let shift = mods_depressed & 1 != 0;
                if shift != s.shift {
                    s.shift = shift;
                    s.events.push(Ev::Shift(shift));
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod pointer_probe_tests {
    //! S-104: a map time `wl_pointer.enter` is not a pointer that moved.
    use super::*;

    const MS: fn(u64) -> Duration = Duration::from_millis;

    #[test]
    fn first_enter_right_after_the_map_is_a_map_time_enter() {
        assert!(is_map_time_enter(MS(0), false));
        assert!(is_map_time_enter(MS(250), false));
    }

    #[test]
    fn a_late_or_second_enter_is_a_real_pointer() {
        assert!(!is_map_time_enter(MS(251), false));
        assert!(!is_map_time_enter(MS(5), true));
    }

    #[test]
    fn two_pixels_from_the_enter_is_still_the_same_place() {
        assert!(!moved_from((39.75, 39.75), (40.0, 40.0)));
        assert!(!moved_from((100.0, 100.0), (102.0, 101.0)));
    }

    #[test]
    fn five_pixels_from_the_enter_is_a_move() {
        assert!(moved_from((40.0, 40.0), (45.0, 40.0)));
        assert!(moved_from((40.0, 40.0), (43.0, 40.0)));
    }

    #[test]
    fn the_hole_is_a_small_square_around_the_anchor() {
        let rect = (0, 0, 400, 300);
        assert_eq!(probe_hole(rect, (200.0, 150.0)), Some((197, 147, 6, 6)));
        // Fractional anchors round to the nearest pixel.
        assert_eq!(probe_hole(rect, (199.6, 150.4)), Some((197, 147, 6, 6)));
    }

    #[test]
    fn the_hole_is_cut_to_the_rectangle_and_absent_outside_it() {
        let rect = (100, 100, 200, 200);
        assert_eq!(probe_hole(rect, (100.0, 100.0)), Some((100, 100, 3, 3)));
        assert_eq!(probe_hole(rect, (10.0, 10.0)), None);
        assert_eq!(probe_hole(rect, (500.0, 150.0)), None);
    }

    fn at(ms: u64) -> Option<Duration> {
        Some(MS(ms))
    }

    #[test]
    fn enter_at_zero_is_ignored_then_small_motion_then_a_five_pixel_move_hides() {
        let mut st = ProbeState::default();
        assert_eq!(
            st.enter(at(0), (40.0, 40.0)),
            vec![Probe::MapEnter(40.0, 40.0)]
        );
        // 2 px: only a position, no hide.
        assert_eq!(st.motion((42.0, 40.0)), vec![Probe::Position(42.0, 40.0)]);
        // 5 px: the hide, once.
        assert_eq!(
            st.motion((45.0, 40.0)),
            vec![Probe::PointerEnter, Probe::Position(45.0, 40.0)]
        );
        assert_eq!(st.motion((60.0, 40.0)), vec![Probe::Position(60.0, 40.0)]);
    }

    #[test]
    fn a_second_enter_is_a_real_pointer() {
        let mut st = ProbeState::default();
        st.enter(at(0), (40.0, 40.0));
        st.leave();
        assert_eq!(
            st.enter(at(10), (40.0, 40.0)),
            vec![Probe::PointerEnter, Probe::Position(40.0, 40.0)]
        );
    }

    #[test]
    fn an_enter_at_three_hundred_ms_is_a_real_pointer() {
        let mut st = ProbeState::default();
        assert_eq!(
            st.enter(at(300), (5.0, 6.0)),
            vec![Probe::PointerEnter, Probe::Position(5.0, 6.0)]
        );
    }

    #[test]
    fn an_enter_with_no_known_map_time_is_a_real_pointer() {
        let mut st = ProbeState::default();
        assert_eq!(st.enter(None, (5.0, 6.0))[0], Probe::PointerEnter);
    }

    #[test]
    fn the_hole_goes_when_the_hive_is_revealed() {
        let rect = (0, 0, 400, 300);
        assert_eq!(
            region_hole(false, Some((200.0, 150.0)), rect),
            Some((197, 147, 6, 6))
        );
        assert_eq!(region_hole(true, Some((200.0, 150.0)), rect), None);
        assert_eq!(region_hole(false, None, rect), None);
    }
}
