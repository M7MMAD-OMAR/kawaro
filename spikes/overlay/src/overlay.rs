//! A Slint window adapter that draws on a zwlr_layer_shell_v1 overlay, not an
//! xdg_toplevel, so no compositor tiles, borders, places or unfocuses it.
//!
//! Surfaces, per shown window:
//! - catcher: a layer surface on the overlay layer, anchored to all edges
//!   (the whole output), exclusive zone -1, keyboard interactivity exclusive
//!   (or none in drop mode). Its buffer is one transparent pixel stretched by
//!   wp_viewporter, so it costs nothing, yet a click anywhere outside the hive
//!   lands on it and dismisses. In drop mode its input region is empty.
//! - hive: a wl_subsurface of the catcher, centred, holding the Slint frame
//!   drawn by the software renderer into a wl_shm buffer.
//!
//! Its own wayland-client Connection, read on a thread; events are dispatched
//! on the Slint thread through `invoke_from_event_loop`, collected first and
//! then fed to the Slint window, so Slint callbacks can map or unmap freely.

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
use std::time::Instant;
use wayland_client::globals::{GlobalList, GlobalListContents, registry_queue_init};
use wayland_client::protocol::{
    wl_buffer::{self, WlBuffer},
    wl_callback::{self, WlCallback},
    wl_compositor::WlCompositor,
    wl_keyboard::{self, WlKeyboard},
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
use wayland_protocols::wp::fractional_scale::v1::client::{
    wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
    wp_fractional_scale_v1::{self, WpFractionalScaleV1},
};
use wayland_protocols::wp::single_pixel_buffer::v1::client::wp_single_pixel_buffer_manager_v1::WpSinglePixelBufferManagerV1;
use wayland_protocols::xdg::activation::v1::client::xdg_activation_v1::XdgActivationV1;
use wayland_protocols::wp::viewporter::client::{
    wp_viewport::WpViewport, wp_viewporter::WpViewporter,
};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{self, Anchor, KeyboardInteractivity, ZwlrLayerSurfaceV1},
};

/// Logs with milliseconds since process start (the run scripts grep these).
pub fn log(msg: impl AsRef<str>) {
    let wall = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    eprintln!("[{:7.1} ms | t={:.3}] {}", crate::since_start(), (wall.as_millis() % 100_000) as f64 / 1000.0, msg.as_ref());
}

thread_local! {
    static ADAPTERS: RefCell<Vec<Weak<OverlayAdapter>>> = const { RefCell::new(Vec::new()) };
    static ON_OUTSIDE: RefCell<Option<Rc<dyn Fn()>>> = const { RefCell::new(None) };
    /// Drop mode for the next show: catcher takes no input, no keyboard.
    static DROP_MODE: Cell<bool> = const { Cell::new(false) };
}

/// Called when the user clicks outside the hive (on the catcher).
pub fn on_outside(f: impl Fn() + 'static) {
    ON_OUTSIDE.with(|o| *o.borrow_mut() = Some(Rc::new(f)));
}

pub fn set_drop_mode(on: bool) {
    DROP_MODE.with(|d| d.set(on));
}

/// wl_shm ARGB8888 is little endian, so bytes are B, G, R, A, premultiplied.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Bgra {
    b: u8,
    g: u8,
    r: u8,
    a: u8,
}

impl TargetPixel for Bgra {
    fn blend(&mut self, c: PremultipliedRgbaColor) {
        let a = (255 - c.alpha) as u16;
        self.r = (self.r as u16 * a / 255) as u8 + c.red;
        self.g = (self.g as u16 * a / 255) as u8 + c.green;
        self.b = (self.b as u16 * a / 255) as u8 + c.blue;
        self.a = (self.a as u16 + c.alpha as u16 - (self.a as u16 * c.alpha as u16) / 255) as u8;
    }
    fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { b, g, r, a: 255 }
    }
    fn background() -> Self {
        Self::default()
    }
}

/// What the Wayland side saw, delivered to Slint after dispatching.
#[derive(Debug)]
enum Ev {
    Configured(u32, u32),
    Closed,
    Scale(f32),
    FrameDone,
    HiveMotion(f64, f64),
    HiveLeave,
    Button { pressed: bool, on_hive: bool },
    Key { code: u32, pressed: bool },
    Focus(bool),
    Shift(bool),
}

