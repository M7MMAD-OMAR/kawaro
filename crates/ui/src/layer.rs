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
//!   first frame), then the hive rectangle, so
//!   it takes the drag (S-020: Hyprland sends no drag motion to a layer
//!   surface's subsurface; S-019 AC8).
//! - hive: a subsurface of the catcher, placed by [`layout::plan_at`] at
//!   the anchor, holding the Slint frame in a wl_shm buffer; no input in
//!   drop mode.
//!
//! The anchor (S-019 AC5, AC8) is the first pointer enter or motion on the
//! catcher, or drag position in drop mode, waited for at most
//! [`ANCHOR_WAIT`] from the first configure; without one the hive is
//! centred. A drop show centred so moves once, at its first position within
//! [`MOVE_WINDOW`] of its first frame; after that its catcher takes the hive
//! rectangle and the hive stays centred (manager decision 12). With
//! `sbarhive drop --at X,Y` the output holding that point (xdg-output) is
//! chosen and the hive placed before the first commit.
//!
//! One wayland-client connection for the process, read on a thread; its
//! events are dispatched on the Slint thread and go to the adapter that is
//! shown. Decisions (surface, placement, keys) come from [`crate::overlay`].
//! A drop-mode show also binds the drop receiver (`receiver.rs`) on this
//! connection's display, to the catcher, offset by the hive position (S-020).

use crate::HiveCell;
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
use std::time::Duration;
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
}

/// How long a show waits for the pointer before it centres the hive
/// (S-019 AC5, AC8).
const ANCHOR_WAIT: Duration = Duration::from_millis(50);

/// How long a centred drop show's catcher keeps its full input region
/// after the first frame, waiting for a position; then it takes the hive
/// rectangle and the hive stays centred (S-019 manager decision 12).
const MOVE_WINDOW: Duration = Duration::from_millis(300);

/// The cells of the next show and its kind, so the hive can be laid out
/// again at the pointer (S-019c, S-019d).
#[derive(Default)]
pub(crate) struct ShowRequest {
    pub cells: Vec<HiveCell>,
    pub rtl: bool,
    pub min_rings: i32,
    /// A drop show (S-014): no keyboard, the catcher takes the drag.
    pub drop: bool,
    /// `sbarhive drop --at X,Y`, global logical coordinates.
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

/// The surface decision for the advertised globals, `SBARHIVE_SURFACE`,
/// `hive_surface` in config.toml (read again at every build) and the
/// screen reader state (S-019 AC1).
fn choice(flag: bool, reader: bool) -> (Surface, String) {
    let forced = std::env::var("SBARHIVE_SURFACE").ok();
    let config = sbarhive_core::config::Config::load().0.hive_surface;
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
        let drop_mode = SHOW.with(|r| r.borrow().drop);
        self.timer.stop();
        self.decided.set(true);
        self.may_move.set(drop_mode && anchor.is_none());
        self.plan_for(anchor, screen);
        if drop_mode && anchor.is_some() {
            self.switched.set(true);
        }
        if !drop_mode && let Some(s) = self.surfaces.borrow().as_ref() {
            s.layer
                .set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
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
            // MOVE_WINDOW from this first frame, so a still pointer is never
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
        if std::env::var_os("SBARHIVE_DEBUG_FOCUS").is_some() {
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
        if std::env::var_os("SBARHIVE_DEBUG_FOCUS").is_some() {
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
                    return;
                }
                self.place();
                self.render();
            }
            Ev::Position(x, y) => {
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
            Ev::PointerEnter => {
                if self.surfaces.borrow().as_ref().is_some_and(|s| s.drop_mode) {
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
                if std::env::var_os("SBARHIVE_DEBUG_FOCUS").is_some() {
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

    /// Creates the catcher layer surface and the hive subsurface.
    fn map(&self) {
        let (drop_mode, at) = SHOW.with(|r| {
            let r = r.borrow();
            (r.drop, r.at)
        });
        *self.activation.borrow_mut() = self.token.borrow_mut().take();
        self.reported.set(false);
        self.decided.set(false);
        self.may_move.set(false);
        self.switched.set(false);
        self.anchor.set(None);
        self.pos.set(None);
        self.sent.set(None);
        // `--at`: the output holding the point, and the point on it.
        let target = at.and_then(output_at);
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
                "sbarhive".into(),
                qh,
                (),
            );
            layer.set_anchor(Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right);
            layer.set_size(0, 0);
            layer.set_exclusive_zone(-1);
            // A normal show takes the keyboard with its first frame (in
            // `decide`), not while the catcher waits for the pointer.
            if drop_mode {
                layer.set_keyboard_interactivity(KeyboardInteractivity::None);
            }
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
                crate::attach_layer_receiver(display, catcher);
            }
        }
        self.waiting_frame.set(false);
        flush();
    }

    fn unmap(&self) {
        // A show closed before its first frame keeps no token for the next.
        self.activation.borrow_mut().take();
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
            // Once per show, with the hive's real size (not the initial 1 x 1).
            if self.logical.get() != (1, 1) && !self.reported.replace(true) {
                crate::report_surface("layer", p.x, p.y, p.destination.0, p.destination.1);
            }
            self.waiting_frame.set(true);
            flush();
            crate::frame_presented();
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
            self.map();
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
    let fd = rustix::fs::memfd_create("sbarhive-hive", rustix::fs::MemfdFlags::CLOEXEC).ok()?;
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
                if s.pointer_on_catcher {
                    s.events.push(Ev::Position(surface_x, surface_y));
                    s.events.push(Ev::PointerEnter);
                }
            }
            wl_pointer::Event::Leave { surface, .. } => {
                if s.hive.as_ref() == Some(&surface) {
                    s.pointer_on_hive = false;
                    s.events.push(Ev::Leave);
                }
                if s.catcher.as_ref() == Some(&surface) {
                    s.pointer_on_catcher = false;
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
            } if s.pointer_on_catcher => s.events.push(Ev::Position(surface_x, surface_y)),
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
