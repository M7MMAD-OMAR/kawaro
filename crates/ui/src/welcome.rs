//! The first-run welcome window (S-087, backlog/S-087-first-run-welcome.md):
//! the Introduction film (S-094c, a live vector scene driven by
//! [`crate::film`]), the caption steps of [`kawaro_core::welcome`], and the
//! two buttons. Like the settings
//! window, it never writes files: the daemon saves the marker and runs the
//! flow behind Try on an image.
//!
//! Right to left: `rtl` mirrors the heading and the button row in
//! `ui/welcome.slint`; the strings arrive from Rust in the window language.

use crate::Lang;
use crate::film;
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

use generated::{FilmPose, FormValues, Palette, Tokens, WelcomeText, WelcomeUi};

/// How the first welcome demonstration played (S-087 AC2). Kept as the pure
/// timing rule of that story; the film of S-094c plays only on request, at
/// [`crate::film::FRAME_MS`] per frame.
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
    /// Whether the film loop plays; it never starts on its own.
    playing: Cell<bool>,
    choose_language: Cell<bool>,
    /// Whether the daemon handed over the saved settings (S-094): until it
    /// has, picking a settings page in the rail asks for them.
    settings_loaded: Cell<bool>,
}

impl Inner {
    fn new_ui(self: &Rc<Self>) -> Result<WelcomeUi, slint::PlatformError> {
        let ui = WelcomeUi::new()?;
        ui.set_rtl(self.rtl);
        let strings = kawaro_core::welcome::strings(self.lang);
        let film = film_strings(self.lang);
        ui.set_text(WelcomeText {
            title: strings.title.as_str().into(),
            try_button: strings.try_button.as_str().into(),
            try_accessible: strings.try_accessible.as_str().into(),
            settings_button: strings.settings_button.as_str().into(),
            settings_accessible: strings.settings_accessible.as_str().into(),
            play_button: film.play.into(),
            pause_button: film.pause.into(),
            play_accessible: film.play_accessible.into(),
            pause_accessible: film.pause_accessible.into(),
        });
        // The shell shows its header and rail from the first frame, so the
        // settings strings go in now, over the default values; `show_settings`
        // replaces the values with the saved ones.
        self.settings_loaded.set(false);
        let defaults = FormState::from_config(&Config::default(), self.rtl);
        self.apply_settings(&ui, &defaults, self.lang);
        ui.set_settings_loaded(settings::form_values!(FormValues, &defaults));
        ui.set_settings_shortcut_text(Config::default().shortcut.as_str().into());
        self.show_update(&ui, self.lang);
        let w = Rc::downgrade(self);
        ui.on_page_changed(move |page| with(&w, |i| i.page_changed(page)));
        let w = Rc::downgrade(self);
        ui.on_update_check(move || with(&w, Inner::update_check));
        let w = Rc::downgrade(self);
        ui.on_update_apply(move || with(&w, Inner::update_apply));
        let w = Rc::downgrade(self);
        ui.on_film_toggle(move || {
            if let Some(inner) = w.upgrade() {
                inner.toggle_film();
            }
        });
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

    fn apply_settings(&self, ui: &WelcomeUi, state: &FormState, lang: Lang) {
        let rtl = lang.rtl();
        let strings = settings::strings(lang);
        ui.set_rtl(rtl);
        ui.set_settings_text(settings::convert_text!(
            generated::SettingsText,
            strings.text
        ));
        let list = |items: &[&str]| ModelRc::new(VecModel::from(settings::ordered(items, rtl)));
        ui.set_settings_language_items(list(&strings.languages));
        ui.set_settings_theme_items(list(&strings.themes));
        ui.set_settings_output_items(list(&strings.outputs));
        ui.set_settings_quality_items(list(&strings.qualities));
        ui.set_settings_update_channel_items(list(&strings.channels));
        ui.set_settings_update_mode_items(list(&strings.modes));
        self.show_update(ui, lang);
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

    fn set_loaded(&self, state: &FormState) {
        self.ui
            .borrow()
            .set_settings_loaded(settings::form_values!(FormValues, state));
    }

    /// A page was picked in the rail. A settings page needs the saved
    /// values, which only the daemon has: ask once, as Open Settings does.
    fn page_changed(&self, page: i32) {
        if page > 0 && !self.settings_loaded.get() {
            self.open_settings();
        }
    }

    #[cfg(feature = "testing-hooks")]
    fn is_rtl(&self) -> bool {
        self.ui.borrow().get_rtl()
    }

    fn settings_language_changed(&self) {
        let old = self.settings_state();
        let old_rtl = self.ui.borrow().get_rtl();
        let config = old.to_config(old_rtl);
        let lang = config.lang();
        let mut next = FormState::from_config(&config, lang.rtl());
        next.folder = old.folder;
        self.apply_settings(&self.ui.borrow(), &next, lang);
        // Same saved settings, indices of the new language.
        let opened = self.settings_opened.borrow();
        self.set_loaded(&FormState::from_config(&opened, lang.rtl()));
    }

    fn save_settings(&self) {
        let state = self.settings_state();
        let config = state.to_config(self.ui.borrow().get_rtl());
        let callback = self.on_settings_save.borrow().clone();
        if callback.is_some_and(|f| f(config.clone())) {
            *self.settings_opened.borrow_mut() = config;
            self.cancel_settings();
        } else {
            self.ui.borrow().set_settings_save_failed(true);
        }
    }

    /// Back to the Introduction: the edits are dropped and the window
    /// returns to its own language.
    fn cancel_settings(&self) {
        let ui = self.ui.borrow();
        ui.set_page(0);
        if self.settings_loaded.get() {
            let opened = self.settings_opened.borrow();
            let state = FormState::from_config(&opened, self.rtl);
            self.apply_settings(&ui, &state, self.lang);
            drop(ui);
            self.set_loaded(&state);
        } else {
            ui.set_rtl(self.rtl);
        }
    }

    fn close(&self) {
        self.timer.stop();
        self.playing.set(false);
        self.ui.borrow().set_film_playing(false);
        self.ui.borrow().hide().ok();
    }

    /// Shows frame `i` of the film: the pose and the caption rows it lights.
    fn show_frame(&self, i: usize) {
        let ui = self.ui.borrow();
        let p = film::pose(i);
        ui.set_film_frame(i32::try_from(i % film::POSES.len()).unwrap_or(0));
        ui.set_pose(FilmPose {
            step: p.step as i32,
            selected: p.selected,
            pointer: p.pointer,
            approach: p.approach,
            lift: p.lift,
            hive: p.hive,
            thumbnail: p.thumbnail,
            drag: p.drag,
            hot: p.hot,
            pressed: p.pressed,
            result: p.result,
        });
        let lit = film::active_captions(p.step);
        ui.set_lit_a(lit[0] as i32);
        ui.set_lit_b(lit.get(1).map_or(-1, |b| *b as i32));
    }

    fn frame(&self) -> usize {
        usize::try_from(self.ui.borrow().get_film_frame()).unwrap_or(0)
    }

    /// One step of the loop, exactly what the timer does; nothing moves
    /// unless the film plays.
    fn tick(&self) {
        if self.playing.get() {
            self.show_frame(self.frame() + 1);
        }
    }

    /// Play or Pause. Playing from the poster starts the flow at its first
    /// step; pausing keeps the frame on screen.
    fn toggle_film(self: &Rc<Self>) {
        if self.playing.get() {
            self.pause_film();
            return;
        }
        if self.frame() == film::poster_index() {
            self.show_frame(0);
        }
        self.playing.set(true);
        self.ui.borrow().set_film_playing(true);
        let w = Rc::downgrade(self);
        self.timer.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(film::FRAME_MS),
            move || with(&w, Inner::tick),
        );
    }

    fn pause_film(&self) {
        self.timer.stop();
        self.playing.set(false);
        self.ui.borrow().set_film_playing(false);
    }

    /// Reduced motion: a still poster, no control, nothing plays.
    fn set_reduced(&self, reduced: bool) {
        self.pause_film();
        let ui = self.ui.borrow();
        ui.set_reduced_motion(reduced);
        ui.global::<Tokens>().set_reduced_motion(reduced);
        drop(ui);
        self.show_frame(film::poster_index());
    }
}

/// The film's own strings: the Play and Pause button and their accessible
/// labels, in the window language.
struct FilmStrings {
    play: &'static str,
    pause: &'static str,
    play_accessible: &'static str,
    pause_accessible: &'static str,
}

fn film_strings(lang: Lang) -> FilmStrings {
    match lang {
        Lang::En => FilmStrings {
            play: "Play",
            pause: "Pause",
            play_accessible: "Play the introduction film",
            pause_accessible: "Pause the introduction film",
        },
        Lang::Ar => FilmStrings {
            play: "تشغيل",
            pause: "إيقاف مؤقت",
            play_accessible: "شغّل فيلم التعريف",
            pause_accessible: "أوقف فيلم التعريف مؤقتاً",
        },
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
            playing: Cell::new(false),
            choose_language: Cell::new(false),
            settings_loaded: Cell::new(false),
        });
        let ui = inner.new_ui()?;
        *inner.ui.borrow_mut() = ui;
        Ok(Self { inner })
    }

    /// Shows the window with the caption steps of `captions` and the hive
    /// placement of `centred`. The film waits on its poster.
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
        *inner.ui.borrow_mut() = ui;

        // Nothing autoplays: the film opens on its poster and waits for
        // Play. Reduced motion also hides the control.
        inner.set_reduced(crate::motion::detect());
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

    /// Whether the main window is showing one of its settings pages.
    pub fn settings_visible(&self) -> bool {
        self.inner.ui.borrow().get_page() != 0
    }

    /// Loads the saved settings into the existing window without replacing
    /// it, and leaves the Introduction for the first settings page.
    pub fn show_settings(&self, config: &Config) {
        let inner = &self.inner;
        *inner.settings_opened.borrow_mut() = config.clone();
        inner.settings_loaded.set(true);
        let lang = config.lang();
        let state = FormState::from_config(config, lang.rtl());
        let ui = inner.ui.borrow();
        ui.set_settings_save_failed(false);
        inner.apply_settings(&ui, &state, lang);
        ui.set_settings_shortcut_text(config.shortcut.as_str().into());
        if ui.get_page() == 0 {
            ui.set_page(1);
        }
        drop(ui);
        inner.set_loaded(&state);
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

    /// Snapshot hook: pins the film at `frame` and stops the clock, so the
    /// reference renders are deterministic.
    #[doc(hidden)]
    pub fn freeze(&self, frame: usize) {
        self.inner.pause_film();
        self.inner.show_frame(frame);
    }

    /// Snapshot hook: forces reduced motion on or off for the film.
    #[doc(hidden)]
    pub fn set_film_reduced(&self, reduced: bool) {
        self.inner.set_reduced(reduced);
    }

    /// Snapshot hook: shows rail item `page` (0 is the Introduction).
    #[doc(hidden)]
    pub fn show_page(&self, page: usize) {
        self.inner
            .ui
            .borrow()
            .set_page(i32::try_from(page).unwrap_or(0));
    }

    #[doc(hidden)]
    pub fn press_save(&self) {
        self.inner.ui.borrow().invoke_settings_save();
    }

    #[doc(hidden)]
    pub fn press_cancel(&self) {
        self.inner.ui.borrow().invoke_settings_cancel();
    }

    /// Snapshot hook: renders the current window content.
    #[doc(hidden)]
    pub fn take_snapshot(
        &self,
    ) -> Result<slint::SharedPixelBuffer<slint::Rgba8Pixel>, slint::PlatformError> {
        self.inner.ui.borrow().window().take_snapshot()
    }
}