struct Surfaces {
    catcher: WlSurface,
    layer: ZwlrLayerSurfaceV1,
    catcher_vp: Option<WpViewport>,
    hive: WlSurface,
    sub: WlSubsurface,
    hive_vp: Option<WpViewport>,
    frac: Option<WpFractionalScaleV1>,
    configured: Option<(u32, u32)>,
}

pub struct Wl {
    qh: QueueHandle<Wl>,
    compositor: WlCompositor,
    subcompositor: WlSubcompositor,
    shm: WlShm,
    layer_shell: ZwlrLayerShellV1,
    viewporter: Option<WpViewporter>,
    fractional: Option<WpFractionalScaleManagerV1>,
    single_pixel: Option<WpSinglePixelBufferManagerV1>,
    activation: Option<XdgActivationV1>,
    pointer: Option<WlPointer>,
    keyboard: Option<WlKeyboard>,
    s: Option<Surfaces>,
    pointer_on_hive: bool,
    shift: bool,
    events: Vec<Ev>,
}

/// Lists the globals a compositor offers (for the ADR's evidence).
pub fn print_globals() -> Result<(), String> {
    let conn = Connection::connect_to_env().map_err(|e| e.to_string())?;
    let (globals, _q): (GlobalList, EventQueue<Probe>) =
        registry_queue_init(&conn).map_err(|e| e.to_string())?;
    let mut list = globals.contents().clone_list();
    list.sort_by(|a, b| a.interface.cmp(&b.interface));
    for g in list {
        let mark = matches!(
            g.interface.as_str(),
            "zwlr_layer_shell_v1"
                | "wp_viewporter"
                | "wp_fractional_scale_manager_v1"
                | "wp_single_pixel_buffer_manager_v1"
                | "xdg_activation_v1"
                | "wl_data_device_manager"
                | "wp_cursor_shape_manager_v1"
        );
        println!("{}{} v{}", if mark { "* " } else { "  " }, g.interface, g.version);
    }
    Ok(())
}

pub struct Probe;
impl Dispatch<WlRegistry, GlobalListContents> for Probe {
    fn event(_: &mut Self, _: &WlRegistry, _: <WlRegistry as Proxy>::Event, _: &GlobalListContents, _: &Connection, _: &QueueHandle<Self>) {}
}

pub struct OverlayAdapter {
    window: slint::Window,
    renderer: SoftwareRenderer,
    conn: Connection,
    wl: RefCell<Wl>,
    queue: RefCell<EventQueue<Wl>>,
    pending: Arc<AtomicBool>,
    logical: Cell<(f32, f32)>,
    scale: Cell<f32>,
    visible: Cell<bool>,
    waiting_frame: Cell<bool>,
    redraw_wanted: Cell<bool>,
    render_scheduled: Cell<bool>,
    shown_at: Cell<Option<Instant>>,
    frames: Cell<u32>,
}

