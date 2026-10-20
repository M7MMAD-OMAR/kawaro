//! Spike: can a Slint (winit 0.30) window on Wayland receive a file drop from
//! another application and map it to a hexagon cell?
//!
//! winit 0.30 has no Wayland drag and drop, and Slint does not feed external
//! drops to its DropArea. So this opens a second wayland-client `Connection`
//! on the same `wl_display` that winit uses (raw-window-handle), with its own
//! event queue and thread, binds `wl_data_device_manager`, and filters
//! `wl_data_device` events by winit's `wl_surface` pointer. Hit testing uses
//! the real `crates/ui/src/geometry.rs`.
//!
//! Prints one line per event to stdout: ready, enter, hover, leave, drop.
//! Env: DROP_SPIKE_EXIT=1 exits one second after a drop.

#[path = "../../../crates/ui/src/geometry.rs"]
#[allow(dead_code)]
mod geometry;

use geometry::{Axial, Layout};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use slint::winit_030::WinitWindowAccessor;
use std::io::{Read, Write};
use std::time::Duration;
use wayland_client::protocol::wl_data_device::{self, WlDataDevice};
use wayland_client::protocol::wl_data_device_manager::{DndAction, WlDataDeviceManager};
use wayland_client::protocol::wl_data_offer::{self, WlDataOffer};
use wayland_client::protocol::wl_registry::{self, WlRegistry};
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, event_created_child};

const RADIUS: f32 = 50.0;
const GAP: f32 = 6.0;
const PAD: f32 = 10.0;
const URI_LIST: &str = "text/uri-list";

slint::slint! {
    export component DropHive inherits Window {
        in property <[{x: length, y: length}]> cells;
        in property <int> highlight: -1;
        in property <string> status: "drop a file on a cell";
        title: "drop-spike";
        no-frame: true;
        background: #202020;
        for c[i] in cells: Rectangle {
            x: c.x - 45px; y: c.y - 45px; width: 90px; height: 90px;
            border-radius: 45px;
            background: i == highlight ? #e0a020 : #505050;
            Text { text: i; color: white; font-size: 20px; }
        }
        Text { y: 4px; text: status; color: white; font-size: 11px; }
    }
}

fn say(line: String) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{line}");
    let _ = out.flush();
}

fn layout() -> (Layout, Vec<Axial>, (f32, f32)) {
    let (w, h) = Layout::extent(RADIUS, GAP, 1, PAD);
    let l = Layout { radius: RADIUS, gap: GAP, rtl: false, origin: (w / 2.0, h / 2.0) };
    let mut cells = vec![Axial { q: 0, r: 0 }];
    cells.extend(geometry::ring(1));
    (l, cells, (w, h))
}

struct Dnd {
    target: usize,
    ui: slint::Weak<DropHive>,
    layout: Layout,
    cells: Vec<Axial>,
    seat: Option<WlSeat>,
    manager: Option<WlDataDeviceManager>,
    device: Option<WlDataDevice>,
    offer: Option<WlDataOffer>,
    mimes: Vec<String>,
    on_us: bool,
    serial: u32,
    hover: Option<usize>,
}

impl Dnd {
    fn set_hover(&mut self, x: f64, y: f64) {
        let hit = self.layout.pick(&self.cells, x as f32, y as f32);
        if hit != self.hover {
            self.hover = hit;
            say(format!("hover cell={}", hit.map_or(-1, |i| i as i32)));
            let n = hit.map_or(-1, |i| i as i32);
            let _ = self.ui.upgrade_in_event_loop(move |ui| ui.set_highlight(n));
        }
        // Accept only over a cell, so the cursor shows "no drop" in the gaps.
        if let Some(o) = &self.offer {
            let ok = hit.is_some() && self.mimes.iter().any(|m| m == URI_LIST);
            o.accept(self.serial, ok.then(|| URI_LIST.to_string()));
        }
    }

