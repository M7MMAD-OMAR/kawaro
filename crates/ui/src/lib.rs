//! SbarHive interface: the hexagonal hive popup and the settings window
//! ([`SettingsWindow`], docs/adr/0002-settings-components.md). Contract in
//! docs/architecture.md ("UI contract"); spike results in
//! docs/adr/0001-ui-toolkit.md.
//!
//! Rendering is Slint with the software renderer on winit. Colours, sizes and
//! durations come from `design/tokens.json` through `build.rs`; the font is
//! IBM Plex Sans Arabic, embedded from `crates/ui/fonts/` (SIL OFL 1.1).
//!
//! Environment overrides read once in [`Hive::new`]:
//! - `SBARHIVE_COLOR_SCHEME=light|dark` forces the theme (otherwise the
//!   system colour scheme, through Slint's `Palette.color-scheme`).
//! - `SBARHIVE_REDUCED_MOTION=1|0` forces reduced motion on or off (otherwise
//!   KDE or GNOME settings, see the `motion` module).

// Some events (Cancel, Escape, ClickOutside) are only fed by its tests so far.
#[cfg_attr(not(test), allow(dead_code))]
mod dnd;
mod geometry;
mod icons;
mod layout;
mod motion;
#[cfg(target_os = "linux")]
mod receiver;
mod settings;
mod tokens {
    include!(concat!(env!("OUT_DIR"), "/tokens.rs"));
}

pub use icons::icon_names;
pub use sbarhive_core::i18n::Lang;
pub use settings::SettingsWindow;

use layout::{Nav, Plan};
use slint::winit_030::{CustomApplicationHandler, EventResult, winit};
use slint::{ComponentHandle, Model, ModelRc, VecModel};
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::{Rc, Weak};
use std::sync::Mutex;
use std::time::{Duration, Instant};

slint::include_modules!();

/// One action in the hive. `code` is the short text drawn in the cell
/// (WEBP, PDF), `label` the full name shown in the centre on hover and read by
/// screen readers. `icon` names one of the built-in icons ([`icon_names`]),
/// drawn above the code; empty or unknown shows the code alone. `ring` is 1
/// for common actions, 2 for the rest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HiveCell {
    pub id: String,
    pub code: String,
    pub label: String,
    pub icon: String,
    pub ring: u8,
}

/// Activation token for the next window adapter, consumed by the winit
/// attributes hook. Slint runs the hook only when a component creates its
/// window adapter, not when it recreates the winit window on `show()` after a
/// `hide()` (those attributes are rebuilt without the hook). So `present`
/// builds a fresh hive component whenever it has a token (S-013).
static PENDING_TOKEN: Mutex<Option<String>> = Mutex::new(None);

thread_local! {
    /// The hive window had keyboard focus since the last `present`.
    static WAS_FOCUSED: Cell<bool> = const { Cell::new(false) };
    /// Called when focus goes away after having been there.
    static ON_FOCUS_LOST: RefCell<Option<Rc<dyn Fn()>>> = const { RefCell::new(None) };
    /// Address of the hive's `slint::Window`, so focus events of other
    /// windows (the settings window) never close the hive. 0 until a hive exists.
    static HIVE_WINDOW: Cell<usize> = const { Cell::new(0) };
    /// Runs once right after the next frame is drawn (measurement hook).
    static AFTER_FRAME: RefCell<Option<Box<dyn FnOnce()>>> = const { RefCell::new(None) };
    /// A drop-mode show waits for its new surface to bind the drop receiver.
    static ATTACH_PENDING: Cell<bool> = const { Cell::new(false) };
    /// The hive that gets the drop receiver's notes (the current one, since
    /// a language or theme change builds a new hive).
    static DROP_HIVE: RefCell<Weak<Inner>> = const { RefCell::new(Weak::new()) };
}

fn window_key(w: &slint::Window) -> usize {
    w as *const slint::Window as usize
}

/// True when the event comes from the hive window. Events without a Slint
/// window, or before a hive exists, count as the hive's (single-window case).
fn is_hive_window(w: Option<&slint::Window>) -> bool {
    let hive = HIVE_WINDOW.with(Cell::get);
    match w {
        Some(w) if hive != 0 => window_key(w) == hive,
        _ => true,
    }
}