impl OverlayAdapter {
    pub fn new() -> Result<Rc<Self>, String> {
        let conn = Connection::connect_to_env().map_err(|e| e.to_string())?;
        let (globals, queue): (GlobalList, EventQueue<Wl>) =
            registry_queue_init(&conn).map_err(|e| e.to_string())?;
        let qh = queue.handle();
        let layer_shell: ZwlrLayerShellV1 = globals
            .bind(&qh, 1..=4, ())
            .map_err(|_| "no zwlr_layer_shell_v1".to_string())?;
        let compositor: WlCompositor = globals.bind(&qh, 4..=6, ()).map_err(|e| e.to_string())?;
        let subcompositor: WlSubcompositor = globals.bind(&qh, 1..=1, ()).map_err(|e| e.to_string())?;
        let shm: WlShm = globals.bind(&qh, 1..=1, ()).map_err(|e| e.to_string())?;
        let viewporter = globals.bind(&qh, 1..=1, ()).ok();
        let fractional = globals.bind(&qh, 1..=1, ()).ok();
        let single_pixel = globals.bind(&qh, 1..=1, ()).ok();
        let activation = globals.bind(&qh, 1..=1, ()).ok();
        let _seat: Option<WlSeat> = globals.bind(&qh, 1..=7, ()).ok();
        log(format!(
            "overlay: layer-shell v{} viewporter={} fractional-scale={} single-pixel={}",
            layer_shell.version(),
            viewporter.is_some(),
            fractional.is_some(),
            single_pixel.is_some()
        ));
        let wl = Wl {
            qh,
            compositor,
            subcompositor,
            shm,
            layer_shell,
            viewporter,
            fractional,
            single_pixel,
            activation,
            pointer: None,
            keyboard: None,
            s: None,
            pointer_on_hive: false,
            shift: false,
            events: Vec::new(),
        };
        let pending = Arc::new(AtomicBool::new(false));
        let adapter = Rc::new_cyclic(|w: &Weak<Self>| Self {
            window: slint::Window::new(w.clone() as Weak<dyn WindowAdapter>),
            renderer: SoftwareRenderer::new_with_repaint_buffer_type(RepaintBufferType::NewBuffer),
            conn: conn.clone(),
            wl: RefCell::new(wl),
            queue: RefCell::new(queue),
            pending: pending.clone(),
            logical: Cell::new((1.0, 1.0)),
            scale: Cell::new(1.0),
            visible: Cell::new(false),
            waiting_frame: Cell::new(false),
            redraw_wanted: Cell::new(false),
            render_scheduled: Cell::new(false),
            shown_at: Cell::new(None),
            frames: Cell::new(0),
        });
        let id = ADAPTERS.with(|a| {
            let mut a = a.borrow_mut();
            a.push(Rc::downgrade(&adapter));
            a.len() - 1
        });
        // Seat capabilities and the first events.
        adapter.dispatch();
        let _ = conn.roundtrip();
        adapter.dispatch();
        std::thread::spawn(move || reader(conn, pending, id));
        Ok(adapter)
    }

    fn dispatch(&self) {
        self.pending.store(false, Ordering::SeqCst);
        let events = {
            let mut wl = self.wl.borrow_mut();
            let _ = self.queue.borrow_mut().dispatch_pending(&mut wl);
            std::mem::take(&mut wl.events)
        };
        for e in events {
            self.handle(e);
        }
        let _ = self.conn.flush();
    }

    fn handle(&self, e: Ev) {
        match e {
            Ev::Configured(w, h) => {
                log(format!("overlay: catcher configured {w}x{h}"));
                self.place(w, h);
                self.render();
            }
            Ev::Closed => {
                log("overlay: closed by compositor");
                self.unmap();
            }
            Ev::Scale(f) => {
                if (f - self.scale.get()).abs() > f32::EPSILON {
                    log(format!("overlay: scale {f}"));
                    self.scale.set(f);
                    self.window.dispatch_event(WindowEvent::ScaleFactorChanged { scale_factor: f });
                    self.request_redraw();
                }
            }
            Ev::FrameDone => {
                self.waiting_frame.set(false);
                let n = self.frames.get() + 1;
                self.frames.set(n);
                if n == 1
                    && let Some(t) = self.shown_at.get()
                {
                    log(format!(
                        "overlay: first frame presented {:.1} ms after show",
                        t.elapsed().as_secs_f64() * 1000.0
                    ));
                }
                if self.redraw_wanted.get() || self.window.has_active_animations() {
                    self.render();
                }
            }
            Ev::HiveMotion(x, y) => self.window.dispatch_event(WindowEvent::PointerMoved {
                position: LogicalPosition::new(x as f32, y as f32),
            }),
            Ev::HiveLeave => self.window.dispatch_event(WindowEvent::PointerExited),
            Ev::Button { pressed, on_hive: true } => {
                let p = self.last_pointer();
                let button = PointerEventButton::Left;
                self.window.dispatch_event(if pressed {
                    WindowEvent::PointerPressed { position: p, button }
                } else {
                    WindowEvent::PointerReleased { position: p, button }
                });
            }
            Ev::Button { pressed: true, on_hive: false } => {
                log("overlay: click outside the hive");
                if let Some(f) = ON_OUTSIDE.with(|o| o.borrow().clone()) {
                    f();
                }
            }
            Ev::Button { .. } => {}
            Ev::Key { code, pressed } => {
                if let Some(text) = key_text(code) {
                    log(format!("overlay: key {code} {}", if pressed { "down" } else { "up" }));
                    self.window.dispatch_event(if pressed {
                        WindowEvent::KeyPressed { text }
                    } else {
                        WindowEvent::KeyReleased { text }
                    });
                }
            }
            Ev::Shift(down) => {
                let text: slint::SharedString = Key::Shift.into();
                self.window.dispatch_event(if down {
                    WindowEvent::KeyPressed { text }
                } else {
                    WindowEvent::KeyReleased { text }
                });
            }
            Ev::Focus(f) => {
                log(format!("overlay: keyboard focus {}", if f { "in" } else { "out" }));
                self.window.dispatch_event(WindowEvent::WindowActiveChanged(f));
            }
        }
    }

