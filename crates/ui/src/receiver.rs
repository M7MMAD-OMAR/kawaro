//! Wayland drop receiver for the hive (S-014b, ADR 0003).
//!
//! winit 0.30 has no Wayland drag and drop, so this opens a second
//! wayland-client `Connection` on the `wl_display` that owns the hive surface,
//! with its own event queue and thread, as in `spikes/drop/`: winit's display
//! on the window path, the layer connection's on the overlay (S-018d), one
//! receiver for each. After every drop-mode show the main thread releases the
//! old `wl_data_device` and asks for a new one, because the hive surface is
//! destroyed on hide; events are kept only for the current hive surface.
//!
//! The receiver accepts `text/uri-list` over the cells only, and only as a
//! copy (`set_actions(Copy, Copy)`), so a source offering a move never deletes
//! the original. It reads the dragged files when the drag enters, so the
//! cells can be built before the drop. Picking is `dnd::pick`; everything
//! else goes to the main thread as a [`Note`].

use crate::dnd;
use crate::layout::Plan;
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use rustix::pipe::{PipeFlags, pipe_with};
use std::io::Read;
use std::os::fd::AsFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use wayland_client::globals::{GlobalListContents, registry_queue_init};
use wayland_client::protocol::wl_data_device::{self, WlDataDevice};
use wayland_client::protocol::wl_data_device_manager::{self, DndAction, WlDataDeviceManager};
use wayland_client::protocol::wl_data_offer::{self, WlDataOffer};
use wayland_client::protocol::wl_registry::{self, WlRegistry};
use wayland_client::protocol::wl_seat::{self, WlSeat};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, WEnum, event_created_child};

const URI_LIST: &str = "text/uri-list";
/// How long `enter` waits for the source to send the file list.
const READ_TIMEOUT: Duration = Duration::from_secs(1);
/// A file list larger than this is cut (thousands of paths already).
const READ_LIMIT: usize = 1 << 20;

/// What the receiver reports to the main thread, in protocol order.
#[derive(Debug)]
pub enum Note {
    /// The drag entered the hive with these local files (maybe none).
    Enter(Vec<PathBuf>),
    /// The cell under the drag changed.
    Motion(Option<usize>),
    Leave,
    /// Dropped; the last `Motion` says on which cell, `None` when refused.
    Drop,
}

/// Called on the main thread for each note.
pub type Sink = fn(Note);

struct Shared {
    conn: Connection,
    qh: QueueHandle<Receiver>,
    manager: WlDataDeviceManager,
    seat: WlSeat,
    device: Mutex<Option<WlDataDevice>>,
    /// `wl_surface` pointer of the hive in drop mode, 0 when detached.
    target: AtomicUsize,
}

/// One receiver per `wl_display` pointer; `None` when it failed to start.
static RECEIVERS: Mutex<Vec<(usize, Option<Arc<Shared>>)>> = Mutex::new(Vec::new());
/// The cells every receiver picks from; `None` refuses every drop.
static PLAN: Mutex<Option<Plan>> = Mutex::new(None);
/// Position of the hive on the attached surface, subtracted before picking:
/// the layer path receives on the catcher (S-020), the window path on the
/// hive itself (0, 0).
static OFFSET: Mutex<(f64, f64)> = Mutex::new((0.0, 0.0));

/// Binds a new data device for the hive surface `surface` on `display` (raw
/// `wl_display` and `wl_surface` pointers, from winit or the layer
/// connection). Every earlier device is released first. Starts the receiver
/// of `display` on first use. Call after every drop-mode show, once the
/// surface exists.
pub fn attach(display: usize, surface: usize, sink: Sink) {
    let shared = {
        let mut all = lock(&RECEIVERS);
        for s in all.iter().filter_map(|(_, s)| s.as_ref()) {
            s.target.store(0, Ordering::SeqCst);
            release(s);
            let _ = s.conn.flush();
        }
        match all.iter().find(|(d, _)| *d == display) {
            Some((_, s)) => s.clone(),
            None => {
                let s = start(display, sink);
                all.push((display, s.clone()));
                s
            }
        }
    };
    *lock(&OFFSET) = (0.0, 0.0);
    let Some(s) = shared else { return };
    s.target.store(surface, Ordering::SeqCst);
    let device = s.manager.get_data_device(&s.seat, &s.qh, ());
    *lock(&s.device) = Some(device);
    let _ = s.conn.flush();
}