/// Focus bookkeeping shared by the winit handler and the tests.
fn focus_changed(focused: bool) {
    if focused {
        WAS_FOCUSED.with(|c| c.set(true));
    } else if WAS_FOCUSED.with(|c| c.replace(false)) {
        let handler = ON_FOCUS_LOST.with(|h| h.borrow().clone());
        if let Some(h) = handler {
            h();
        }
    }
}

struct FocusWatcher;

impl CustomApplicationHandler for FocusWatcher {
    fn window_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _id: winit::window::WindowId,
        _winit_window: Option<&winit::window::Window>,
        slint_window: Option<&slint::Window>,
        event: &winit::event::WindowEvent,
    ) -> EventResult {
        match event {
            winit::event::WindowEvent::Focused(f) if is_hive_window(slint_window) => {
                // Observation hook for tests/focus: logs the hive's focus changes.
                if std::env::var_os("SBARHIVE_DEBUG_FOCUS").is_some() {
                    eprintln!("hive focus: {f}");
                }
                focus_changed(*f)
            }
            winit::event::WindowEvent::RedrawRequested => {
                // The new surface exists by its first redraw, after its
                // get_xdg_surface, so the data device is bound to it.
                if is_hive_window(slint_window)
                    && _winit_window.is_some()
                    && ATTACH_PENDING.with(|c| c.replace(false))
                {
                    attach_receiver(_winit_window);
                }
                // Slint draws right after this hook returns; a zero timer
                // fires on the next loop turn, once the frame is submitted.
                if let Some(f) = AFTER_FRAME.with(|h| h.borrow_mut().take()) {
                    slint::Timer::single_shot(Duration::ZERO, f);
                }
            }
            _ => {}
        }
        EventResult::Propagate
    }
}

/// Selects winit with the software renderer, installs the activation-token
/// hook and the focus watcher. Call once, before creating any window.
pub fn init() -> Result<(), slint::PlatformError> {
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("software".into())
        .with_winit_window_attributes_hook(|attrs| {
            use winit::platform::startup_notify::WindowAttributesExtStartupNotify;
            use winit::window::ActivationToken;
            let token = PENDING_TOKEN.lock().ok().and_then(|mut t| t.take());
            match token {
                Some(t) => attrs.with_activation_token(ActivationToken::from_raw(t)),
                None => attrs,
            }
        })
        .with_winit_custom_application_handler(FocusWatcher)
        .select()
}

/// Binds the drop receiver to the hive surface (Wayland only).
#[cfg(target_os = "linux")]
fn attach_receiver(window: Option<&winit::window::Window>) {
    use winit::raw_window_handle::{
        HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle,
    };
    let Some(w) = window else { return };
    let display = match w.display_handle().map(|h| h.as_raw()) {
        Ok(RawDisplayHandle::Wayland(d)) => d.display.as_ptr() as usize,
        _ => return,
    };
    let surface = match w.window_handle().map(|h| h.as_raw()) {
        Ok(RawWindowHandle::Wayland(s)) => s.surface.as_ptr() as usize,
        _ => return,
    };
    receiver::attach(display, surface, on_note);
}

#[cfg(not(target_os = "linux"))]
fn attach_receiver(_window: Option<&winit::window::Window>) {}

/// A note from the drop receiver, on the event loop thread.
#[cfg(target_os = "linux")]
fn on_note(note: receiver::Note) {
    let Some(inner) = DROP_HIVE.with(|h| h.borrow().upgrade()) else {
        return;
    };
    let event = match note {
        receiver::Note::Enter(files) => {
            inner.drop_files(files);
            dnd::Event::Enter
        }
        receiver::Note::Motion(cell) => dnd::Event::Motion(cell),
        receiver::Note::Leave => dnd::Event::Leave,
        receiver::Note::Drop => dnd::Event::Drop,
    };
    inner.drag(event);
}

/// Measurement hook: runs `f` once, right after the next frame is presented.
/// Only works with the backend installed by [`init`].
#[doc(hidden)]
pub fn after_next_frame(f: impl FnOnce() + 'static) {
    AFTER_FRAME.with(|h| *h.borrow_mut() = Some(Box::new(f)));
}

type ChooseFn = Rc<dyn Fn(String)>;
type DismissFn = Rc<dyn Fn()>;
type FilesFn = Rc<dyn Fn(Vec<PathBuf>) -> (String, Vec<HiveCell>)>;