    fn last_pointer(&self) -> LogicalPosition {
        LAST_POINTER.with(|p| {
            let (x, y) = p.get();
            LogicalPosition::new(x as f32, y as f32)
        })
    }

    /// Creates the catcher layer surface and the hive subsurface.
    fn map(&self) {
        let drop = DROP_MODE.with(|d| d.get());
        let mut wl = self.wl.borrow_mut();
        let qh = wl.qh.clone();
        let catcher = wl.compositor.create_surface(&qh, ());
        let layer = wl.layer_shell.get_layer_surface(
            &catcher,
            None,
            Layer::Overlay,
            "sbarhive".into(),
            &qh,
            (),
        );
        layer.set_anchor(Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right);
        layer.set_size(0, 0);
        layer.set_exclusive_zone(-1);
        layer.set_keyboard_interactivity(if drop {
            KeyboardInteractivity::None
        } else {
            KeyboardInteractivity::Exclusive
        });
        if drop {
            // Drags and clicks pass through outside the hive.
            let r: WlRegion = wl.compositor.create_region(&qh, ());
            catcher.set_input_region(Some(&r));
            r.destroy();
        }
        let catcher_vp = wl.viewporter.as_ref().map(|v| v.get_viewport(&catcher, &qh, ()));
        let hive = wl.compositor.create_surface(&qh, ());
        let sub = wl.subcompositor.get_subsurface(&hive, &catcher, &qh, ());
        let hive_vp = wl.viewporter.as_ref().map(|v| v.get_viewport(&hive, &qh, ()));
        let frac = wl.fractional.as_ref().map(|f| f.get_fractional_scale(&hive, &qh, ()));
        catcher.commit();
        // An activation token from the file manager, as the daemon gets over IPC.
        if let (Some(a), Ok(t)) = (&wl.activation, std::env::var("SPIKE_TOKEN")) {
            a.activate(t.clone(), &catcher);
            log(format!("overlay: activate with token {}", &t[..t.len().min(12)]));
        }
        wl.s = Some(Surfaces { catcher, layer, catcher_vp, hive, sub, hive_vp, frac, configured: None });
    }

    fn unmap(&self) {
        let mut wl = self.wl.borrow_mut();
        if let Some(s) = wl.s.take() {
            if let Some(f) = s.frac {
                f.destroy();
            }
            if let Some(v) = s.hive_vp {
                v.destroy();
            }
            s.sub.destroy();
            s.hive.destroy();
            if let Some(v) = s.catcher_vp {
                v.destroy();
            }
            s.layer.destroy();
            s.catcher.destroy();
        }
        wl.pointer_on_hive = false;
        drop(wl);
        let _ = self.conn.flush();
        self.waiting_frame.set(false);
        self.frames.set(0);
    }