#[cfg(feature = "testing-hooks")]
impl WelcomeWindow {
    /// The frame of `film::frames()` now shown.
    #[doc(hidden)]
    pub fn film_frame(&self) -> usize {
        self.inner.frame()
    }

    #[doc(hidden)]
    pub fn film_step(&self) -> usize {
        film::pose(self.inner.frame()).step
    }

    #[doc(hidden)]
    pub fn film_is_poster(&self) -> bool {
        self.inner.frame() == film::poster_index()
    }

    #[doc(hidden)]
    pub fn film_playing(&self) -> bool {
        self.inner.playing.get()
    }

    #[doc(hidden)]
    pub fn film_timer_running(&self) -> bool {
        self.inner.timer.running()
    }

    /// What the timer does, only while playing.
    #[doc(hidden)]
    pub fn film_tick(&self) {
        self.inner.tick();
    }

    #[doc(hidden)]
    pub fn press_film_play_pause(&self) {
        self.inner.ui.borrow().invoke_film_toggle();
    }

    /// The accessible label of the Play and Pause button.
    #[doc(hidden)]
    pub fn film_button_label(&self) -> String {
        let f = film_strings(self.inner.lang);
        if self.inner.playing.get() {
            f.pause_accessible
        } else {
            f.play_accessible
        }
        .to_owned()
    }

