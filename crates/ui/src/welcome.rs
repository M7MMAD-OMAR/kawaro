//! The first-run welcome window (S-087, backlog/S-087-first-run-welcome.md):
//! the demonstration frames of `crates/ui/resources/welcome`, the caption
//! steps of [`kawaro_core::welcome`], and the two buttons. Like the settings
//! window, it never writes files: the daemon saves the marker and runs the
//! flow behind Try on an image.
//!
//! Right to left: `rtl` mirrors the heading and the button row in
//! `ui/welcome.slint`; the strings arrive from Rust in the window language.

use crate::Lang;
use crate::settings::{self, FormState};
use crate::updates::{UpdateModel, UpdateState};
use kawaro_core::config::{Config, OutputPlace};
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
type SettingsSave = Rc<dyn Fn(Config) -> bool>;
type LanguageSave = Rc<dyn Fn(Lang) -> bool>;

struct Inner {
    rtl: bool,
    lang: Lang,
    ui: RefCell<WelcomeUi>,
    on_try: RefCell<Option<Callback>>,
    on_settings: RefCell<Option<Callback>>,
    on_settings_save: RefCell<Option<SettingsSave>>,
    settings_opened: RefCell<Config>,
    on_language: RefCell<Option<LanguageSave>>,
    update: RefCell<UpdateModel>,
    on_update_check: RefCell<Option<Callback>>,
    on_update_apply: RefCell<Option<Callback>>,
    timer: slint::Timer,
    plan: RefCell<DemoPlan>,
    ticks: Cell<u64>,
    choose_language: Cell<bool>,
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
        ui.set_version_text(crate::about_line(self.lang).into());
        self.show_update(&ui, self.lang);
        let w = Rc::downgrade(self);
        ui.on_update_check(move || with(&w, Inner::update_check));
        let w = Rc::downgrade(self);
        ui.on_update_apply(move || with(&w, Inner::update_apply));
        let w = Rc::downgrade(self);
        ui.on_try_sample(move || with(&w, Inner::try_sample));
        let w = Rc::downgrade(self);
        ui.on_open_settings(move || with(&w, Inner::open_settings));
        let w = Rc::downgrade(self);
        ui.on_settings_save(move || with(&w, Inner::save_settings));
        let w = Rc::downgrade(self);
        ui.on_settings_cancel(move || with(&w, Inner::cancel_settings));
        let w = Rc::downgrade(self);
        ui.on_settings_language_selected(move || with(&w, Inner::settings_language_changed));
        let w = Rc::downgrade(self);
        ui.on_select_language(move |arabic| {
            if let Some(inner) = w.upgrade() {
                let lang = if arabic { Lang::Ar } else { Lang::En };
                let saved = inner.on_language.borrow().as_ref().is_some_and(|f| f(lang));
                inner.ui.borrow().set_language_save_failed(!saved);
            }
        });
        let w = Rc::downgrade(self);
        ui.on_dismissed(move || with(&w, Inner::close));
        let w = Rc::downgrade(self);
        ui.window().on_close_requested(move || {
            with(&w, Inner::close);
            slint::CloseRequestResponse::HideWindow
        });
        Ok(ui)
    }

    fn show_update(&self, ui: &WelcomeUi, lang: Lang) {
        let r = self.update.borrow().render(lang);
        ui.set_settings_update_version_text(r.version_text.into());
        ui.set_settings_update_state_line(r.state_line.into());
        ui.set_settings_update_check_enabled(r.check_enabled);
        ui.set_settings_update_apply_enabled(r.apply_enabled);
    }

    fn current_lang(&self) -> Lang {
        if self.ui.borrow().get_rtl() {
            Lang::Ar
        } else {
            Lang::En
        }
    }

    fn update_check(&self) {
        if !self.update.borrow_mut().begin_check() {
            return;
        }
        self.show_update(&self.ui.borrow(), self.current_lang());
        let f = self.on_update_check.borrow().clone();
        if let Some(f) = f {
            f();
        }
    }

    fn update_apply(&self) {
        if !self.update.borrow_mut().begin_apply() {
            return;
        }
        self.show_update(&self.ui.borrow(), self.current_lang());
        let f = self.on_update_apply.borrow().clone();
        if let Some(f) = f {
            f();
        }
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

    fn settings_state(&self) -> FormState {
        let ui = self.ui.borrow();
        FormState {
            language: ui.get_settings_language(),
            theme: ui.get_settings_theme(),
            output: ui.get_settings_output(),
            quality: ui.get_settings_quality(),
            folder: ui.get_settings_folder().into(),
            notify: ui.get_settings_notify(),
            notify_long_only: ui.get_settings_notify_long_only(),
            open_folder: ui.get_settings_open_folder(),
            update_check: ui.get_settings_update_check_auto(),
            update_channel: ui.get_settings_update_channel(),
            update_mode: ui.get_settings_update_mode(),
            opened: self.settings_opened.borrow().clone(),
        }
    }

    fn set_settings_state(&self, state: &FormState, lang: Lang) {
        let ui = self.ui.borrow();
        let rtl = lang.rtl();
        let strings = settings::strings(lang);
        let text = strings.text;
        ui.set_rtl(rtl);
        ui.set_settings_text(generated::SettingsText {
            title: text.title,
            language: text.language,
            theme: text.theme,
            output: text.output,
            folder: text.folder,
            quality: text.quality,
            notify: text.notify,
            notify_long_only: text.notify_long_only,
            open_folder: text.open_folder,
            save: text.save,
            cancel: text.cancel,
            updates: text.updates,
            update_check: text.update_check,
            update_apply: text.update_apply,
            update_auto: text.update_auto,
            update_channel: text.update_channel,
            update_mode: text.update_mode,
        });
        let list = |items: &[&str]| ModelRc::new(VecModel::from(settings::ordered(items, rtl)));
        ui.set_settings_language_items(list(&strings.languages));
        ui.set_settings_theme_items(list(&strings.themes));
        ui.set_settings_output_items(list(&strings.outputs));
        ui.set_settings_quality_items(list(&strings.qualities));
        ui.set_settings_update_channel_items(list(&strings.channels));
        ui.set_settings_update_mode_items(list(&strings.modes));
        self.show_update(&ui, lang);
        ui.set_settings_subfolder_index(settings::to_visual(
            &settings::OUTPUTS,
            &OutputPlace::Subfolder,
            rtl,
        ));
        ui.set_settings_language(state.language);
        ui.set_settings_theme(state.theme);
        ui.set_settings_output(state.output);
        ui.set_settings_quality(state.quality);
        ui.set_settings_folder(state.folder.as_str().into());
        ui.set_settings_notify(state.notify);
        ui.set_settings_notify_long_only(state.notify_long_only);
        ui.set_settings_open_folder(state.open_folder);
        ui.set_settings_update_check_auto(state.update_check);
        ui.set_settings_update_channel(state.update_channel);
        ui.set_settings_update_mode(state.update_mode);
    }

    fn settings_language_changed(&self) {
        let old = self.settings_state();
        let old_rtl = self.ui.borrow().get_rtl();
        let config = old.to_config(old_rtl);
        let lang = config.lang();
        let mut next = FormState::from_config(&config, lang.rtl());
        next.folder = old.folder;
        self.set_settings_state(&next, lang);
    }

    fn save_settings(&self) {
        let config = self.settings_state().to_config(self.ui.borrow().get_rtl());
        let callback = self.on_settings_save.borrow().clone();
        if callback.is_some_and(|f| f(config)) {
            self.cancel_settings();
        } else {
            self.ui.borrow().set_settings_save_failed(true);
        }
    }

    fn cancel_settings(&self) {
        let ui = self.ui.borrow();
        ui.set_show_settings(false);
        ui.set_rtl(self.rtl);
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
            on_settings_save: RefCell::new(None),
            settings_opened: RefCell::new(Config::default()),
            on_language: RefCell::new(None),
            update: RefCell::new(UpdateModel::default()),
            on_update_check: RefCell::new(None),
            on_update_apply: RefCell::new(None),
            timer: slint::Timer::default(),
            plan: RefCell::new(demo_plan(1, false)),
            ticks: Cell::new(0),
            choose_language: Cell::new(false),
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
        ui.set_choose_language(inner.choose_language.get());
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

    /// Show the language choice before the introduction.
    pub fn choose_language(&self) {
        self.inner.choose_language.set(true);
        self.inner.ui.borrow().set_choose_language(true);
    }

    /// Reports a failure to reopen after saving the language.
    pub fn language_reload_failed(&self) {
        self.inner.ui.borrow().set_language_save_failed(true);
    }

    /// Persist a chosen language. False leaves the choice visible with an error.
    pub fn on_language(&self, f: impl Fn(Lang) -> bool + 'static) {
        *self.inner.on_language.borrow_mut() = Some(Rc::new(f));
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

    /// Shows the installed `version`, the update `state` and whether the
    /// background service answers (S-090), inside the embedded settings.
    pub fn set_update_view(&self, version: &str, state: UpdateState, service_running: bool) {
        let inner = &self.inner;
        inner
            .update
            .borrow_mut()
            .set_view(version, state, service_running);
        inner.show_update(&inner.ui.borrow(), inner.current_lang());
    }

    /// Update now was accepted but waits for running jobs (S-054b AC7).
    pub fn show_update_deferred(&self) {
        let inner = &self.inner;
        inner.update.borrow_mut().set_deferred();
        inner.show_update(&inner.ui.borrow(), inner.current_lang());
    }

    /// Called when Check for updates is pressed while it is enabled.
    pub fn on_update_check(&self, f: impl Fn() + 'static) {
        *self.inner.on_update_check.borrow_mut() = Some(Rc::new(f));
    }

    /// Called when Update now is pressed while it is enabled.
    pub fn on_update_apply(&self, f: impl Fn() + 'static) {
        *self.inner.on_update_apply.borrow_mut() = Some(Rc::new(f));
    }

    /// Called when Try on an image is pressed, by pointer or keyboard.
    pub fn on_try(&self, f: impl Fn() + 'static) {
        *self.inner.on_try.borrow_mut() = Some(Rc::new(f));
    }

    /// Applies the configured theme to the main window.
    pub fn apply_theme(&self, theme: kawaro_core::config::Theme) {
        let env = std::env::var("KAWARO_COLOR_SCHEME").ok();
        if let Some(scheme) = crate::settings::forced_scheme(env.as_deref(), theme) {
            self.inner
                .ui
                .borrow()
                .global::<Palette>()
                .set_color_scheme(scheme);
        }
    }

    /// Whether the main window is showing its settings panel.
    pub fn settings_visible(&self) -> bool {
        self.inner.ui.borrow().get_show_settings()
    }

    /// Shows settings inside the existing window without replacing it.
    pub fn show_settings(&self, config: &Config) {
        *self.inner.settings_opened.borrow_mut() = config.clone();
        let lang = config.lang();
        self.inner.ui.borrow().set_settings_save_failed(false);
        self.inner
            .set_settings_state(&FormState::from_config(config, lang.rtl()), lang);
        self.inner.ui.borrow().set_show_settings(true);
    }

    /// Called when embedded settings are saved.
    pub fn on_settings_save(&self, f: impl Fn(Config) -> bool + 'static) {
        *self.inner.on_settings_save.borrow_mut() = Some(Rc::new(f));
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

#[cfg(test)]
mod unified_tests {
    use super::*;
    use i_slint_backend_testing::{AccessibleRole, ElementHandle};
    use kawaro_core::config::{LangSetting, OutputPlace};

    #[test]
    fn embedded_settings_switch_language_save_and_cancel_in_same_window() {
        i_slint_backend_testing::init_no_event_loop();
        let win = WelcomeWindow::new(Lang::En).unwrap();
        let captions = Captions {
            steps: std::array::from_fn(|_| "Step".to_owned()),
        };
        win.open(&captions, false).unwrap();
        if std::env::var_os("KAWARO_COLOR_SCHEME").is_none() {
            win.apply_theme(kawaro_core::config::Theme::Dark);
            assert_eq!(
                win.inner.ui.borrow().global::<Palette>().get_color_scheme(),
                slint::language::ColorScheme::Dark
            );
            win.apply_theme(kawaro_core::config::Theme::Light);
            assert_eq!(
                win.inner.ui.borrow().global::<Palette>().get_color_scheme(),
                slint::language::ColorScheme::Light
            );
        }
        let original = win.inner.ui.borrow().window() as *const slint::Window;
        let find_button = |label: &str| {
            let ui = win.inner.ui.borrow();
            ElementHandle::find_by_accessible_label(&*ui, label)
                .find(|e| e.accessible_role() == Some(AccessibleRole::Button))
                .unwrap()
        };
        let requested = Rc::new(Cell::new(0));
        let count = requested.clone();
        win.on_settings(move || count.set(count.get() + 1));
        find_button("Settings").invoke_accessible_default_action();
        assert_eq!(requested.get(), 1);
        let saved = Rc::new(RefCell::new(Vec::new()));
        let result = saved.clone();
        win.on_settings_save(|_| false);
        let config = Config {
            output: OutputPlace::Subfolder,
            ..Config::default()
        };
        win.show_settings(&config);
        assert!(win.settings_visible());
        find_button("Introduction").invoke_accessible_default_action();
        assert!(!win.settings_visible());
        find_button("Settings").invoke_accessible_default_action();
        assert_eq!(requested.get(), 2);
        win.show_settings(&config);
        {
            let ui = win.inner.ui.borrow();
            ui.set_settings_folder("  unchanged  ".into());
            ui.set_settings_notify_long_only(true);
            ui.set_settings_language(1);
        }
        win.inner.settings_language_changed();
        {
            let ui = win.inner.ui.borrow();
            assert!(ui.get_rtl());
            assert_eq!(ui.get_settings_folder(), "  unchanged  ");
            assert!(ui.get_settings_notify_long_only());
            assert_eq!(ui.get_settings_output(), 0);
            assert_eq!(ui.window() as *const slint::Window, original);
        }
        win.inner.save_settings();
        assert!(win.settings_visible());
        assert!(win.inner.ui.borrow().get_settings_save_failed());
        assert_eq!(win.inner.ui.borrow().get_settings_folder(), "  unchanged  ");
        win.on_settings_save(move |config| {
            result.borrow_mut().push(config);
            true
        });
        win.inner.save_settings();
        assert!(!win.settings_visible());
        assert_eq!(saved.borrow()[0].language, LangSetting::Ar);
        assert_eq!(saved.borrow()[0].output, OutputPlace::Subfolder);
        assert_eq!(saved.borrow()[0].output_folder, "unchanged");
        assert!(saved.borrow()[0].notify_long_only);
        win.show_settings(&config);
        win.inner.ui.borrow().set_settings_language(1);
        win.inner.settings_language_changed();
        let ui = win.inner.ui.borrow();
        ui.window()
            .dispatch_event(slint::platform::WindowEvent::KeyPressed {
                text: slint::platform::Key::Escape.into(),
            });
        drop(ui);
        assert_eq!(saved.borrow().len(), 1);
        assert!(!win.settings_visible());
        win.close();
    }
}

#[cfg(test)]
mod s090_tests;