    /// Gives the catcher its stretched transparent pixel and centres the hive.
    fn place(&self, w: u32, h: u32) {
        let mut wl = self.wl.borrow_mut();
        let qh = wl.qh.clone();
        let (lw, lh) = self.logical.get();
        let buffer = match (&wl.single_pixel, wl.s.as_ref().and_then(|s| s.catcher_vp.as_ref())) {
            (Some(spb), Some(_)) => spb.create_u32_rgba_buffer(0, 0, 0, 0, &qh, ()),
            (None, Some(_)) => shm_buffer(&wl.shm, &qh, 1, 1, &[Bgra::default()]),
            _ => shm_buffer(&wl.shm, &qh, w, h, &vec![Bgra::default(); (w * h) as usize]),
        };
        let Some(s) = wl.s.as_mut() else { return };
        s.configured = Some((w, h));
        if let Some(vp) = &s.catcher_vp {
            vp.set_destination(w as i32, h as i32);
        }
        s.catcher.attach(Some(&buffer), 0, 0);
        s.catcher.damage_buffer(0, 0, i32::MAX, i32::MAX);
        let x = ((w as f32 - lw) / 2.0).round() as i32;
        let y = ((h as f32 - lh) / 2.0).round() as i32;
        s.sub.set_position(x, y);
        log(format!("overlay: hive {lw}x{lh} at {x},{y} on {w}x{h}"));
    }

    /// Draws the Slint window into a new shm buffer on the hive subsurface.
    fn render(&self) {
        self.render_scheduled.set(false);
        if self.waiting_frame.get() {
            self.redraw_wanted.set(true);
            return;
        }
        let wl = self.wl.borrow();
        let Some(s) = wl.s.as_ref() else { return };
        if s.configured.is_none() {
            return;
        }
        self.redraw_wanted.set(false);
        let (lw, lh) = self.logical.get();
        let sc = self.scale.get();
        let (pw, ph) = ((lw * sc).ceil().max(1.0) as u32, (lh * sc).ceil().max(1.0) as u32);
        let mut pixels = vec![Bgra::default(); (pw * ph) as usize];
        self.renderer.render(&mut pixels, pw as usize);
        let buffer = shm_buffer(&wl.shm, &wl.qh, pw, ph, &pixels);
        s.hive.attach(Some(&buffer), 0, 0);
        s.hive.damage_buffer(0, 0, pw as i32, ph as i32);
        match &s.hive_vp {
            Some(vp) => {
                s.hive.set_buffer_scale(1);
                vp.set_destination(lw.round() as i32, lh.round() as i32);
            }
            None => s.hive.set_buffer_scale(sc.round().max(1.0) as i32),
        }
        s.hive.frame(&wl.qh, ());
        s.hive.commit();
        // The subsurface is synchronised: its state applies with the parent's commit.
        s.catcher.commit();
        drop(wl);
        self.waiting_frame.set(true);
        let _ = self.conn.flush();
    }

    fn set_logical(&self, w: f32, h: f32) {
        if (w, h) != self.logical.get() && w > 0.0 && h > 0.0 {
            self.logical.set((w, h));
            self.window.dispatch_event(WindowEvent::Resized { size: LogicalSize::new(w, h) });
            let configured = self.wl.borrow().s.as_ref().and_then(|s| s.configured);
            if let Some((cw, ch)) = configured {
                self.place(cw, ch);
            }
            self.request_redraw();
        }
    }
}

thread_local! {
    static LAST_POINTER: Cell<(f64, f64)> = const { Cell::new((0.0, 0.0)) };
}

impl WindowAdapter for OverlayAdapter {
    fn window(&self) -> &slint::Window {
        &self.window
    }
    fn renderer(&self) -> &dyn Renderer {
        &self.renderer
    }
    fn size(&self) -> PhysicalSize {
        let (w, h) = self.logical.get();
        let sc = self.scale.get();
        PhysicalSize::new((w * sc).ceil() as u32, (h * sc).ceil() as u32)
    }
    fn set_size(&self, size: WindowSize) {
        let l = size.to_logical(self.scale.get());
        self.set_logical(l.width, l.height);
    }
    fn set_visible(&self, visible: bool) -> Result<(), PlatformError> {
        if visible == self.visible.get() {
            return Ok(());
        }
        self.visible.set(visible);
        if visible {
            self.shown_at.set(Some(Instant::now()));
            self.map();
            log("overlay: mapped");
        } else {
            self.unmap();
            log("overlay: unmapped");
        }
        let _ = self.conn.flush();
        Ok(())
    }
    fn request_redraw(&self) {
        if self.render_scheduled.replace(true) {
            return;
        }
        // Rendering right here could run inside a Slint callback; defer.
        let w = ADAPTERS.with(|a| {
            a.borrow()
                .iter()
                .position(|x| std::ptr::eq(x.as_ptr(), self))
                .and_then(|i| a.borrow()[i].upgrade())
        });
        slint::Timer::single_shot(std::time::Duration::ZERO, move || {
            if let Some(a) = &w {
                a.render();
            }
        });
    }
    fn update_window_properties(&self, p: slint::platform::WindowProperties<'_>) {
        let c = p.layout_constraints();
        self.set_logical(c.preferred.width, c.preferred.height);
    }
}