struct Inner {
    /// Replaced by a fresh component when `present` gets a token.
    ui: RefCell<HiveWindow>,
    /// Number of components built so far (tests check the rebuild).
    builds: Cell<u32>,
    rtl: bool,
    reduced_motion: bool,
    plan: RefCell<Option<Plan>>,
    on_choose: RefCell<Option<ChooseFn>>,
    on_dismiss: RefCell<Option<DismissFn>>,
    on_drop_files: RefCell<Option<FilesFn>>,
    /// Pending timer that starts the opening animation after the first frame.
    opener: slint::Timer,
    /// Hive mode (normal or waiting for a drop), see `dnd`.
    machine: RefCell<dnd::Machine>,
    /// Clock for the drop hive's enter timeout.
    ticker: slint::Timer,
}

impl Inner {
    fn visible(&self) -> bool {
        self.ui.borrow().window().is_visible()
    }

    fn close(&self) {
        // Forget the focus first: hiding drops focus, and that must not be
        // reported as a dismissal.
        WAS_FOCUSED.with(|c| c.set(false));
        self.opener.stop();
        self.ticker.stop();
        *self.machine.borrow_mut() = dnd::Machine::new();
        ATTACH_PENDING.with(|c| c.set(false));
        #[cfg(target_os = "linux")]
        receiver::detach();
        let ui = self.ui.borrow();
        ui.hide().ok();
        ui.set_highlight(-1);
    }

    /// Shows `plan` in the window and keeps it for picking.
    fn apply(&self, centre: &str, plan: Plan) {
        let ui = self.ui.borrow();
        let data: Vec<CellData> = plan
            .cells
            .iter()
            .map(|p| {
                let icon = icons::image(&p.cell.icon);
                CellData {
                    x: p.x,
                    y: p.y,
                    code: p.cell.code.as_str().into(),
                    label: p.cell.label.as_str().into(),
                    has_icon: icon.is_some(),
                    icon: icon.unwrap_or_default(),
                }
            })
            .collect();
        ui.set_cells(ModelRc::new(VecModel::from(data)));
        ui.set_hex_r(plan.layout.radius);
        ui.set_centre_x(plan.layout.origin.0);
        ui.set_centre_y(plan.layout.origin.1);
        ui.set_win_w(plan.width);
        ui.set_win_h(plan.height);
        ui.set_centre_text(centre.into());
        ui.set_centre_label(centre.into());
        #[cfg(target_os = "linux")]
        receiver::set_plan(Some(plan.clone()));
        *self.plan.borrow_mut() = Some(plan);
    }

    /// The drag entered with `files`: lays out their actions in the same
    /// two-ring window, so the centre does not move under the drag.
    fn drop_files(&self, files: Vec<PathBuf>) {
        if !matches!(
            self.machine.borrow().state(),
            dnd::State::Waiting | dnd::State::Over(_)
        ) {
            return;
        }
        let f = self.on_drop_files.borrow().clone();
        let (centre, cells) = f.map(|f| f(files)).unwrap_or_default();
        self.apply(&centre, layout::plan_rings(cells, self.rtl, 2));
    }

    /// Feeds a drag or clock event to the mode machine and acts on the result.
    /// A repeated `Running` or `Hidden` is harmless: `choose` and `dismiss`
    /// do nothing once the hive is closed, and only a new show reopens it.
    fn drag(&self, event: dnd::Event) {
        let state = self.machine.borrow_mut().handle(event, Instant::now());
        match state {
            dnd::State::Over(i) => self.ui.borrow().set_highlight(i as i32),
            dnd::State::Waiting => self.ui.borrow().set_highlight(-1),
            dnd::State::Running(i) => self.choose(i as i32),
            dnd::State::Hidden => self.dismiss(),
            _ => {}
        }
    }

    /// Focus left the hive: closes it, except while waiting for a drop.
    fn focus_lost(&self) {
        let state = self
            .machine
            .borrow_mut()
            .handle(dnd::Event::FocusLost, Instant::now());
        if !matches!(state, dnd::State::Waiting | dnd::State::Over(_)) {
            self.dismiss();
        }
    }

    fn choose(&self, index: i32) {
        let id = self.plan.borrow().as_ref().and_then(|p| {
            usize::try_from(index)
                .ok()
                .and_then(|i| p.cells.get(i))
                .map(|c| c.cell.id.clone())
        });
        let Some(id) = id else { return };
        if !self.visible() {
            return;
        }
        self.close();
        let f = self.on_choose.borrow().clone();
        if let Some(f) = f {
            f(id);
        }
    }

