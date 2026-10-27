//! Kawaro interface: the hexagonal hive popup and the settings window
//! ([`SettingsWindow`], docs/adr/0002-settings-components.md). Contract in
//! docs/architecture.md ("UI contract"); spike results in
//! docs/adr/0001-ui-toolkit.md.
//!
//! Rendering is Slint with the software renderer on winit. Colours, sizes and
//! durations come from `design/tokens.json` through `build.rs`; the font is
//! IBM Plex Sans Arabic, embedded from `crates/ui/fonts/` (SIL OFL 1.1).
//!
//! Environment overrides read once in [`Hive::new`]:
//! - `KAWARO_COLOR_SCHEME=light|dark` forces the theme (otherwise the
//!   system colour scheme, through Slint's `Palette.color-scheme`).
//! - `KAWARO_REDUCED_MOTION=1|0` forces reduced motion on or off (otherwise
//!   KDE or GNOME settings, see the `motion` module).
//! - `KAWARO_GLASS=1|0` forces the glass surfaces on or off (S-017; the
//!   compositor blur detection of S-017d drives [`Hive::set_glass`]).

// Windows has no drop receiver or preview yet, so some items are unused there.
#![cfg_attr(not(unix), allow(dead_code))]

// Some events (Cancel, Escape, ClickOutside) are only fed by its tests so far.
#[cfg(target_os = "linux")]
mod a11y;
#[cfg_attr(not(test), allow(dead_code))]
mod dnd;
// The pure drop tracker (S-047); only the Windows glue drives it.
#[cfg_attr(not(windows), allow(dead_code))]
mod drop_track;
mod geometry;
mod icons;
#[cfg(target_os = "linux")]
mod layer;
#[cfg(target_os = "linux")]
#[doc(hidden)]
pub use layer::after_catcher_ready;
mod layout;
mod motion;
// Only the Windows window picks a monitor; the tests run everywhere.
#[cfg_attr(not(windows), allow(dead_code))]
mod monitor;
pub mod overlay;
#[cfg(target_os = "linux")]
mod platform;
#[cfg(unix)]
mod preview;
#[cfg(target_os = "linux")]
mod receiver;
mod settings;
#[cfg(unix)]
pub mod thumb;
#[cfg(windows)]
mod win_window;
mod tokens {
    include!(concat!(env!("OUT_DIR"), "/tokens.rs"));
}

pub use drop_track::{Source, drop_hive_allowed};
pub use icons::icon_names;
pub use kawaro_core::i18n::Lang;
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
    /// The window path still owes this show's `hive surface` line, for a
    /// window of this logical size (S-018 AC11).
    static WINDOW_REPORT: Cell<Option<(f32, f32)>> = const { Cell::new(None) };
    /// The hive of the last show, which the layer path lays out again at
    /// the pointer (S-019c, S-019d).
    static SHOWN: RefCell<Weak<Inner>> = const { RefCell::new(Weak::new()) };
}

/// Lays out the shown hive with `plan` (the layer path's plan at the
/// pointer), keeping its texts and preview.
#[cfg(target_os = "linux")]
fn set_shown_plan(plan: Plan) {
    if let Some(inner) = SHOWN.with(|h| h.borrow().upgrade()) {
        inner.set_plan(plan);
    }
}

/// Prints `hive surface: KIND X Y W H` in logical screen pixels when
/// `KAWARO_DEBUG_FOCUS` is set (S-018 AC11).
fn report_surface(kind: &str, x: i32, y: i32, w: u32, h: u32) {
    if std::env::var_os("KAWARO_DEBUG_FOCUS").is_some() {
        stderr_line(&format!("hive surface: {kind} {x} {y} {w} {h}"));
    }
}