/// Stops receiving: releases the data devices and forgets the surface.
pub fn detach() {
    for s in lock(&RECEIVERS).iter().filter_map(|(_, s)| s.as_ref()) {
        s.target.store(0, Ordering::SeqCst);
        release(s);
        let _ = s.conn.flush();
    }
    *lock(&PLAN) = None;
}

/// Where the hive sits on the attached surface; set after [`attach`].
pub fn set_offset(x: f64, y: f64) {
    *lock(&OFFSET) = (x, y);
}

/// The cells the receiver picks from; `None` refuses every drop.
pub fn set_plan(plan: Option<Plan>) {
    *lock(&PLAN) = plan;
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn release(s: &Shared) {
    if let Some(old) = lock(&s.device).take()
        && old.version() >= 2
    {
        old.release();
    }
}

fn start(display: usize, sink: Sink) -> Option<Arc<Shared>> {
    // SAFETY: the pointer is a live `wl_display`, winit's (kept for the life
    // of its event loop) or the layer connection's (kept in a thread local of
    // the event loop thread), that is for the life of this process.
    let backend =
        unsafe { wayland_backend::client::Backend::from_foreign_display(display as *mut _) };
    let conn = Connection::from_backend(backend);
    let (globals, mut queue) = match registry_queue_init::<Receiver>(&conn) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("sbarhive: drop receiver: {e}");
            return None;
        }
    };
    let qh = queue.handle();
    let manager = globals.bind::<WlDataDeviceManager, _, _>(&qh, 1..=3, ());
    let seat = globals.bind::<WlSeat, _, _>(&qh, 1..=7, ());
    let (Ok(manager), Ok(seat)) = (manager, seat) else {
        eprintln!("sbarhive: drop receiver: no seat or data device manager");
        return None;
    };
    let shared = Arc::new(Shared {
        conn,
        qh,
        manager,
        seat,
        device: Mutex::new(None),
        target: AtomicUsize::new(0),
    });
    let mut state = Receiver {
        shared: shared.clone(),
        sink,
        offer: None,
        mimes: Vec::new(),
        on_us: false,
        serial: 0,
        hover: None,
        accepted: false,
        action: false,
    };
    let spawned = std::thread::Builder::new()
        .name("sbarhive-drop".into())
        .spawn(move || {
            while queue.blocking_dispatch(&mut state).is_ok() {}
            eprintln!("sbarhive: drop receiver stopped");
        });
    if let Err(e) = spawned {
        eprintln!("sbarhive: drop receiver: {e}");
        return None;
    }
    Some(shared)
}

/// State of the receiver thread.
struct Receiver {
    shared: Arc<Shared>,
    sink: Sink,
    offer: Option<WlDataOffer>,
    mimes: Vec<String>,
    /// The current drag is over the hive surface.
    on_us: bool,
    serial: u32,
    hover: Option<usize>,
    /// The last `accept` named a type.
    accepted: bool,
    /// The compositor chose a (copy) action for the current offer.
    action: bool,
}

impl Receiver {
    fn note(&self, note: Note) {
        let sink = self.sink;
        let _ = slint::invoke_from_event_loop(move || sink(note));
    }

    fn has_uri_list(&self) -> bool {
        self.mimes.iter().any(|m| m == URI_LIST)
    }

    fn pick(&self, x: f64, y: f64) -> Option<usize> {
        let (ox, oy) = *lock(&OFFSET);
        lock(&PLAN)
            .as_ref()
            .and_then(|p| dnd::pick(p, x - ox, y - oy))
    }

    /// Accepts over a cell only, so the cursor shows "no drop" elsewhere.
    fn hover_at(&mut self, x: f64, y: f64) {
        let hit = self.pick(x, y);
        if hit != self.hover {
            self.hover = hit;
            self.note(Note::Motion(hit));
        }
        if let Some(o) = &self.offer {
            let ok = hit.is_some() && self.has_uri_list();
            o.accept(self.serial, ok.then(|| URI_LIST.to_string()));
            self.accepted = ok;
        }
    }

    /// Forgets the current offer without finishing it.
    fn clear(&mut self) {
        if let Some(o) = self.offer.take() {
            o.destroy();
        }
        self.mimes.clear();
        self.on_us = false;
        self.hover = None;
        self.accepted = false;
        self.action = false;
    }

    fn enter(&mut self, x: f64, y: f64) {
        let files = match &self.offer {
            Some(o) => {
                // Copy only: a move would let the source delete the original.
                if o.version() >= 3 {
                    o.set_actions(DndAction::Copy, DndAction::Copy);
                }
                if self.has_uri_list() {
                    read_files(o, &self.shared.conn)
                } else {
                    Vec::new()
                }
            }
            None => Vec::new(),
        };
        self.note(Note::Enter(files));
        self.hover_at(x, y);
    }