    fn dismiss(&self) {
        if !self.visible() {
            return;
        }
        self.close();
        let f = self.on_dismiss.borrow().clone();
        if let Some(f) = f {
            f();
        }
    }
}

/// The hive popup. One per process; `present` reuses its window unless it
/// has an activation token.
pub struct Hive {
    inner: Rc<Inner>,
}

impl Hive {
    pub fn new(lang: Lang) -> Result<Self, slint::PlatformError> {
        let reduced_motion = motion::detect();
        let inner = Rc::new(Inner {
            ui: RefCell::new(build_window(reduced_motion)?),
            builds: Cell::new(1),
            rtl: lang.rtl(),
            reduced_motion,
            plan: RefCell::new(None),
            on_choose: RefCell::new(None),
            on_dismiss: RefCell::new(None),
            on_drop_files: RefCell::new(None),
            opener: slint::Timer::default(),
            machine: RefCell::new(dnd::Machine::new()),
            ticker: slint::Timer::default(),
        });
        wire(&inner);
        let w = Rc::downgrade(&inner);
        ON_FOCUS_LOST.with(|h| {
            *h.borrow_mut() = Some(Rc::new(move || {
                if let Some(i) = w.upgrade() {
                    i.focus_lost();
                }
            }))
        });

        Ok(Self { inner })
    }

    /// Lays out up to 6 cells in ring 1 and 12 in ring 2 (overflow from ring 1
    /// moves to ring 2 ahead of the ring-2 cells, beyond 18 is dropped, a short
    /// ring 1 is not filled from ring 2), sets the centre text, and shows the
    /// window, activating it with `token` when given. On Wayland the compositor
    /// places the window, normally centred.
    pub fn present(
        &self,
        centre: &str,
        cells: Vec<HiveCell>,
        token: Option<String>,
    ) -> Result<(), slint::PlatformError> {
        let plan = layout::plan(cells, self.inner.rtl);
        self.show(centre, plan, token, false)
    }

    /// Shows an empty two-ring hive that waits for a dragged file (S-014,
    /// `sbarhive drop`). When the drag enters, [`Hive::on_drop_files`] gives
    /// the cells; a drop on a cell runs [`Hive::on_choose`], a drop elsewhere,
    /// a `leave`, or 10 s without the drag closes it with
    /// [`Hive::on_dismiss`]. Focus loss does not close it. On Wayland a drop
    /// receiver is bound to each new hive surface; elsewhere nothing is
    /// received yet.
    pub fn present_drop(
        &self,
        centre: &str,
        token: Option<String>,
    ) -> Result<(), slint::PlatformError> {
        let plan = layout::plan_rings(Vec::new(), self.inner.rtl, 2);
        self.show(centre, plan, token, true)
    }

    fn show(
        &self,
        centre: &str,
        plan: Plan,
        token: Option<String>,
        drop: bool,
    ) -> Result<(), slint::PlatformError> {
        let inner = &self.inner;
        if inner.visible() {
            // Hide first so the old surface goes away before the new one.
            inner.close();
        }
        if token.is_some() {
            // Only a new window adapter runs the attributes hook, which hands
            // the token to winit (see PENDING_TOKEN).
            if let Ok(mut t) = PENDING_TOKEN.lock() {
                *t = token;
            }
            let fresh = build_window(inner.reduced_motion);
            if let Ok(mut t) = PENDING_TOKEN.lock() {
                // Never leak a token into another window (the settings window).
                t.take();
            }
            *inner.ui.borrow_mut() = fresh?;
            inner.builds.set(inner.builds.get() + 1);
            wire(inner);
        }
        inner.apply(centre, plan);
        inner
            .machine
            .borrow_mut()
            .handle(dnd::Event::Show { drop }, Instant::now());
        if drop {
            DROP_HIVE.with(|h| *h.borrow_mut() = Rc::downgrade(inner));
            ATTACH_PENDING.with(|c| c.set(true));
            let w = Rc::downgrade(inner);
            inner.ticker.start(
                slint::TimerMode::Repeated,
                Duration::from_millis(250),
                move || {
                    if let Some(i) = w.upgrade() {
                        i.drag(dnd::Event::Tick);
                    }
                },
            );
        }

        let ui = inner.ui.borrow();
        ui.set_highlight(-1);
        WAS_FOCUSED.with(|c| c.set(false));
        if inner.reduced_motion {
            ui.set_opened(true);
            ui.show()
        } else {
            ui.set_opened(false);
            ui.show()?;
            let w = Rc::downgrade(inner);
            inner
                .opener
                .start(slint::TimerMode::SingleShot, Duration::ZERO, move || {
                    if let Some(i) = w.upgrade() {
                        i.ui.borrow().set_opened(true);
                    }
                });
            Ok(())
        }
    }

