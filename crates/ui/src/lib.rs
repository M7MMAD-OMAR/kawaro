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

mod geometry;
mod layout;
mod motion;
mod settings;
mod tokens {
    include!(concat!(env!("OUT_DIR"), "/tokens.rs"));
}

pub use sbarhive_core::i18n::Lang;
pub use settings::SettingsWindow;

use layout::{Nav, Plan};
use slint::winit_030::{CustomApplicationHandler, EventResult, winit};
use slint::{ComponentHandle, Model, ModelRc, VecModel};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::Mutex;
use std::time::Duration;

slint::include_modules!();

/// One action in the hive. `code` is the short text drawn in the cell
/// (WEBP, PDF), `label` the full name shown in the centre on hover and read by
/// screen readers. `ring` is 1 for common actions, 2 for the rest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HiveCell {
    pub id: String,
    pub code: String,
    pub label: String,
    pub ring: u8,
}

/// Activation token for the next window creation, consumed by the winit
/// attributes hook. Slint recreates the winit window on every `show()` on
/// Wayland, so the hook runs once per `present`.
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
                focus_changed(*f)
            }
            winit::event::WindowEvent::RedrawRequested => {
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

/// Measurement hook: runs `f` once, right after the next frame is presented.
/// Only works with the backend installed by [`init`].
#[doc(hidden)]
pub fn after_next_frame(f: impl FnOnce() + 'static) {
    AFTER_FRAME.with(|h| *h.borrow_mut() = Some(Box::new(f)));
}

type ChooseFn = Rc<dyn Fn(String)>;
type DismissFn = Rc<dyn Fn()>;

struct Inner {
    ui: HiveWindow,
    rtl: bool,
    reduced_motion: bool,
    plan: RefCell<Option<Plan>>,
    on_choose: RefCell<Option<ChooseFn>>,
    on_dismiss: RefCell<Option<DismissFn>>,
    /// Pending timer that starts the opening animation after the first frame.
    opener: slint::Timer,
}

impl Inner {
    fn visible(&self) -> bool {
        self.ui.window().is_visible()
    }

    fn close(&self) {
        // Forget the focus first: hiding drops focus, and that must not be
        // reported as a dismissal.
        WAS_FOCUSED.with(|c| c.set(false));
        self.opener.stop();
        self.ui.hide().ok();
        self.ui.set_highlight(-1);
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

/// The hive popup. One per process; `present` reuses its window.
pub struct Hive {
    inner: Rc<Inner>,
}

impl Hive {
    pub fn new(lang: Lang) -> Result<Self, slint::PlatformError> {
        let ui = HiveWindow::new()?;
        let tokens = ui.global::<Tokens>();
        match std::env::var("SBARHIVE_COLOR_SCHEME").as_deref() {
            Ok("light") => tokens.set_dark(false),
            Ok("dark") => tokens.set_dark(true),
            _ => {}
        }
        let reduced_motion = motion::detect();
        tokens.set_reduced_motion(reduced_motion);

        let inner = Rc::new(Inner {
            ui,
            rtl: lang.rtl(),
            reduced_motion,
            plan: RefCell::new(None),
            on_choose: RefCell::new(None),
            on_dismiss: RefCell::new(None),
            opener: slint::Timer::default(),
        });
        let ui = &inner.ui;
        HIVE_WINDOW.with(|c| c.set(window_key(ui.window())));

        let w = Rc::downgrade(&inner);
        ui.on_pick(move |x, y| {
            w.upgrade()
                .and_then(|i| i.plan.borrow().as_ref().map(|p| p.pick(x, y)))
                .unwrap_or(-1)
        });
        let w = Rc::downgrade(&inner);
        ui.on_navigate(move |current, code| {
            let Some(i) = w.upgrade() else { return -1 };
            let count = i.ui.get_cells().row_count();
            match Nav::from_code(code) {
                Some(nav) => layout::step(current, nav, count, i.rtl),
                None => current,
            }
        });
        let w = Rc::downgrade(&inner);
        ui.on_choose(move |index| {
            if let Some(i) = w.upgrade() {
                i.choose(index);
            }
        });
        let w = Rc::downgrade(&inner);
        ui.on_dismiss(move || dismiss_weak(&w));
        let w = Rc::downgrade(&inner);
        ui.window().on_close_requested(move || {
            dismiss_weak(&w);
            slint::CloseRequestResponse::HideWindow
        });
        let w = Rc::downgrade(&inner);
        ON_FOCUS_LOST.with(|h| *h.borrow_mut() = Some(Rc::new(move || dismiss_weak(&w))));

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
        let inner = &self.inner;
        let ui = &inner.ui;
        let plan = layout::plan(cells, inner.rtl);
        let data: Vec<CellData> = plan
            .cells
            .iter()
            .map(|p| CellData {
                x: p.x,
                y: p.y,
                code: p.cell.code.as_str().into(),
                label: p.cell.label.as_str().into(),
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
        *inner.plan.borrow_mut() = Some(plan);

        if inner.visible() {
            // Hide first so the new surface picks up the new token.
            inner.close();
        }
        ui.set_highlight(-1);
        WAS_FOCUSED.with(|c| c.set(false));
        if let Ok(mut t) = PENDING_TOKEN.lock() {
            *t = token;
        }
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
                        i.ui.set_opened(true);
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

    /// Snapshot hook: highlights a cell as if hovered (-1 for none).
    #[doc(hidden)]
    pub fn set_highlight(&self, index: i32) {
        self.inner.ui.set_highlight(index);
    }

    /// Snapshot hook: renders the current window content.
    #[doc(hidden)]
    pub fn take_snapshot(
        &self,
    ) -> Result<slint::SharedPixelBuffer<slint::Rgba8Pixel>, slint::PlatformError> {
        self.inner.ui.window().take_snapshot()
    }
}

fn dismiss_weak(w: &Weak<Inner>) {
    if let Some(i) = w.upgrade() {
        i.dismiss();
    }
}

#[cfg(test)]
mod ui_tests;
