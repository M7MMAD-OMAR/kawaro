//! The first-run welcome window (S-087, backlog/S-087-first-run-welcome.md):
//! the demonstration frames of `crates/ui/resources/welcome`, the caption
//! steps of [`kawaro_core::welcome`], and the two buttons. Like the settings
//! window, it never writes files: the daemon saves the marker and runs the
//! flow behind Try on an image.
//!
//! Right to left: `rtl` mirrors the heading and the button row in
//! `ui/welcome.slint`; the strings arrive from Rust in the window language.

use crate::Lang;
use kawaro_core::welcome::Captions;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};
use std::time::Duration;

mod generated {
    include!(concat!(env!("OUT_DIR"), "/welcome_ui.rs"));
}

mod resources {
    include!(concat!(env!("OUT_DIR"), "/welcome_resources.rs"));
}

use generated::{Palette, WelcomeText, WelcomeUi};

/// How the demonstration plays (AC2): the frames loop while the caption
/// steps cycle, and reduced motion shows one static frame with the same
/// caption steps instead of looping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DemoPlan {
    /// Frames to show: the full set, or one static frame.
    pub frames: usize,
    /// Milliseconds per frame; 0 when nothing moves.
    pub frame_interval_ms: u64,
    /// The caption steps under the demonstration.
    pub caption_steps: usize,
    /// Milliseconds per caption step.
    pub caption_interval_ms: u64,
}

/// The caption steps always show long enough to read; the moving loop runs
/// one full flow per caption step.
pub fn demo_plan(total_frames: usize, reduced_motion: bool) -> DemoPlan {
    if reduced_motion {
        DemoPlan {
            frames: 1,
            frame_interval_ms: 0,
            caption_steps: 5,
            caption_interval_ms: 2400,
        }
    } else {
        DemoPlan {
            frames: total_frames.max(1),
            frame_interval_ms: 150,
            caption_steps: 5,
            caption_interval_ms: 2400,
        }
    }
}

/// Whether the demonstration shows the hive centred: AC2 shows the hive at
/// the pointer wherever the platform puts it there (Windows, X11 and the
/// layer path) and centred where it does not (the Wayland window path).
/// Pure, so the rule is tested without a compositor.
pub fn demo_centred(wayland: bool, layer_surface: bool) -> bool {
    wayland && !layer_surface
}

/// The placement rule of [`demo_centred`] answered for this session: the
/// same layer decision the hive uses (`crate::layer::wanted`).
pub fn demo_hive_centred() -> bool {
    #[cfg(target_os = "linux")]
    {
        demo_centred(
            std::env::var_os("WAYLAND_DISPLAY").is_some(),
            crate::layer::wanted(),
        )
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// The bundled sample image (AC4), one embedded copy in the binary.
pub fn sample_bytes() -> &'static [u8] {
    resources::SAMPLE
}

/// Writes the bundled sample into `dir` as `sample.png` (AC4): a fresh,
/// untouched copy on every click. The caller passes a writable temporary
/// folder.
pub fn install_sample(dir: &Path) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join("sample.png");
    std::fs::write(&path, sample_bytes())?;
    Ok(path)
}

type Callback = Rc<dyn Fn()>;

struct Inner {
    rtl: bool,
    lang: Lang,
    ui: RefCell<WelcomeUi>,
    on_try: RefCell<Option<Callback>>,
    on_settings: RefCell<Option<Callback>>,
    timer: slint::Timer,
    plan: RefCell<DemoPlan>,
    ticks: Cell<u64>,
}

impl Inner {
    fn new_ui(self: &Rc<Self>) -> Result<WelcomeUi, slint::PlatformError> {
        let ui = WelcomeUi::new()?;
        ui.set_rtl(self.rtl);
        let strings = kawaro_core::welcome::strings(self.lang);
        ui.set_text(WelcomeText {
            title: strings.title.as_str().into(),
            try_button: strings.try_button.as_str().into(),
            try_accessible: strings.try_accessible.as_str().into(),
            settings_button: strings.settings_button.as_str().into(),
            settings_accessible: strings.settings_accessible.as_str().into(),
        });
        let w = Rc::downgrade(self);
        ui.on_try_sample(move || with(&w, Inner::try_sample));
        let w = Rc::downgrade(self);
        ui.on_open_settings(move || with(&w, Inner::open_settings));
        let w = Rc::downgrade(self);
        ui.on_dismissed(move || with(&w, Inner::close));
        let w = Rc::downgrade(self);
        ui.window().on_close_requested(move || {
            with(&w, Inner::close);
            slint::CloseRequestResponse::HideWindow
        });
        Ok(ui)
    }

    fn visible(&self) -> bool {
        self.ui.borrow().window().is_visible()
    }

    fn try_sample(&self) {
        let f = self.on_try.borrow().clone();
        if let Some(f) = f {
            f();
        }
    }

    fn open_settings(&self) {
        let f = self.on_settings.borrow().clone();
        if let Some(f) = f {
            f();
        }
    }

    fn close(&self) {
        self.timer.stop();
        self.ui.borrow().hide().ok();
    }