    /// Hides the hive without firing any callback.
    pub fn close(&self) {
        self.inner.close();
    }

    /// Called with the action id after a cell is chosen; the hive is already
    /// closed.
    pub fn on_choose(&self, f: impl Fn(String) + 'static) {
        *self.inner.on_choose.borrow_mut() = Some(Rc::new(f));
    }

    /// Called after Esc, focus loss, a click in a gap or on the centre, or a
    /// close request from the compositor; the hive is already closed.
    pub fn on_dismiss(&self, f: impl Fn() + 'static) {
        *self.inner.on_dismiss.borrow_mut() = Some(Rc::new(f));
    }

    /// Called on the event loop thread when a drag enters the drop hive,
    /// with its local files (maybe none). Returns the centre text and the
    /// cells for them; no cells refuses the drop.
    pub fn on_drop_files(&self, f: impl Fn(Vec<PathBuf>) -> (String, Vec<HiveCell>) + 'static) {
        *self.inner.on_drop_files.borrow_mut() = Some(Rc::new(f));
    }

    /// Snapshot hook: highlights a cell as if hovered (-1 for none).
    #[doc(hidden)]
    pub fn set_highlight(&self, index: i32) {
        self.inner.ui.borrow().set_highlight(index);
    }

    /// Snapshot hook: renders the current window content.
    #[doc(hidden)]
    pub fn take_snapshot(
        &self,
    ) -> Result<slint::SharedPixelBuffer<slint::Rgba8Pixel>, slint::PlatformError> {
        self.inner.ui.borrow().window().take_snapshot()
    }
}

/// A new hive component with the theme overrides applied. Creating it
/// creates its window adapter, which runs the attributes hook.
fn build_window(reduced_motion: bool) -> Result<HiveWindow, slint::PlatformError> {
    let ui = HiveWindow::new()?;
    let tokens = ui.global::<Tokens>();
    if let Some(dark) = forced_dark(std::env::var("SBARHIVE_COLOR_SCHEME").ok().as_deref()) {
        tokens.set_dark(dark);
    }
    tokens.set_reduced_motion(reduced_motion);
    Ok(ui)
}

/// `SBARHIVE_COLOR_SCHEME` as a forced dark flag; `None` follows the system.
fn forced_dark(scheme: Option<&str>) -> Option<bool> {
    match scheme {
        Some("light") => Some(false),
        Some("dark") => Some(true),
        _ => None,
    }
}

/// Connects the current component's callbacks to `inner` and marks its
/// window as the hive's for the focus watcher.
fn wire(inner: &Rc<Inner>) {
    let ui = inner.ui.borrow();
    HIVE_WINDOW.with(|c| c.set(window_key(ui.window())));

    let w = Rc::downgrade(inner);
    ui.on_pick(move |x, y| {
        w.upgrade()
            .and_then(|i| i.plan.borrow().as_ref().map(|p| p.pick(x, y)))
            .unwrap_or(-1)
    });
    let w = Rc::downgrade(inner);
    ui.on_navigate(move |current, code| {
        let Some(i) = w.upgrade() else { return -1 };
        let count = i.ui.borrow().get_cells().row_count();
        match Nav::from_code(code) {
            Some(nav) => layout::step(current, nav, count, i.rtl),
            None => current,
        }
    });
    let w = Rc::downgrade(inner);
    ui.on_choose(move |index| {
        if let Some(i) = w.upgrade() {
            i.choose(index);
        }
    });
    let w = Rc::downgrade(inner);
    ui.on_dismiss(move || dismiss_weak(&w));
    let w = Rc::downgrade(inner);
    ui.window().on_close_requested(move || {
        dismiss_weak(&w);
        slint::CloseRequestResponse::HideWindow
    });
}

fn dismiss_weak(w: &Weak<Inner>) {
    if let Some(i) = w.upgrade() {
        i.dismiss();
    }
}

#[cfg(all(test, target_os = "linux"))]
mod drop_hive_tests;
#[cfg(test)]
mod drop_tests;
#[cfg(test)]
mod ui_tests;