    fn clear(&mut self) {
        if let Some(o) = self.offer.take() {
            o.destroy();
        }
        self.on_us = false;
        self.hover = None;
        let _ = self.ui.upgrade_in_event_loop(|ui| ui.set_highlight(-1));
    }
}

impl Dispatch<WlRegistry, ()> for Dnd {
    fn event(s: &mut Self, r: &WlRegistry, e: wl_registry::Event, _: &(), _: &Connection, qh: &QueueHandle<Self>) {
        if let wl_registry::Event::Global { name, interface, version } = e {
            match interface.as_str() {
                "wl_seat" if s.seat.is_none() => s.seat = Some(r.bind(name, version.min(7), qh, ())),
                "wl_data_device_manager" => s.manager = Some(r.bind(name, version.min(3), qh, ())),
                _ => {}
            }
        }
    }
}

impl Dispatch<WlSeat, ()> for Dnd {
    fn event(_: &mut Self, _: &WlSeat, _: wayland_client::protocol::wl_seat::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<WlDataDeviceManager, ()> for Dnd {
    fn event(_: &mut Self, _: &WlDataDeviceManager, _: wayland_client::protocol::wl_data_device_manager::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<WlDataOffer, ()> for Dnd {
    fn event(s: &mut Self, o: &WlDataOffer, e: wl_data_offer::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        match e {
            wl_data_offer::Event::Offer { mime_type } => {
                if s.offer.as_ref() == Some(o) || s.offer.is_none() {
                    s.mimes.push(mime_type);
                }
            }
            wl_data_offer::Event::SourceActions { source_actions } => {
                say(format!("source-actions {source_actions:?}"));
            }
            wl_data_offer::Event::Action { dnd_action } => say(format!("action {dnd_action:?}")),
            _ => {}
        }
    }
}

impl Dispatch<WlDataDevice, ()> for Dnd {
    fn event(s: &mut Self, _: &WlDataDevice, e: wl_data_device::Event, _: &(), conn: &Connection, _: &QueueHandle<Self>) {
        match e {
            wl_data_device::Event::DataOffer { id } => {
                // A new offer: its mime types follow before enter.
                if let Some(old) = s.offer.replace(id) {
                    old.destroy();
                }
                s.mimes.clear();
            }
            wl_data_device::Event::Enter { serial, surface, x, y, id } => {
                s.on_us = surface.id().as_ptr() as usize == s.target;
                say(format!("enter ours={} x={x:.0} y={y:.0} mimes={:?}", s.on_us, s.mimes));
                if !s.on_us || id.is_none() {
                    s.clear();
                    return;
                }
                s.serial = serial;
                if let Some(o) = &s.offer {
                    // Copy only: a move would let the source delete the original.
                    if o.version() >= 3 {
                        o.set_actions(DndAction::Copy, DndAction::Copy);
                    }
                }
                s.set_hover(x, y);
            }
            wl_data_device::Event::Motion { x, y, .. } if s.on_us => s.set_hover(x, y),
            wl_data_device::Event::Leave => {
                if s.on_us {
                    say("leave".into());
                }
                s.clear();
            }
            wl_data_device::Event::Drop if s.on_us => {
                let cell = s.hover;
                let mut uris = String::new();
                if let (Some(o), Some(_)) = (&s.offer, cell) {
                    let (r, w) = rustix::pipe::pipe().expect("pipe");
                    o.receive(URI_LIST.into(), w.as_fd_ref());
                    drop(w);
                    let _ = conn.flush();
                    let _ = std::fs::File::from(r).read_to_string(&mut uris);
                    if o.version() >= 3 {
                        o.finish();
                    }
                }
                let files: Vec<&str> = uris.lines().filter(|l| !l.starts_with('#') && !l.is_empty()).collect();
                say(format!("drop cell={} files={files:?}", cell.map_or(-1, |i| i as i32)));
                let n = cell.map_or(-1, |i| i as i32);
                let _ = s.ui.upgrade_in_event_loop(move |ui| {
                    ui.set_status(format!("dropped on cell {n}").into());
                });
                s.clear();
                if std::env::var_os("DROP_SPIKE_EXIT").is_some() {
                    std::thread::spawn(|| {
                        std::thread::sleep(Duration::from_secs(1));
                        std::process::exit(0);
                    });
                }
            }
            _ => {}
        }
    }

    event_created_child!(Dnd, WlDataDevice, [
        wl_data_device::EVT_DATA_OFFER_OPCODE => (WlDataOffer, ()),
    ]);
}

trait AsFdRef {
    fn as_fd_ref(&self) -> std::os::fd::BorrowedFd<'_>;
}
impl AsFdRef for std::os::fd::OwnedFd {
    fn as_fd_ref(&self) -> std::os::fd::BorrowedFd<'_> {
        use std::os::fd::AsFd;
        self.as_fd()
    }
}

/// Starts the drop receiver on winit's display, for winit's surface.
fn start_receiver(ui: &DropHive) {
    let handles = ui.window().with_winit_window(|w| {
        let d = match w.display_handle().map(|h| h.as_raw()) {
            Ok(RawDisplayHandle::Wayland(d)) => d.display.as_ptr() as usize,
            other => {
                say(format!("error: display handle {other:?}"));
                return None;
            }
        };
        let s = match w.window_handle().map(|h| h.as_raw()) {
            Ok(RawWindowHandle::Wayland(s)) => s.surface.as_ptr() as usize,
            other => {
                say(format!("error: window handle {other:?}"));
                return None;
            }
        };
        Some((d, s))
    });
    let Some(Some((display, surface))) = handles else {
        say(format!("error: not a Wayland winit window (has winit window: {})", handles.is_some()));
        return;
    };
    let (layout, cells, _) = layout();
    let weak = ui.as_weak();
    std::thread::spawn(move || {
        // SAFETY: winit keeps the display alive for the life of the event loop,
        // which is the life of this process.
        let backend = unsafe {
            wayland_backend::client::Backend::from_foreign_display(display as *mut _)
        };
        let conn = Connection::from_backend(backend);
        let mut queue = conn.new_event_queue();
        let qh = queue.handle();
        conn.display().get_registry(&qh, ());
        let mut st = Dnd {
            target: surface, ui: weak, layout, cells, seat: None, manager: None, device: None,
            offer: None, mimes: vec![], on_us: false, serial: 0, hover: None,
        };
        queue.roundtrip(&mut st).expect("roundtrip");
        match (&st.manager, &st.seat) {
            (Some(m), Some(seat)) => st.device = Some(m.get_data_device(seat, &qh, ())),
            _ => {
                say("error: no seat or data device manager".into());
                return;
            }
        }
        queue.roundtrip(&mut st).expect("roundtrip");
        say("ready".into());
        loop {
            if queue.blocking_dispatch(&mut st).is_err() {
                say("error: dispatch".into());
                return;
            }
        }
    });
}

thread_local! { static STARTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }

fn main() -> Result<(), slint::PlatformError> {
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("software".into())
        .select()?;
    let ui = DropHive::new()?;
    let (l, cells, (w, h)) = layout();
    let model: Vec<_> = cells
        .iter()
        .map(|a| {
            let (x, y) = l.center(*a);
            (x, y)
        })
        .collect();
    ui.set_cells(std::rc::Rc::new(slint::VecModel::from(
        model,
    )).into());
    ui.window().set_size(slint::LogicalSize::new(w, h));
    ui.show()?;
    let weak = ui.as_weak();
    // The winit window exists only once the event loop runs; poll for it.
    let poll = slint::Timer::default();
    poll.start(slint::TimerMode::Repeated, Duration::from_millis(20), move || {
        if let Some(ui) = weak.upgrade() {
            if ui.window().has_winit_window() && !STARTED.replace(true) {
                start_receiver(&ui);
            }
        }
    });
    slint::run_event_loop()
}