    /// One playback step: the frame advances under the caption steps, and
    /// reduced motion only steps the captions (AC2).
    fn tick(&self) {
        let ui = self.ui.borrow();
        let plan = *self.plan.borrow();
        let ticks = self.ticks.get() + 1;
        self.ticks.set(ticks);
        let steps = i32::try_from(plan.caption_steps).unwrap_or(1).max(1);
        if plan.frame_interval_ms > 0 {
            let frames = ui.get_frame_count().max(1);
            ui.set_frame_index((ui.get_frame_index() + 1) % frames);
        }
        // A caption step lasts one loop of the frames, and a static demo
        // steps the captions on every tick (no frame interval divides).
        let caption_every = plan
            .caption_interval_ms
            .checked_div(plan.frame_interval_ms)
            .unwrap_or(1)
            .max(1);
        if ticks.is_multiple_of(caption_every) {
            ui.set_caption_index((ui.get_caption_index() + 1) % steps);
        }
    }
}

fn with(w: &Weak<Inner>, f: impl FnOnce(&Inner)) {
    if let Some(i) = w.upgrade() {
        f(&i);
    }
}

/// The welcome window. One per process; `open` shows it again with fresh
/// captions. The demonstration plays from the frame set embedded in
/// `ui/welcome.slint`.
pub struct WelcomeWindow {
    inner: Rc<Inner>,
}

impl WelcomeWindow {
    /// The window language is fixed here; a language change applies the next
    /// time the daemon creates the window.
    pub fn new(lang: Lang) -> Result<Self, slint::PlatformError> {
        let ui = WelcomeUi::new()?;
        let inner = Rc::new(Inner {
            rtl: lang.rtl(),
            lang,
            ui: RefCell::new(ui),
            on_try: RefCell::new(None),
            on_settings: RefCell::new(None),
            timer: slint::Timer::default(),
            plan: RefCell::new(demo_plan(1, false)),
            ticks: Cell::new(0),
        });
        let ui = inner.new_ui()?;
        *inner.ui.borrow_mut() = ui;
        Ok(Self { inner })
    }

    /// Shows the window with the caption steps of `captions` and the hive
    /// placement of `centred`, and starts the demonstration.
    pub fn open(&self, captions: &Captions, centred: bool) -> Result<(), slint::PlatformError> {
        let inner = &self.inner;
        inner.close();
        let ui = inner.new_ui()?;
        let env = std::env::var("KAWARO_COLOR_SCHEME").ok();
        if let Some(scheme) = forced_scheme(env.as_deref()) {
            ui.global::<Palette>().set_color_scheme(scheme);
        }
        ui.set_centred(centred);
        ui.set_caption_steps(ModelRc::new(VecModel::from(
            captions
                .steps
                .iter()
                .map(|s| SharedString::from(s.as_str()))
                .collect::<Vec<_>>(),
        )));
        ui.set_caption_index(0);
        ui.set_frame_index(0);
        *inner.ui.borrow_mut() = ui;
        inner.ticks.set(0);

        // The demonstration: one timer step per frame under reduced motion's
        // caption steps (AC2).
        let frames = self.inner.ui.borrow().get_frame_count();
        let reduced = crate::motion::detect();
        let plan = demo_plan(usize::try_from(frames).unwrap_or(1).max(1), reduced);
        *inner.plan.borrow_mut() = plan;
        let interval = if plan.frame_interval_ms > 0 {
            plan.frame_interval_ms
        } else {
            plan.caption_interval_ms
        };
        let w = Rc::downgrade(inner);
        inner.timer.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(interval),
            move || with(&w, Inner::tick),
        );
        inner.ui.borrow().show()
    }

    /// Whether the window is currently shown.
    pub fn is_visible(&self) -> bool {
        self.inner.visible()
    }

    /// The native handle of the shown window, for the foreground call.
    #[cfg(windows)]
    pub fn hwnd(&self) -> Option<isize> {
        crate::win_window::hwnd(self.inner.ui.borrow().window())
    }

    /// Called when Try on an image is pressed, by pointer or keyboard.
    pub fn on_try(&self, f: impl Fn() + 'static) {
        *self.inner.on_try.borrow_mut() = Some(Rc::new(f));
    }

    /// Called when Open Settings is pressed.
    pub fn on_settings(&self, f: impl Fn() + 'static) {
        *self.inner.on_settings.borrow_mut() = Some(Rc::new(f));
    }

    /// Hides the window and stops the demonstration.
    pub fn close(&self) {
        self.inner.close();
    }

    /// Snapshot hook: pins the demonstration at `frame` and `caption` and
    /// stops the clock, so the reference renders are deterministic.
    #[doc(hidden)]
    pub fn freeze(&self, frame: i32, caption: i32) {
        self.inner.timer.stop();
        let ui = self.inner.ui.borrow();
        ui.set_frame_index(frame);
        ui.set_caption_index(caption);
    }

    /// Snapshot hook: renders the current window content.
    #[doc(hidden)]
    pub fn take_snapshot(
        &self,
    ) -> Result<slint::SharedPixelBuffer<slint::Rgba8Pixel>, slint::PlatformError> {
        self.inner.ui.borrow().window().take_snapshot()
    }
}

/// Colour scheme the window is drawn in: `KAWARO_COLOR_SCHEME` first (as for
/// the hive); `None` follows the system.
fn forced_scheme(env: Option<&str>) -> Option<slint::language::ColorScheme> {
    use slint::language::ColorScheme;
    match env {
        Some("light") => Some(ColorScheme::Light),
        Some("dark") => Some(ColorScheme::Dark),
        _ => None,
    }
}