/// The window path's line, on a redraw once the window has the hive's size.
/// No Wayland client knows its own position, so X Y assume the compositor
/// centres the window on its output, as GNOME and KWin do.
fn report_window(w: &winit::window::Window) {
    let Some((ew, eh)) = WINDOW_REPORT.with(Cell::get) else {
        return;
    };
    let size = w.inner_size().to_logical::<f64>(w.scale_factor());
    if (size.width - f64::from(ew)).abs() > 1.0 || (size.height - f64::from(eh)).abs() > 1.0 {
        return;
    }
    WINDOW_REPORT.with(|c| c.set(None));
    let (mx, my, mw, mh) = w
        .current_monitor()
        .or_else(|| w.available_monitors().next())
        .map(|m| {
            let s = m.scale_factor();
            let p = m.position().to_logical::<f64>(s);
            let z = m.size().to_logical::<f64>(s);
            (p.x, p.y, z.width, z.height)
        })
        .unwrap_or((0.0, 0.0, size.width, size.height));
    let centre = |o: f64, m: f64, s: f64| (o + ((m - s) / 2.0).max(0.0)).round() as i32;
    report_surface(
        "window",
        centre(mx, mw, size.width),
        centre(my, mh, size.height),
        size.width.round() as u32,
        size.height.round() as u32,
    );
}

/// Prints `line` and its newline to stderr in one write, so the libwayland
/// trace (`WAYLAND_DEBUG`) printed on the same stderr from another thread
/// cannot land inside a line the test scripts read.
pub fn stderr_line(line: &str) {
    use std::io::Write;
    let _ = std::io::stderr().write_all(format!("{line}\n").as_bytes());
}

/// Prints `hive preview: WHAT` when `KAWARO_DEBUG_FOCUS` is set: the
/// thumbnail size handed to Slint, or the kind icon (S-018 AC27).
#[cfg(unix)]
fn debug_preview(what: &str) {
    if std::env::var_os("KAWARO_DEBUG_FOCUS").is_some() {
        stderr_line(&format!("hive preview: {what}"));
    }
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
        // Drags from Explorer (S-047): winit's file hover and drop events.
        #[cfg(windows)]
        if is_hive_window(slint_window) {
            dnd::windows::window_event(event);
        }
        match event {
            winit::event::WindowEvent::Focused(f) if is_hive_window(slint_window) => {
                // Observation hook for tests/focus: logs the hive's focus changes.
                if std::env::var_os("KAWARO_DEBUG_FOCUS").is_some() {
                    stderr_line(&format!("hive focus: {f}"));
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
                if let (true, Some(w)) = (is_hive_window(slint_window), _winit_window) {
                    report_window(w);
                }
                // Slint draws right after this hook returns; a zero timer
                // fires on the next loop turn, once the frame is submitted.
                frame_presented();
            }
            _ => {}
        }
        EventResult::Propagate
    }
}

/// Runs the [`after_next_frame`] hook on the next loop turn.
fn frame_presented() {
    if let Some(f) = AFTER_FRAME.with(|h| h.borrow_mut().take()) {
        slint::Timer::single_shot(Duration::ZERO, f);
    }
}

/// Hands a pending activation token to a new winit window.
#[cfg(unix)]
fn token_hook(attrs: winit::window::WindowAttributes) -> winit::window::WindowAttributes {
    use winit::platform::startup_notify::WindowAttributesExtStartupNotify;
    use winit::window::ActivationToken;
    // The hive has no plate (S-074): ask for an alpha surface so the
    // transparent window background shows the desktop.
    let attrs = attrs.with_transparent(true);
    let token = PENDING_TOKEN.lock().ok().and_then(|mut t| t.take());
    match token {
        Some(t) => attrs.with_activation_token(ActivationToken::from_raw(t)),
        None => attrs,
    }
}