    fn drop_here(&mut self) {
        let Some(o) = self.offer.take() else {
            self.note(Note::Motion(None));
            self.note(Note::Drop);
            self.clear();
            return;
        };
        let v3 = o.version() >= 3;
        let ok = self.hover.is_some() && self.accepted && (!v3 || self.action);
        if ok && v3 {
            o.finish();
        }
        o.destroy();
        let _ = self.shared.conn.flush();
        if !ok {
            self.note(Note::Motion(None));
        }
        self.note(Note::Drop);
        self.clear();
    }
}

/// Asks the source for the `text/uri-list` and reads it, waiting at most
/// [`READ_TIMEOUT`]. Drags are allowed to read before the drop.
fn read_files(offer: &WlDataOffer, conn: &Connection) -> Vec<PathBuf> {
    let Ok((r, w)) = pipe_with(PipeFlags::CLOEXEC) else {
        return Vec::new();
    };
    offer.receive(URI_LIST.into(), w.as_fd());
    drop(w);
    let _ = conn.flush();
    let mut file = std::fs::File::from(r);
    let deadline = Instant::now() + READ_TIMEOUT;
    let mut data = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() || data.len() >= READ_LIMIT {
            break;
        }
        let timeout = Timespec {
            tv_sec: left.as_secs() as _,
            tv_nsec: left.subsec_nanos() as _,
        };
        let mut fds = [PollFd::new(&file, PollFlags::IN)];
        match poll(&mut fds, Some(&timeout)) {
            Ok(0) => break,
            Ok(_) => match file.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => data.extend_from_slice(&buf[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(_) => break,
            },
            Err(rustix::io::Errno::INTR) => {}
            Err(_) => break,
        }
    }
    sbarhive_core::uri_list::parse(&String::from_utf8_lossy(&data))
}

impl Dispatch<WlRegistry, GlobalListContents> for Receiver {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlSeat, ()> for Receiver {
    fn event(
        _: &mut Self,
        _: &WlSeat,
        _: wl_seat::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlDataDeviceManager, ()> for Receiver {
    fn event(
        _: &mut Self,
        _: &WlDataDeviceManager,
        _: wl_data_device_manager::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlDataOffer, ()> for Receiver {
    fn event(
        s: &mut Self,
        o: &WlDataOffer,
        e: wl_data_offer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if s.offer.as_ref() != Some(o) {
            return;
        }
        match e {
            wl_data_offer::Event::Offer { mime_type } => s.mimes.push(mime_type),
            wl_data_offer::Event::Action { dnd_action } => {
                s.action = matches!(dnd_action, WEnum::Value(a) if !a.is_empty());
            }
            _ => {}
        }
    }
}

impl Dispatch<WlDataDevice, ()> for Receiver {
    fn event(
        s: &mut Self,
        _: &WlDataDevice,
        e: wl_data_device::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match e {
            wl_data_device::Event::DataOffer { id } => {
                // A drag offer (its types follow before `enter`) or the
                // clipboard selection (`selection` follows). A drag on us
                // cannot be replaced without a `leave`, so a new offer then
                // is the selection.
                if !s.on_us {
                    s.clear();
                    s.offer = Some(id);
                }
            }
            wl_data_device::Event::Selection { id: Some(sel) } => {
                // The clipboard is not ours to read (the clipboard has its
                // own connection); free the offer.
                if s.offer.as_ref() == Some(&sel) {
                    s.offer = None;
                    s.mimes.clear();
                }
                sel.destroy();
            }
            wl_data_device::Event::Enter {
                serial,
                surface,
                x,
                y,
                id,
            } => {
                let target = s.shared.target.load(Ordering::SeqCst);
                let ours = target != 0 && surface.id().as_ptr() as usize == target;
                if !ours || id.is_none() || id != s.offer {
                    s.clear();
                    return;
                }
                s.on_us = true;
                s.serial = serial;
                s.enter(x, y);
            }
            wl_data_device::Event::Motion { x, y, .. } if s.on_us => s.hover_at(x, y),
            wl_data_device::Event::Leave => {
                if s.on_us {
                    s.note(Note::Leave);
                }
                s.clear();
            }
            wl_data_device::Event::Drop if s.on_us => s.drop_here(),
            _ => {}
        }
    }

    event_created_child!(Receiver, WlDataDevice, [
        wl_data_device::EVT_DATA_OFFER_OPCODE => (WlDataOffer, ()),
    ]);
}