    #[doc(hidden)]
    pub fn set_reduced_motion(&self, reduced: bool) {
        self.inner.set_reduced(reduced);
    }

    /// The card with the film, the button and the captions.
    #[doc(hidden)]
    pub fn film_card_rect(&self) -> crate::Rect {
        crate::shell::probe::rect_of(&*self.inner.ui.borrow(), "FilmCard::card")
    }

    /// Sets the window scale factor, keeping the logical size.
    #[doc(hidden)]
    pub fn set_scale_factor(&self, scale: f32) {
        let ui = self.inner.ui.borrow();
        let w = ui.window();
        let logical = w.size().to_logical(w.scale_factor());
        w.dispatch_event(slint::platform::WindowEvent::ScaleFactorChanged {
            scale_factor: scale,
        });
        w.set_size(logical);
    }

    /// The pixel size the film is drawn at: the film is vector art drawn at
    /// the displayed size, so this is that size times the scale factor.
    #[doc(hidden)]
    pub fn film_render_pixels(&self) -> (u32, u32) {
        let scale = self.inner.ui.borrow().window().scale_factor();
        self.film_rect().map_or((0, 0), |r| {
            ((r.w * scale).round() as u32, (r.h * scale).round() as u32)
        })
    }
}

#[cfg(feature = "testing-hooks")]
crate::shell::shell_probes!(WelcomeWindow, |w, i| w
    .inner
    .ui
    .borrow()
    .set_settings_theme(i));

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
        find_button("Open the Kawaro settings window").invoke_accessible_default_action();
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
        win.press_cancel();
        assert!(!win.settings_visible());
        find_button("Open the Kawaro settings window").invoke_accessible_default_action();
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