/// Selects winit with the software renderer, installs the activation-token
/// hook and the focus watcher. Call once, before creating any window.
///
/// On Wayland, when [`overlay::choose_surface`] gives `Layer` for the
/// compositor's globals and `KAWARO_SURFACE`, winit is wrapped by a thin
/// platform that puts every hive build on a layer-shell overlay (S-018b,
/// ADR 0004); other windows stay winit's.
pub fn init() -> Result<(), slint::PlatformError> {
    #[cfg(all(unix, target_os = "linux"))]
    if layer::start() {
        let winit = i_slint_backend_winit::Backend::builder()
            .with_renderer_name("software")
            .with_window_attributes_hook(token_hook)
            .with_custom_application_handler(Box::new(FocusWatcher))
            .build()?;
        return slint::platform::set_platform(Box::new(platform::Wrap { winit }))
            .map_err(slint::PlatformError::SetPlatformError);
    }
    let selector = slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("software".into());
    #[cfg(unix)]
    let selector = selector.with_winit_window_attributes_hook(token_hook);
    #[cfg(windows)]
    let selector = selector.with_winit_window_attributes_hook(win_window::attributes);
    selector
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

/// Binds the drop receiver to the catcher on the layer connection (S-018d,
/// S-020), once per drop-mode show, right after its surfaces are made.
#[cfg(target_os = "linux")]
fn attach_layer_receiver(display: usize, surface: usize) {
    if ATTACH_PENDING.with(|c| c.replace(false)) {
        receiver::attach(display, surface, on_note);
    }
}

/// A note from the drop receiver, on the event loop thread.
#[cfg(target_os = "linux")]
fn on_note(note: receiver::Note) {
    let Some(inner) = DROP_HIVE.with(|h| h.borrow().upgrade()) else {
        return;
    };
    let event = match note {
        receiver::Note::Enter(files) => inner.drop_files(files),
        receiver::Note::Motion(cell) => dnd::Event::Motion(cell),
        receiver::Note::Leave => dnd::Event::Leave,
        receiver::Note::Drop => dnd::Event::Drop,
        receiver::Note::PointerEnter => dnd::Event::PointerEnter,
        // The anchor phase on the layer path (S-019 AC9): never the machine.
        receiver::Note::Position(x, y) => return layer::position(x, y),
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
    /// The current component draws on the layer-shell overlay.
    on_layer: Cell<bool>,
    rtl: bool,
    reduced_motion: bool,
    plan: RefCell<Option<Plan>>,
    on_choose: RefCell<Option<ChooseFn>>,
    on_dismiss: RefCell<Option<DismissFn>>,
    on_drop_files: RefCell<Option<FilesFn>>,
    /// Pending timer that starts the opening animation after the first frame.
    opener: slint::Timer,
    /// Pending timer that hides the hive at the end of the close animation
    /// (S-017c).
    closer: slint::Timer,
    /// Glass surfaces on this hive (S-017 AC13), kept across rebuilds.
    glass: Cell<bool>,
    /// Hive mode (normal or waiting for a drop), see `dnd`.
    machine: RefCell<dnd::Machine>,
    /// Clock for the drop hive's enter timeout.
    ticker: slint::Timer,
    /// The cells `on_drop_files` gave for the last enter (S-021 AC3).
    found: Cell<usize>,
    /// Where `present_files` looks for thumbnails and file kinds.
    #[cfg(unix)]
    preview_source: RefCell<Option<(PathBuf, thumb::Globs)>>,
}

impl Inner {
    fn visible(&self) -> bool {
        self.ui.borrow().window().is_visible()
    }

    fn close(&self) {
        self.closer.stop();
        self.ui.borrow().set_closing(false);
        self.teardown();
        self.ui.borrow().hide().ok();
    }

    /// Everything a close does at once: forget the focus first (hiding
    /// drops focus, and that must not be reported as a dismissal), stop the
    /// timers, reset the machine and free the pictures. An animated close
    /// (S-017c) does this at once and hides after `Tokens.hive-close`.
    fn teardown(&self) {
        WAS_FOCUSED.with(|c| c.set(false));
        self.opener.stop();
        self.ticker.stop();
        *self.machine.borrow_mut() = dnd::Machine::new();
        ATTACH_PENDING.with(|c| c.set(false));
        #[cfg(target_os = "linux")]
        receiver::detach();
        // A closed hive ends an Explorer drag (S-067).
        #[cfg(windows)]
        dnd::windows::hive_closed();
        let ui = self.ui.borrow();
        ui.set_highlight(-1);
        // Never keep a picture while hidden (S-018 AC24, AC27).
        ui.set_preview_image(slint::Image::default());
        ui.set_preview_icon(slint::Image::default());
    }

    /// The user driven close (S-017c AC8): `on_choose` and `on_dismiss` run
    /// in the same turn as before, and the surface stays for the 90 ms of
    /// `Tokens.hive-close` while the hive fades and shrinks. A probing drop
    /// show has drawn nothing and closes at once (AC9); so does reduced
    /// motion. `present` during the close cancels it (`close`).
    fn animate_close(&self) {
        let probing = self.machine.borrow().state() == dnd::State::Probing;
        if self.reduced_motion || probing {
            return self.close();
        }
        self.teardown();
        let ui = self.ui.borrow();
        ui.set_closing(true);
        let w = ui.as_weak();
        self.closer.start(
            slint::TimerMode::SingleShot,
            Duration::from_millis(tokens::HIVE_CLOSE_MS),
            move || {
                if let Some(ui) = w.upgrade() {
                    ui.set_closing(false);
                    ui.hide().ok();
                }
            },
        );
    }

    /// Ends the shown hive for a new show, without `on_dismiss` (S-022
    /// decision 8). Returns the reason the replaced show prints: `replaced`
    /// for a probing show, none for any other or a hidden one.
    fn replace(&self) -> Option<&'static str> {
        if !self.visible() {
            return None;
        }
        let probing = self.machine.borrow().state() == dnd::State::Probing;
        self.close();
        probing.then_some("replaced")
    }

    /// Starts the opening animation: at once with reduced motion, else
    /// after the first frame (on the layer path, the first drawn one).
    fn open(&self) {
        let ui = self.ui.borrow();
        if self.reduced_motion {
            ui.set_opened(true);
            return;
        }
        let w = ui.as_weak();
        let start = move || {
            if let Some(ui) = w.upgrade() {
                ui.set_opened(true);
            }
        };
        #[cfg(target_os = "linux")]
        if self.on_layer.get() {
            return layer::after_drawn(start);
        }
        self.opener
            .start(slint::TimerMode::SingleShot, Duration::ZERO, start);
    }

    /// Shows `plan` in the window and keeps it for picking.
    fn apply(&self, centre: &str, plan: Plan) {
        self.set_plan(plan);
        let ui = self.ui.borrow();
        ui.set_centre_text(centre.into());
        ui.set_centre_label(centre.into());
    }

    /// The cells, size and centre of `plan`, without touching the texts.
    fn set_plan(&self, plan: Plan) {
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
                    ring: p.axial.distance(),
                }
            })
            .collect();
        ui.set_cells(ModelRc::new(VecModel::from(data)));
        ui.set_hex_r(plan.layout.radius);
        ui.set_centre_x(plan.layout.origin.0);
        ui.set_centre_y(plan.layout.origin.1);
        ui.set_win_w(plan.width);
        ui.set_win_h(plan.height);
        ui.set_rtl(self.rtl);
        #[cfg(target_os = "linux")]
        receiver::set_plan(Some(plan.clone()));
        *self.plan.borrow_mut() = Some(plan);
    }

    /// Puts `files` in the centre cell (S-018h): one file's cached thumbnail,
    /// downscaled to the physical side of the square inscribed in the centre
    /// hexagon, else the icon of the files' common kind with their shared
    /// format code. One file names the centre by its file name; several keep
    /// `centre`. No files shows no preview.
    #[cfg(unix)]
    fn set_preview(&self, centre: &str, files: &[PathBuf]) {
        let ui = self.ui.borrow();
        ui.set_preview_count(files.len() as i32);
        ui.set_preview_image(slint::Image::default());
        ui.set_preview_icon(slint::Image::default());
        ui.set_preview_code("".into());
        if files.is_empty() {
            return;
        }
        let names: Vec<String> = files
            .iter()
            .map(|f| {
                f.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            })
            .collect();
        ui.set_centre_label(if files.len() == 1 { &names[0] } else { centre }.into());
        let code = names
            .iter()
            .map(|n| thumb::format_code(n))
            .reduce(|a, b| if a == b { a } else { None });
        ui.set_preview_code(code.flatten().unwrap_or_default().into());
        let source = self.preview_source.borrow();
        if let ([file], Some((thumbnails, _))) = (files, source.as_ref())
            && let Some(path) = thumb::find_thumbnail(thumbnails, file)
        {
            let side = (3.0 - 3f32.sqrt()) * ui.get_hex_r() * ui.window().scale_factor();
            if let Some(buf) = preview::decode_fit(&path, side.floor().max(1.0) as u32) {
                debug_preview(&format!("thumb {} {}", buf.width(), buf.height()));
                ui.set_preview_image(slint::Image::from_rgba8(buf));
                return;
            }
        }
        let kind = match source.as_ref() {
            Some((_, globs)) => thumb::common_kind(names.iter().map(|n| thumb::kind_for(globs, n))),
            None => thumb::Kind::File,
        };
        debug_preview(&format!("icon {}", kind.icon()));
        ui.set_preview_icon(icons::image(kind.icon()).unwrap_or_default());
    }

    /// The drag entered with `files`: lays out their actions in the same
    /// two-ring window, so the centre does not move under the drag. Returns
    /// the machine event: a probing show with no cell is rejected and hides
    /// without the centre text (S-021).
    fn drop_files(&self, files: Vec<PathBuf>) -> dnd::Event {
        let state = self.machine.borrow().state();
        if !matches!(
            state,
            dnd::State::Probing | dnd::State::Waiting | dnd::State::Over(_)
        ) {
            return dnd::Event::Enter;
        }
        let f = self.on_drop_files.borrow().clone();
        #[cfg(unix)]
        let dragged = files.clone();
        let (centre, cells) = f.map(|f| f(files)).unwrap_or_default();
        self.found.set(cells.len());
        if state == dnd::State::Probing && cells.is_empty() {
            return dnd::Event::Reject;
        }
        // On the layer path the hive keeps the place and box it took at the
        // pointer (S-019d).
        #[cfg(target_os = "linux")]
        let plan = layer::replan(&cells);
        #[cfg(not(target_os = "linux"))]
        let plan = None;
        #[cfg_attr(not(target_os = "linux"), allow(clippy::unnecessary_literal_unwrap))]
        let plan = plan.unwrap_or_else(|| layout::plan_rings(cells, self.rtl, 2));
        self.apply(&centre, plan);
        // The dragged files in the centre, by the S-018h rules (S-019f).
        #[cfg(unix)]
        self.set_preview(&centre, &dragged);
        dnd::Event::Enter
    }

    /// Feeds a drag or clock event to the mode machine and acts on the result.
    /// A repeated `Running` or `Hidden` is harmless: `choose` and `dismiss`
    /// do nothing once the hive is closed, and only a new show reopens it.
    fn drag(&self, event: dnd::Event) {
        let before = self.machine.borrow().state();
        let state = self.machine.borrow_mut().handle(event, Instant::now());
        // Its line before the hide unmaps the catcher.
        #[cfg(target_os = "linux")]
        layer::probed(before, state, event, self.found.get());
        #[cfg(not(target_os = "linux"))]
        let _ = before;
        // A probed show opens at its reveal (S-022).
        if before == dnd::State::Probing && state == dnd::State::Waiting {
            self.open();
        }
        match state {
            dnd::State::Over(i) => self.ui.borrow().set_highlight(i as i32),
            dnd::State::Waiting => self.ui.borrow().set_highlight(-1),
            dnd::State::Running(i) => self.choose(i as i32),
            // A probing show has drawn nothing and hides at once (S-017 AC9).
            dnd::State::Hidden => self.dismiss_inner(before == dnd::State::Probing),
            _ => {}
        }
    }

    /// Focus left the hive: closes it, except while probing or waiting for
    /// a drop.
    fn focus_lost(&self) {
        let state = self
            .machine
            .borrow_mut()
            .handle(dnd::Event::FocusLost, Instant::now());
        if !matches!(
            state,
            dnd::State::Probing | dnd::State::Waiting | dnd::State::Over(_)
        ) {
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
        // One callback per show: a click or a key during the close (S-017c
        // AC8) adds nothing.
        if !self.visible() || self.ui.borrow().get_closing() {
            return;
        }
        self.animate_close();
        let f = self.on_choose.borrow().clone();
        if let Some(f) = f {
            f(id);
        }
    }

    fn dismiss(&self) {
        let probing = self.machine.borrow().state() == dnd::State::Probing;
        self.dismiss_inner(probing);
    }

    /// Dismisses with the close at once when `instant` (a probing show has
    /// drawn nothing, S-017 AC9), and with the animated close otherwise.
    /// `on_dismiss` runs in the same turn as before, once per show.
    fn dismiss_inner(&self, instant: bool) {
        if !self.visible() || self.ui.borrow().get_closing() {
            return;
        }
        if instant {
            self.close();
        } else {
            self.animate_close();
        }
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
        // The glass surfaces (S-017 AC13) with `KAWARO_GLASS` as the test
        // and demo switch; the compositor blur detection of S-017d drives
        // `set_glass` in the daemon.
        let glass = std::env::var("KAWARO_GLASS")
            .ok()
            .as_deref()
            .and_then(motion::parse_env)
            .unwrap_or(false);
        let inner = Rc::new(Inner {
            ui: RefCell::new(build_window(reduced_motion, glass)?),
            builds: Cell::new(1),
            on_layer: Cell::new(built_on_layer()),
            rtl: lang.rtl(),
            reduced_motion,
            plan: RefCell::new(None),
            on_choose: RefCell::new(None),
            on_dismiss: RefCell::new(None),
            on_drop_files: RefCell::new(None),
            opener: slint::Timer::default(),
            closer: slint::Timer::default(),
            glass: Cell::new(glass),
            machine: RefCell::new(dnd::Machine::new()),
            ticker: slint::Timer::default(),
            found: Cell::new(0),
            #[cfg(unix)]
            preview_source: RefCell::new(None),
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
        self.show(centre, cells, 1, token, None, false, &[])
    }

    /// [`Hive::present`] with the selected `files` in the centre cell
    /// (S-018h): one file's cached thumbnail or the files' kind icon, and for
    /// several a stack of two hexagons with their count. `centre` is the
    /// selection text, read by screen readers for several files and shown
    /// on no hover only when there is no file. The layout is `present`'s.
    #[cfg(unix)]
    pub fn present_files(
        &self,
        centre: &str,
        files: &[PathBuf],
        cells: Vec<HiveCell>,
        token: Option<String>,
    ) -> Result<(), slint::PlatformError> {
        self.show(centre, cells, 1, token, None, false, files)
    }

    /// Where [`Hive::present_files`] looks for thumbnails (the freedesktop
    /// `thumbnails` folder, see [`thumb::thumbnails_dir`]) and file kinds.
    /// The caller reads its environment; the hive never does.
    #[cfg(unix)]
    pub fn set_preview_source(&self, thumbnails: PathBuf, globs: thumb::Globs) {
        *self.inner.preview_source.borrow_mut() = Some((thumbnails, globs));
    }

    /// Shows an empty two-ring hive that waits for a dragged file (S-014,
    /// `kawaro drop`). When the drag enters, [`Hive::on_drop_files`] gives
    /// the cells; a drop on a cell runs [`Hive::on_choose`], a drop elsewhere,
    /// a `leave`, or 10 s without the drag closes it with
    /// [`Hive::on_dismiss`]. Focus loss does not close it. On the layer path
    /// it probes first (S-021): a pointer enter with no drag, files with no
    /// action, or 10 s with no event close it the same way. On Wayland a
    /// drop receiver is bound to each new hive surface; elsewhere nothing is
    /// received yet.
    pub fn present_drop(
        &self,
        centre: &str,
        token: Option<String>,
    ) -> Result<(), slint::PlatformError> {
        self.present_drop_at(centre, token, None)
    }

    /// [`Hive::present_drop`] with the pointer position `at` in global
    /// logical coordinates (`kawaro drop --at`, S-019 decision 1). The
    /// layer path places the hive there at once instead of waiting for the
    /// first drag position; the window path ignores it.
    pub fn present_drop_at(
        &self,
        centre: &str,
        token: Option<String>,
        at: Option<(f64, f64)>,
    ) -> Result<(), slint::PlatformError> {
        self.show(centre, Vec::new(), 2, token, Some(at), false, &[])
    }

    /// [`Hive::present_drop_at`] for a held key (`kawaro drop --hold`,
    /// S-019e), shown when the daemon's long press gate has passed its
    /// threshold. [`Hive::key_released`] closes it, and the drag leaving
    /// it does not (manager decision 3).
    pub fn present_drop_hold(
        &self,
        centre: &str,
        token: Option<String>,
        at: Option<(f64, f64)>,
    ) -> Result<(), slint::PlatformError> {
        self.show(centre, Vec::new(), 2, token, Some(at), true, &[])
    }

    /// The held key of [`Hive::present_drop_hold`] was released: a hold
    /// show probing, waiting or over a cell closes with [`Hive::on_dismiss`]
    /// and runs nothing. Any other show ignores it.
    pub fn key_released(&self) {
        self.inner.drag(dnd::Event::Release);
    }

    /// Shows `cells` in a hive of at least `min_rings` rings; `drop` is
    /// `Some(at)` for a drop show, and `hold` a drop show of a held key. On
    /// the layer path the hive is laid out again at the pointer once its
    /// position is known (S-019c, S-019d).
    #[allow(clippy::too_many_arguments)]
    fn show(
        &self,
        centre: &str,
        cells: Vec<HiveCell>,
        min_rings: i32,
        token: Option<String>,
        drop: Option<Option<(f64, f64)>>,
        hold: bool,
        files: &[PathBuf],
    ) -> Result<(), slint::PlatformError> {
        let inner = &self.inner;
        #[cfg(windows)]
        let (plan, win_at) = win_window::plan(cells.clone(), inner.rtl, min_rings);
        #[cfg(not(windows))]
        let plan = layout::plan_rings(cells.clone(), inner.rtl, min_rings);
        if inner.visible() {
            // Hide first so the old surface goes away before the new one.
            let reason = inner.replace();
            #[cfg(target_os = "linux")]
            layer::replaced(reason);
            #[cfg(not(target_os = "linux"))]
            let _ = reason;
        }
        // A token needs a new window adapter, which runs the attributes hook
        // and hands the token to winit (see PENDING_TOKEN). So does a changed
        // surface decision, such as a screen reader turned on or off (S-018
        // AC12): the adapter kind is fixed when the component is built.
        if token.is_some() || inner.on_layer.get() != wants_layer() {
            if let Ok(mut t) = PENDING_TOKEN.lock() {
                *t = token;
            }
            let fresh = build_window(inner.reduced_motion, inner.glass.get());
            if let Ok(mut t) = PENDING_TOKEN.lock() {
                // Never leak a token into another window (the settings window).
                t.take();
            }
            *inner.ui.borrow_mut() = fresh?;
            inner.on_layer.set(built_on_layer());
            inner.builds.set(inner.builds.get() + 1);
            wire(inner);
        }
        inner.apply(centre, plan);
        #[cfg(unix)]
        inner.set_preview(centre, files);
        #[cfg(not(unix))]
        let _ = files;
        let (at, drop) = (drop.flatten(), drop.is_some());
        // On the layer path a drop show probes for a file drag (S-021).
        let layer = inner.on_layer.get();
        inner
            .machine
            .borrow_mut()
            .handle(dnd::Event::Show { drop, hold, layer }, Instant::now());
        #[cfg(target_os = "linux")]
        layer::set_show(layer::ShowRequest {
            cells,
            rtl: inner.rtl,
            min_rings,
            drop,
            at,
        });
        #[cfg(not(target_os = "linux"))]
        let _ = (cells, at);
        SHOWN.with(|h| *h.borrow_mut() = Rc::downgrade(inner));
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
        // The layer adapter prints its own line with its first frame.
        let size = (!inner.on_layer.get()).then(|| (ui.get_win_w(), ui.get_win_h()));
        WINDOW_REPORT.with(|c| c.set(size));
        // A probing show opens at its reveal, in `Inner::drag` (S-022).
        let probing = inner.machine.borrow().state() == dnd::State::Probing;
        ui.set_opened(false);
        // A show cancels any close still animating (S-017c AC8).
        ui.set_closing(false);
        #[cfg(windows)]
        ui.window().set_position(win_at);
        ui.show()?;
        #[cfg(windows)]
        win_window::square_corners(ui.window());
        if !probing {
            inner.open();
        }
        Ok(())
    }

    /// The native handle of the shown hive window, for the foreground call.
    #[cfg(windows)]
    pub fn hwnd(&self) -> Option<isize> {
        win_window::hwnd(self.inner.ui.borrow().window())
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

    /// The glass surfaces of S-017 AC13: cells and stacks `glass-2`, the
    /// centre `glass-3`, only over a compositor blur. The daemon drives it
    /// from the blur capability (S-017d); `KAWARO_GLASS` forces it at build.
    pub fn set_glass(&self, glass: bool) {
        self.inner.glass.set(glass);
        self.inner.ui.borrow().set_glass(glass);
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
fn build_window(reduced_motion: bool, glass: bool) -> Result<HiveWindow, slint::PlatformError> {
    // Every hive build goes through the surface decision (S-018 AC5).
    #[cfg(target_os = "linux")]
    platform::building_hive(true);
    #[cfg(windows)]
    win_window::set_building(true);
    let ui = HiveWindow::new();
    #[cfg(windows)]
    win_window::set_building(false);
    #[cfg(target_os = "linux")]
    platform::building_hive(false);
    let ui = ui?;
    let tokens = ui.global::<Tokens>();
    if let Some(dark) = forced_dark(std::env::var("KAWARO_COLOR_SCHEME").ok().as_deref()) {
        tokens.set_dark(dark);
    }
    tokens.set_reduced_motion(reduced_motion);
    ui.set_glass(glass);
    Ok(ui)
}

/// The next hive build goes on the overlay.
fn wants_layer() -> bool {
    #[cfg(target_os = "linux")]
    return layer::wanted();
    #[cfg(not(target_os = "linux"))]
    false
}

/// The last hive build got the overlay adapter.
fn built_on_layer() -> bool {
    #[cfg(target_os = "linux")]
    return platform::last_hive_on_layer();
    #[cfg(not(target_os = "linux"))]
    false
}

/// `KAWARO_COLOR_SCHEME` as a forced dark flag; `None` follows the system.
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
        // The close animation keeps the surface for `Tokens.hive-close`;
        // its timer hides the window (S-017c AC8).
        slint::CloseRequestResponse::KeepWindowShown
    });
}

fn dismiss_weak(w: &Weak<Inner>) {
    if let Some(i) = w.upgrade() {
        i.dismiss();
    }
}

#[cfg(test)]
mod anchor_tests;
#[cfg(test)]
pub(crate) mod test_env {
    //! `KAWARO_*` variables are process wide while the tests run on many
    //! threads. Every `Hive::new` holds [`hold`], and the tests that set a
    //! variable around a build hold it across the whole dance, so no build
    //! can observe another test's environment.
    use crate::Lang;
    use std::sync::{Mutex, MutexGuard};

    static LOCK: Mutex<()> = Mutex::new(());

    pub fn hold() -> MutexGuard<'static, ()> {
        LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Builds a hive with `vars` set, then removes them again.
    pub fn build(vars: &[(&str, &str)], lang: Lang) -> crate::Hive {
        let _g = hold();
        for (k, v) in vars {
            // SAFETY: the lock keeps every other build out of this window.
            unsafe { std::env::set_var(k, v) };
        }
        let hive = crate::Hive::new(lang);
        for (k, _) in vars {
            // SAFETY: as above.
            unsafe { std::env::remove_var(k) };
        }
        hive.unwrap()
    }
}
#[cfg(all(test, target_os = "linux"))]
mod drop_hive_tests;
#[cfg(test)]
mod drop_tests;
#[cfg(test)]
mod drop_track_tests;
#[cfg(test)]
mod monitor_tests;
#[cfg(test)]
mod s017_motion_tests;
#[cfg(test)]
mod s067_drop_track_tests;
#[cfg(test)]
mod s074_tests;
#[cfg(test)]
mod s084_trigger_tests;
#[cfg(test)]
mod ui_tests;