/// Blocks on the socket off the Slint thread and wakes it to dispatch.
fn reader(conn: Connection, pending: Arc<AtomicBool>, id: usize) {
    loop {
        let Some(guard) = conn.prepare_read() else {
            std::thread::sleep(std::time::Duration::from_millis(1));
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
        if !pending.swap(true, Ordering::SeqCst) {
            let ok = slint::invoke_from_event_loop(move || {
                let a = ADAPTERS.with(|a| a.borrow().get(id).and_then(|w| w.upgrade()));
                if let Some(a) = a {
                    a.dispatch();
                }
            });
            if ok.is_err() {
                return;
            }
        }
    }
}

fn shm_buffer(shm: &WlShm, qh: &QueueHandle<Wl>, w: u32, h: u32, px: &[Bgra]) -> WlBuffer {
    let fd = rustix::fs::memfd_create("sbarhive-overlay", rustix::fs::MemfdFlags::CLOEXEC)
        .expect("memfd");
    let mut file = std::fs::File::from(fd);
    // SAFETY: Bgra is four plain bytes with no padding.
    let bytes = unsafe { std::slice::from_raw_parts(px.as_ptr() as *const u8, px.len() * 4) };
    file.write_all(bytes).expect("write buffer");
    let pool: WlShmPool = shm.create_pool(file.as_fd(), bytes.len() as i32, qh, ());
    let b = pool.create_buffer(0, w as i32, h as i32, (w * 4) as i32, wl_shm::Format::Argb8888, qh, ());
    pool.destroy();
    b
}

/// Layout independent keys the hive uses, from evdev codes.
fn key_text(code: u32) -> Option<slint::SharedString> {
    let k = match code {
        1 => Key::Escape,
        28 | 96 => Key::Return,
        57 => return Some(" ".into()),
        15 => Key::Tab,
        103 => Key::UpArrow,
        108 => Key::DownArrow,
        105 => Key::LeftArrow,
        106 => Key::RightArrow,
        _ => return None,
    };
    Some(k.into())
}

// Dispatch implementations.

impl Dispatch<WlRegistry, GlobalListContents> for Wl {
    fn event(_: &mut Self, _: &WlRegistry, _: <WlRegistry as Proxy>::Event, _: &GlobalListContents, _: &Connection, _: &QueueHandle<Self>) {}
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

impl Dispatch<WlBuffer, ()> for Wl {
    fn event(_: &mut Self, b: &WlBuffer, e: wl_buffer::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        if let wl_buffer::Event::Release = e {
            b.destroy();
        }
    }
}

impl Dispatch<WlCallback, ()> for Wl {
    fn event(s: &mut Self, _: &WlCallback, e: wl_callback::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        if let wl_callback::Event::Done { .. } = e {
            s.events.push(Ev::FrameDone);
        }
    }
}

impl Dispatch<WlSurface, ()> for Wl {
    fn event(s: &mut Self, surf: &WlSurface, e: wl_surface::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        // Integer scale, used only without fractional-scale-v1.
        if let wl_surface::Event::PreferredBufferScale { factor } = e
            && s.fractional.is_none()
            && s.s.as_ref().is_some_and(|x| &x.hive == surf)
        {
            s.events.push(Ev::Scale(factor as f32));
        }
    }
}

impl Dispatch<WpFractionalScaleV1, ()> for Wl {
    fn event(s: &mut Self, _: &WpFractionalScaleV1, e: wp_fractional_scale_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        if let wp_fractional_scale_v1::Event::PreferredScale { scale } = e {
            s.events.push(Ev::Scale(scale as f32 / 120.0));
        }
    }
}

impl Dispatch<ZwlrLayerSurfaceV1, ()> for Wl {
    fn event(s: &mut Self, l: &ZwlrLayerSurfaceV1, e: zwlr_layer_surface_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        match e {
            zwlr_layer_surface_v1::Event::Configure { serial, width, height } => {
                l.ack_configure(serial);
                s.events.push(Ev::Configured(width, height));
            }
            zwlr_layer_surface_v1::Event::Closed => s.events.push(Ev::Closed),
            _ => {}
        }
    }
}

impl Dispatch<WlSeat, ()> for Wl {
    fn event(s: &mut Self, seat: &WlSeat, e: wl_seat::Event, _: &(), _: &Connection, qh: &QueueHandle<Self>) {
        if let wl_seat::Event::Capabilities { capabilities: WEnum::Value(c) } = e {
            eprintln!("seat capabilities {c:?}");
            if c.contains(wl_seat::Capability::Pointer) && s.pointer.is_none() {
                s.pointer = Some(seat.get_pointer(qh, ()));
            }
            if c.contains(wl_seat::Capability::Keyboard) && s.keyboard.is_none() {
                s.keyboard = Some(seat.get_keyboard(qh, ()));
            }
        }
    }
}

impl Dispatch<WlPointer, ()> for Wl {
    fn event(s: &mut Self, _: &WlPointer, e: wl_pointer::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        let is_hive = |s: &Self, surf: &WlSurface| s.s.as_ref().is_some_and(|x| &x.hive == surf);
        match e {
            wl_pointer::Event::Enter { surface, surface_x, surface_y, .. } => {
                s.pointer_on_hive = is_hive(s, &surface);
                if s.pointer_on_hive {
                    LAST_POINTER.with(|p| p.set((surface_x, surface_y)));
                    s.events.push(Ev::HiveMotion(surface_x, surface_y));
                }
            }
            wl_pointer::Event::Leave { surface, .. } => {
                if is_hive(s, &surface) {
                    s.pointer_on_hive = false;
                    s.events.push(Ev::HiveLeave);
                }
            }
            wl_pointer::Event::Motion { surface_x, surface_y, .. } if s.pointer_on_hive => {
                LAST_POINTER.with(|p| p.set((surface_x, surface_y)));
                s.events.push(Ev::HiveMotion(surface_x, surface_y));
            }
            wl_pointer::Event::Button { button: 0x110, state: WEnum::Value(st), .. } => {
                s.events.push(Ev::Button {
                    pressed: st == wl_pointer::ButtonState::Pressed,
                    on_hive: s.pointer_on_hive,
                });
            }
            _ => {}
        }
    }
}

impl Dispatch<WlKeyboard, ()> for Wl {
    fn event(s: &mut Self, _: &WlKeyboard, e: wl_keyboard::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        let ours = |s: &Self, surf: &WlSurface| s.s.as_ref().is_some_and(|x| &x.catcher == surf || &x.hive == surf);
        match e {
            // The keymap fd is dropped (closed): the hive needs layout independent keys only.
            wl_keyboard::Event::Keymap { .. } => {}
            wl_keyboard::Event::Enter { surface, .. } => {
                eprintln!("wl_keyboard enter, ours={}", ours(s, &surface));
                if ours(s, &surface) {
                    s.events.push(Ev::Focus(true));
                }
            }
            wl_keyboard::Event::Leave { surface, .. } => {
                eprintln!("wl_keyboard leave, ours={}", ours(s, &surface));
                if ours(s, &surface) {
                    s.events.push(Ev::Focus(false));
                }
            }
            wl_keyboard::Event::Key { key, state: WEnum::Value(st), .. } => {
                eprintln!("wl_keyboard key {key} {st:?}");
                s.events.push(Ev::Key { code: key, pressed: st == wl_keyboard::KeyState::Pressed })
            }
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
