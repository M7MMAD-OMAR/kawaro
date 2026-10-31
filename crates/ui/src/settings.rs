//! The settings window (story S-011). Material 3 components from the vendored
//! library in `crates/ui/material/`, colours from `design/tokens.json`
//! (docs/adr/0002-settings-components.md).
//!
//! The window edits a [`Config`] and hands it to [`SettingsWindow::on_save`];
//! it never writes files, the daemon saves. It runs on the same event loop as
//! the hive, after [`crate::init`].
//!
//! Right to left: every option list is shown in reading order, so for Arabic
//! the lists are reversed and the visual index of an option differs from its
//! reading index. [`FormState`] holds visual indices; the conversions below
//! are the only place that knows the order.

use crate::Lang;
use crate::shell::{privacy_statement, remember_page, remembered_page};
use crate::updates::{POLL_TICK, UpdateModel, UpdateState, about_line, clock_ms};
use kawaro_core::config::{Config, LangSetting, OutputPlace, Quality, Theme, UpdateMode};
use kawaro_core::update::Channel;
use kawaro_core::update::manifest::Notes;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

pub(crate) mod generated {
    include!(concat!(env!("OUT_DIR"), "/settings_ui.rs"));
}

use generated::{FormValues, Palette, SettingsText, SettingsUi, UpdateCard};

/// Options in reading order, as shown to an English reader from left to right.
pub(crate) const LANGUAGES: [LangSetting; 3] =
    [LangSetting::Auto, LangSetting::Ar, LangSetting::En];
pub(crate) const THEMES: [Theme; 3] = [Theme::Auto, Theme::Light, Theme::Dark];
pub(crate) const OUTPUTS: [OutputPlace; 2] = [OutputPlace::Beside, OutputPlace::Subfolder];
pub(crate) const QUALITIES: [Quality; 3] = [Quality::Light, Quality::Balanced, Quality::Max];
pub(crate) const CHANNELS: [Channel; 2] = [Channel::Stable, Channel::Beta];
pub(crate) const MODES: [UpdateMode; 2] = [UpdateMode::Ask, UpdateMode::Auto];

/// Visual position of the option at reading position `i` among `n`.
fn visual(i: usize, n: usize, rtl: bool) -> usize {
    if rtl { n - 1 - i } else { i }
}

/// Visual index of `value` in `options`.
pub(crate) fn to_visual<T: PartialEq>(options: &[T], value: &T, rtl: bool) -> i32 {
    let i = options.iter().position(|o| o == value).unwrap_or(0);
    visual(i, options.len(), rtl) as i32
}

/// Option at visual index `index`; the type's default when out of range.
fn from_visual<T: Copy + Default>(options: &[T], index: i32, rtl: bool) -> T {
    usize::try_from(index)
        .ok()
        .filter(|&i| i < options.len())
        .map(|i| options[visual(i, options.len(), rtl)])
        .unwrap_or_default()
}

/// `items` in visual order.
pub(crate) fn ordered(items: &[&str], rtl: bool) -> Vec<SharedString> {
    let mut v: Vec<SharedString> = items.iter().map(|s| (*s).into()).collect();
    if rtl {
        v.reverse();
    }
    v
}

/// What the window shows: visual indices for the option lists, plus the
/// complete opened Config so hidden settings survive Save unchanged.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FormState {
    pub(crate) language: i32,
    pub(crate) theme: i32,
    pub(crate) output: i32,
    pub(crate) quality: i32,
    pub(crate) folder: String,
    pub(crate) notify: bool,
    pub(crate) notify_long_only: bool,
    pub(crate) open_folder: bool,
    pub(crate) update_check: bool,
    pub(crate) update_channel: i32,
    pub(crate) update_mode: i32,
    pub(crate) opened: Config,
}

impl FormState {
    pub(crate) fn from_config(c: &Config, rtl: bool) -> Self {
        Self {
            language: to_visual(&LANGUAGES, &c.language, rtl),
            theme: to_visual(&THEMES, &c.theme, rtl),
            output: to_visual(&OUTPUTS, &c.output, rtl),
            quality: to_visual(&QUALITIES, &c.quality, rtl),
            folder: c.output_folder.clone(),
            notify: c.notify,
            notify_long_only: c.notify_long_only,
            open_folder: c.open_folder_when_done,
            update_check: c.update_check,
            update_channel: to_visual(&CHANNELS, &c.update_channel, rtl),
            update_mode: to_visual(&MODES, &c.update_mode, rtl),
            opened: c.clone(),
        }
    }

    /// The folder name is trimmed; an empty one goes back to the default.
    pub(crate) fn to_config(&self, rtl: bool) -> Config {
        let folder = self.folder.trim();
        let channel = from_visual(&CHANNELS, self.update_channel, rtl);
        Config {
            language: from_visual(&LANGUAGES, self.language, rtl),
            theme: from_visual(&THEMES, self.theme, rtl),
            output: from_visual(&OUTPUTS, self.output, rtl),
            quality: from_visual(&QUALITIES, self.quality, rtl),
            output_folder: if folder.is_empty() {
                Config::default().output_folder
            } else {
                folder.to_string()
            },
            open_folder_when_done: self.open_folder,
            notify: self.notify,
            notify_long_only: self.notify_long_only,
            update_check: self.update_check,
            update_channel: channel,
            // Only a changed channel becomes an explicit key (S-053a).
            update_channel_present: self.opened.update_channel_present
                || channel != self.opened.update_channel,
            update_mode: from_visual(&MODES, self.update_mode, rtl),
            ..self.opened.clone()
        }
    }
}

/// Copies the strings of one generated `SettingsText` into another generated
/// type with the same fields: the settings window and the welcome window each
/// compile `ui/shell.slint` into their own Rust module (S-094).
macro_rules! convert_text {
    ($ty:path, $src:expr) => {{
        let t = $src;
        $ty {
            title: t.title,
            language: t.language,
            theme: t.theme,
            output: t.output,
            folder: t.folder,
            quality: t.quality,
            notify: t.notify,
            notify_long_only: t.notify_long_only,
            open_folder: t.open_folder,
            save: t.save,
            cancel: t.cancel,
            updates: t.updates,
            update_check: t.update_check,
            update_apply: t.update_apply,
            update_auto: t.update_auto,
            update_channel: t.update_channel,
            update_mode: t.update_mode,
            app_name: t.app_name,
            version: t.version,
            logo_label: t.logo_label,
            nav_intro: t.nav_intro,
            nav_general: t.nav_general,
            nav_conversion: t.nav_conversion,
            nav_notifications: t.nav_notifications,
            nav_about: t.nav_about,
            group_language: t.group_language,
            group_results: t.group_results,
            group_finish: t.group_finish,
            group_integration: t.group_integration,
            group_update_settings: t.group_update_settings,
            folder_hint: t.folder_hint,
            drop_shortcut: t.drop_shortcut,
            unsaved: t.unsaved,
            save_failed: t.save_failed,
            about_version: t.about_version,
            about_description: t.about_description,
            licence_label: t.licence_label,
            licence: t.licence,
            privacy: t.privacy,
            about_website: t.about_website,
            about_source: t.about_source,
            about_copy: t.about_copy,
            about_copied: t.about_copied,
            about_check: t.about_check,
            about_website_label: t.about_website_label,
            about_source_label: t.about_source_label,
        }
    }};
}
pub(crate) use convert_text;

/// The values of a [`FormState`] as the generated `FormValues` of a window.
macro_rules! form_values {
    ($ty:path, $state:expr) => {{
        let s = $state;
        $ty {
            language: s.language,
            theme: s.theme,
            output: s.output,
            quality: s.quality,
            folder: s.folder.as_str().into(),
            notify: s.notify,
            notify_long_only: s.notify_long_only,
            open_folder: s.open_folder,
            update_check: s.update_check,
            update_channel: s.update_channel,
            update_mode: s.update_mode,
        }
    }};
}
pub(crate) use form_values;

/// The cards of an update view as the generated `UpdateCard` model of a window.
macro_rules! update_cards {
    ($ty:ident, $cards:expr) => {{
        let cards: &[crate::updates::Card] = $cards;
        let mut out = Vec::new();
        for c in cards {
            let bullets: Vec<slint::SharedString> =
                c.bullets.iter().map(|b| b.as_str().into()).collect();
            let card = $ty {
                title: c.title.as_str().into(),
                label: c.label.as_str().into(),
                bullets: slint::ModelRc::new(slint::VecModel::from(bullets)),
            };
            out.push(card);
        }
        slint::ModelRc::new(slint::VecModel::from(out))
    }};
}
pub(crate) use update_cards;

/// All strings of the window in reading order.
pub(crate) struct Strings {
    pub(crate) text: SettingsText,
    pub(crate) languages: [&'static str; 3],
    pub(crate) themes: [&'static str; 3],
    pub(crate) outputs: [&'static str; 2],
    pub(crate) qualities: [&'static str; 3],
    pub(crate) channels: [&'static str; 2],
    pub(crate) modes: [&'static str; 2],
}

/// The header and About name, one string in both languages (S-094 AC1).
pub(crate) const APP_NAME: &str = "كوارو | Kawaro";

/// Automatic install exists only on Windows (S-054b AC4); elsewhere the
/// choice is kept but the update still waits for the user, and the label says so.
#[cfg(windows)]
const AUTO_EN: &str = "Automatic";
#[cfg(not(windows))]
const AUTO_EN: &str = "Automatic (Windows only)";
#[cfg(windows)]
const AUTO_AR: &str = "تلقائي";
#[cfg(not(windows))]
const AUTO_AR: &str = "تلقائي (ويندوز فقط)";

fn about_text(lang: Lang) -> kawaro_core::about::AboutStrings {
    kawaro_core::about::strings(lang)
}

pub(crate) fn strings(lang: Lang) -> Strings {
    match lang {
        Lang::Ar => Strings {
            text: SettingsText {
                title: "إعدادات كوارو".into(),
                language: "اللغة".into(),
                theme: "المظهر".into(),
                output: "مكان النتائج".into(),
                folder: "اسم المجلد".into(),
                quality: "الجودة".into(),
                notify: "إشعار عند انتهاء ملف أو عملية".into(),
                notify_long_only: "فقط إن استغرق 10 ثوانٍ أو أكثر".into(),
                open_folder: "فتح المجلد عند الانتهاء".into(),
                save: "حفظ".into(),
                cancel: "إلغاء".into(),
                updates: "التحديثات".into(),
                update_check: "ابحث عن تحديث".into(),
                update_apply: "حدّث الآن".into(),
                update_auto: "ابحث عن التحديثات تلقائيًا".into(),
                update_channel: "قناة التحديث".into(),
                update_mode: "تثبيت التحديثات".into(),
                app_name: APP_NAME.into(),
                version: env!("CARGO_PKG_VERSION").into(),
                logo_label: "شعار كوارو".into(),
                nav_intro: "المقدمة".into(),
                nav_general: "عام".into(),
                nav_conversion: "التحويل".into(),
                nav_notifications: "الإشعارات والتكامل".into(),
                nav_about: "حول".into(),
                group_language: "اللغة والمظهر".into(),
                group_results: "النتائج".into(),
                group_finish: "عند انتهاء المهمة".into(),
                group_integration: "التكامل".into(),
                group_update_settings: "إعدادات التحديث".into(),
                folder_hint: "يُستخدم عند حفظ النتائج في مجلد فرعي.".into(),
                drop_shortcut: "اختصار الإفلات".into(),
                unsaved: "تغييرات غير محفوظة".into(),
                save_failed: "تعذر حفظ الإعدادات، حاول مرة أخرى.".into(),
                about_version: about_line(Lang::Ar).into(),
                about_description: "حوّل الملفات وعالجها على جهازك.".into(),
                licence_label: "الرخصة".into(),
                licence: env!("CARGO_PKG_LICENSE").into(),
                privacy: privacy_statement(Lang::Ar).into(),
                about_website: about_text(Lang::Ar).website.into(),
                about_source: about_text(Lang::Ar).source.into(),
                about_copy: about_text(Lang::Ar).copy_version.into(),
                about_copied: about_text(Lang::Ar).copied.into(),
                about_check: about_text(Lang::Ar).check_updates.into(),
                about_website_label: about_text(Lang::Ar).website_label.into(),
                about_source_label: about_text(Lang::Ar).source_label.into(),
            },
            languages: ["تلقائي", "العربية", "English"],
            themes: ["تلقائي", "فاتح", "داكن"],
            outputs: ["بجانب الملف الأصلي", "في مجلد فرعي"],
            qualities: ["ملفات أصغر", "متوازنة", "أفضل جودة"],
            channels: ["مستقرة", "تجريبية"],
            modes: ["اسألني", AUTO_AR],
        },
        Lang::En => Strings {
            text: SettingsText {
                title: "Kawaro settings".into(),
                language: "Language".into(),
                theme: "Theme".into(),
                output: "Where results go".into(),
                folder: "Folder name".into(),
                quality: "Quality".into(),
                notify: "Notify when a file or operation finishes".into(),
                notify_long_only: "Only if it took 10 seconds or more".into(),
                open_folder: "Open the folder when done".into(),
                save: "Save".into(),
                cancel: "Cancel".into(),
                updates: "Updates".into(),
                update_check: "Check for updates".into(),
                update_apply: "Update now".into(),
                update_auto: "Check automatically".into(),
                update_channel: "Update channel".into(),
                update_mode: "Install updates".into(),
                app_name: APP_NAME.into(),
                version: env!("CARGO_PKG_VERSION").into(),
                logo_label: "Kawaro logo".into(),
                nav_intro: "Introduction".into(),
                nav_general: "General".into(),
                nav_conversion: "Conversion".into(),
                nav_notifications: "Notifications and integration".into(),
                nav_about: "About".into(),
                group_language: "Language and appearance".into(),
                group_results: "Results".into(),
                group_finish: "When a job finishes".into(),
                group_integration: "Integration".into(),
                group_update_settings: "Update settings".into(),
                folder_hint: "Used when results go in a subfolder.".into(),
                drop_shortcut: "Drop shortcut".into(),
                unsaved: "Unsaved changes".into(),
                save_failed: "Could not save settings. Please try again.".into(),
                about_version: about_line(Lang::En).into(),
                about_description: "Convert and process files on your own computer.".into(),
                licence_label: "Licence".into(),
                licence: env!("CARGO_PKG_LICENSE").into(),
                privacy: privacy_statement(Lang::En).into(),
                about_website: about_text(Lang::En).website.into(),
                about_source: about_text(Lang::En).source.into(),
                about_copy: about_text(Lang::En).copy_version.into(),
                about_copied: about_text(Lang::En).copied.into(),
                about_check: about_text(Lang::En).check_updates.into(),
                about_website_label: about_text(Lang::En).website_label.into(),
                about_source_label: about_text(Lang::En).source_label.into(),
            },
            languages: ["Automatic", "العربية", "English"],
            themes: ["Automatic", "Light", "Dark"],
            outputs: ["Next to the original", "In a subfolder"],
            qualities: ["Smaller files", "Balanced", "Best quality"],
            channels: ["Stable", "Beta"],
            modes: ["Ask me", AUTO_EN],
        },
    }
}

/// Colour scheme the window is drawn in: `KAWARO_COLOR_SCHEME` first (as
/// for the hive), then the theme being edited. `None` follows the system.
pub(crate) fn forced_scheme(
    env: Option<&str>,
    theme: Theme,
) -> Option<slint::language::ColorScheme> {
    use slint::language::ColorScheme;
    match (env, theme) {
        (Some("light"), _) => Some(ColorScheme::Light),
        (Some("dark"), _) => Some(ColorScheme::Dark),
        (_, Theme::Light) => Some(ColorScheme::Light),
        (_, Theme::Dark) => Some(ColorScheme::Dark),
        (_, Theme::Auto) => None,
    }
}

type SaveFn = Rc<dyn Fn(Config)>;
type Callback = Rc<dyn Fn()>;

struct Inner {
    rtl: Cell<bool>,
    initial_lang: Lang,
    ui: RefCell<SettingsUi>,
    on_save: RefCell<Option<SaveFn>>,
    update: RefCell<UpdateModel>,
    on_update_check: RefCell<Option<Callback>>,
    on_update_apply: RefCell<Option<Callback>>,
    pub(crate) about: crate::shell::AboutHandlers,
    /// Fires `poll_tick` while the window is open.
    timer: slint::Timer,
    /// The complete [`Config`] the window was opened with. `hive_surface`
    /// and `hold_ms` have no control here (S-019 decision 4, S-019e AC18),
    /// and `portal_shortcut` is hidden state kept as opened, including a
    /// remembered denial (S-076a AC3, AC4): a save changes only the fields
    /// the form shows and preserves every other opened value.
    opened: RefCell<Config>,
}

impl Inner {
    fn new_ui(self: &Rc<Self>) -> Result<SettingsUi, slint::PlatformError> {
        let ui = SettingsUi::new()?;
        self.apply_language(&ui, if self.rtl.get() { Lang::Ar } else { Lang::En });

        let w = Rc::downgrade(self);
        ui.on_save(move || with(&w, Inner::save));
        let w = Rc::downgrade(self);
        ui.on_language_selected(move || with(&w, Inner::change_language));
        let w = Rc::downgrade(self);
        ui.on_cancel(move || with(&w, Inner::close));
        let w = Rc::downgrade(self);
        ui.on_update_check(move || with(&w, Inner::update_check));
        let w = Rc::downgrade(self);
        ui.on_update_apply(move || with(&w, Inner::update_apply));
        let w = Rc::downgrade(self);
        ui.on_open_link(move |id| with(&w, |i| i.about.open_link(&id)));
        let w = Rc::downgrade(self);
        ui.on_copy_version(move || with(&w, |i| i.about.copy_version()));
        // The page the user picked is remembered while the process runs.
        ui.on_page_changed(|page| remember_page(usize::try_from(page).unwrap_or(0)));
        let w = Rc::downgrade(self);
        ui.window().on_close_requested(move || {
            with(&w, Inner::close);
            slint::CloseRequestResponse::HideWindow
        });
        Ok(ui)
    }

    fn apply_language(&self, ui: &SettingsUi, lang: Lang) {
        let rtl = lang.rtl();
        let s = strings(lang);
        let list = |items: &[&str]| ModelRc::new(VecModel::from(ordered(items, rtl)));
        ui.set_rtl(rtl);
        ui.set_text(s.text);
        ui.set_language_items(list(&s.languages));
        ui.set_theme_items(list(&s.themes));
        ui.set_output_items(list(&s.outputs));
        ui.set_quality_items(list(&s.qualities));
        ui.set_subfolder_index(to_visual(&OUTPUTS, &OutputPlace::Subfolder, rtl));
        ui.set_update_channel_items(list(&s.channels));
        ui.set_update_mode_items(list(&s.modes));
        self.show_update(ui, lang);
    }

    fn show_update(&self, ui: &SettingsUi, lang: Lang) {
        let r = self.update.borrow().render(lang);
        ui.set_update_version_text(r.version_text.into());
        ui.set_update_available_text(r.available_text.into());
        ui.set_update_state_line(r.state_line.into());
        ui.set_update_banner(r.banner.into());
        ui.set_update_banner_failed(r.banner_failed);
        ui.set_update_auto_text(crate::updates::auto_notice(lang).into());
        ui.set_update_auto_index(to_visual(&MODES, &UpdateMode::Auto, lang.rtl()));
        ui.set_update_cards(update_cards!(UpdateCard, &r.cards));
        ui.set_update_check_enabled(r.check_enabled);
        ui.set_update_apply_enabled(r.apply_enabled);
    }

    fn lang(&self) -> Lang {
        if self.rtl.get() { Lang::Ar } else { Lang::En }
    }

    #[cfg(feature = "testing-hooks")]
    fn is_rtl(&self) -> bool {
        self.rtl.get()
    }

    fn update_check(&self) {
        if !self.update.borrow_mut().begin_check() {
            return;
        }
        self.show_update(&self.ui.borrow(), self.lang());
        let f = self.on_update_check.borrow().clone();
        if let Some(f) = f {
            f();
        }
    }

    /// One tick of the poll (S-099 B3): while the update downloads or
    /// installs, asks the daemon again once a second. A closed window asks
    /// nothing.
    fn poll_tick(&self, now_ms: u64) {
        let fire = self.visible() && self.update.borrow_mut().poll(now_ms);
        if !fire {
            return;
        }
        let f = self.on_update_check.borrow().clone();
        if let Some(f) = f {
            f();
        }
    }

    fn update_apply(&self) {
        if !self.update.borrow_mut().begin_apply() {
            return;
        }
        self.show_update(&self.ui.borrow(), self.lang());
        let f = self.on_update_apply.borrow().clone();
        if let Some(f) = f {
            f();
        }
    }

    fn change_language(&self) {
        let state = self.state();
        let config = state.to_config(self.rtl.get());
        let lang = config.lang();
        self.rtl.set(lang.rtl());
        self.apply_language(&self.ui.borrow(), lang);
        let mut translated = FormState::from_config(&config, lang.rtl());
        translated.folder = state.folder;
        self.set_state(&translated);
        // The loaded values follow the language: same settings, new indices.
        let opened = self.opened.borrow();
        self.set_loaded(&FormState::from_config(&opened, lang.rtl()));
    }

    fn state(&self) -> FormState {
        let ui = self.ui.borrow();
        FormState {
            language: ui.get_language(),
            theme: ui.get_theme(),
            output: ui.get_output(),
            quality: ui.get_quality(),
            folder: ui.get_folder().into(),
            notify: ui.get_notify(),
            notify_long_only: ui.get_notify_long_only(),
            open_folder: ui.get_open_folder(),
            update_check: ui.get_update_check_auto(),
            update_channel: ui.get_update_channel(),
            update_mode: ui.get_update_mode(),
            opened: self.opened.borrow().clone(),
        }
    }

    fn set_state(&self, s: &FormState) {
        let ui = self.ui.borrow();
        ui.set_language(s.language);
        ui.set_theme(s.theme);
        ui.set_output(s.output);
        ui.set_quality(s.quality);
        ui.set_folder(s.folder.as_str().into());
        ui.set_notify(s.notify);
        ui.set_notify_long_only(s.notify_long_only);
        ui.set_open_folder(s.open_folder);
        ui.set_update_check_auto(s.update_check);
        ui.set_update_channel(s.update_channel);
        ui.set_update_mode(s.update_mode);
    }

    /// The values the unsaved marker compares against.
    fn set_loaded(&self, s: &FormState) {
        self.ui.borrow().set_loaded(form_values!(FormValues, s));
    }

    fn visible(&self) -> bool {
        self.ui.borrow().window().is_visible()
    }

    fn save(&self) {
        if !self.visible() {
            return;
        }
        let state = self.state();
        let config = state.to_config(self.rtl.get());
        self.set_loaded(&state);
        let f = self.on_save.borrow().clone();
        if let Some(f) = f {
            f(config);
        }
        self.close();
    }

    fn close(&self) {
        self.timer.stop();
        self.ui.borrow().hide().ok();
    }
}

fn with(w: &Weak<Inner>, f: impl FnOnce(&Inner)) {
    if let Some(i) = w.upgrade() {
        f(&i);
    }
}

/// The settings window. One per process; `open` shows it again with fresh
/// values.
pub struct SettingsWindow {
    inner: Rc<Inner>,
}

impl SettingsWindow {
    /// Creates a window in the requested initial language.
    pub fn new(lang: Lang) -> Result<Self, slint::PlatformError> {
        let ui = SettingsUi::new()?;
        let inner = Rc::new(Inner {
            rtl: Cell::new(lang.rtl()),
            initial_lang: lang,
            ui: RefCell::new(ui),
            on_save: RefCell::new(None),
            update: RefCell::new(UpdateModel::default()),
            on_update_check: RefCell::new(None),
            on_update_apply: RefCell::new(None),
            about: Default::default(),
            timer: slint::Timer::default(),
            opened: RefCell::new(Config::default()),
        });
        let ui = inner.new_ui()?;
        *inner.ui.borrow_mut() = ui;
        Ok(Self { inner })
    }

    /// Fills the window from `config` and shows it as a normal decorated
    /// window. A fresh window is built on every call, so a theme saved as
    /// automatic follows the system again after a forced light or dark one.
    pub fn open(&self, config: &Config) -> Result<(), slint::PlatformError> {
        let inner = &self.inner;
        inner.close();
        inner.update.borrow_mut().reset_view();
        let lang = match config.language {
            LangSetting::Auto => inner.initial_lang,
            _ => config.lang(),
        };
        inner.rtl.set(lang.rtl());
        let ui = inner.new_ui()?;
        let env = std::env::var("KAWARO_COLOR_SCHEME").ok();
        if let Some(scheme) = forced_scheme(env.as_deref(), config.theme) {
            ui.global::<Palette>().set_color_scheme(scheme);
        }
        ui.set_page(i32::try_from(remembered_page()).unwrap_or(0));
        ui.set_shortcut_text(config.shortcut.as_str().into());
        *inner.ui.borrow_mut() = ui;
        *inner.opened.borrow_mut() = config.clone();
        let state = FormState::from_config(config, inner.rtl.get());
        inner.set_state(&state);
        inner.set_loaded(&state);
        let w = Rc::downgrade(inner);
        inner
            .timer
            .start(slint::TimerMode::Repeated, POLL_TICK, move || {
                with(&w, |i| i.poll_tick(clock_ms()));
            });
        inner.ui.borrow().show()
    }

    /// Whether the window is currently shown (S-086 AC2): an open window is
    /// raised, never rebuilt.
    pub fn is_visible(&self) -> bool {
        self.inner.visible()
    }

    /// The native handle of the shown window, for the foreground call
    /// (S-086 AC2).
    #[cfg(windows)]
    pub fn hwnd(&self) -> Option<isize> {
        crate::win_window::hwnd(self.inner.ui.borrow().window())
    }

    /// Called with the edited settings when Save is pressed, before the
    /// window closes. Cancel, Esc and the window close button do not call it.
    pub fn on_save(&self, f: impl Fn(Config) + 'static) {
        *self.inner.on_save.borrow_mut() = Some(Rc::new(f));
    }

    /// Shows the installed `version`, the update `state` and whether the
    /// background service answers (S-090 AC1, AC5). An answer re-enables the
    /// buttons that a pending request had disabled.
    pub fn set_update_view(&self, version: &str, state: UpdateState, service_running: bool) {
        let inner = &self.inner;
        inner
            .update
            .borrow_mut()
            .set_view(version, state, service_running);
        inner.show_update(&inner.ui.borrow(), inner.lang());
    }

    /// The update the daemon knows: its version, the download size in bytes
    /// and the notes of its signed manifest (S-099). Independent of
    /// `set_update_view`, in either order.
    pub fn set_update_available(
        &self,
        version: Option<&str>,
        size: Option<u64>,
        notes: Option<&Notes>,
    ) {
        let inner = &self.inner;
        inner
            .update
            .borrow_mut()
            .set_available(version, size, notes);
        inner.show_update(&inner.ui.borrow(), inner.lang());
    }

    /// Shows how the last install ended as a banner, or clears it (S-101 B3).
    pub fn show_update_result(&self, result: Option<crate::UpdateResult>) {
        let inner = &self.inner;
        inner.update.borrow_mut().set_result(result);
        inner.show_update(&inner.ui.borrow(), inner.lang());
    }

    /// Update now was accepted but waits for running jobs (S-054b AC7).
    pub fn show_update_deferred(&self) {
        let inner = &self.inner;
        inner.update.borrow_mut().set_deferred();
        inner.show_update(&inner.ui.borrow(), inner.lang());
    }

    /// Called when Check for updates is pressed while it is enabled.
    pub fn on_update_check(&self, f: impl Fn() + 'static) {
        *self.inner.on_update_check.borrow_mut() = Some(Rc::new(f));
    }

    /// Called when Update now is pressed while it is enabled.
    pub fn on_update_apply(&self, f: impl Fn() + 'static) {
        *self.inner.on_update_apply.borrow_mut() = Some(Rc::new(f));
    }

    #[doc(hidden)]
    pub fn update_version_text(&self) -> String {
        self.inner.ui.borrow().get_update_version_text().into()
    }

    /// The window clock calls this every 250 ms with monotonic milliseconds.
    #[doc(hidden)]
    pub fn poll_tick(&self, now_ms: u64) {
        self.inner.poll_tick(now_ms);
    }

    #[doc(hidden)]
    pub fn update_available_text(&self) -> Option<String> {
        let t: String = self.inner.ui.borrow().get_update_available_text().into();
        (!t.is_empty()).then_some(t)
    }

    #[doc(hidden)]
    pub fn update_banner(&self) -> Option<String> {
        let t: String = self.inner.ui.borrow().get_update_banner().into();
        (!t.is_empty()).then_some(t)
    }

    #[doc(hidden)]
    pub fn update_cards(&self) -> Vec<(String, Vec<String>)> {
        use slint::Model;
        let ui = self.inner.ui.borrow();
        let mut cards: Vec<(String, Vec<String>)> = ui
            .get_update_cards()
            .iter()
            .map(|c| {
                (
                    c.title.to_string(),
                    c.bullets.iter().map(|b| b.to_string()).collect(),
                )
            })
            .collect();
        // Shown with the available version first; read installed first.
        if self.inner.update.borrow().render(self.inner.lang()).swapped {
            cards.swap(0, 1);
        }
        cards
    }

    #[doc(hidden)]
    pub fn update_state_line(&self) -> String {
        self.inner.ui.borrow().get_update_state_line().into()
    }

    #[doc(hidden)]
    pub fn update_check_enabled(&self) -> bool {
        self.inner.ui.borrow().get_update_check_enabled()
    }

    #[doc(hidden)]
    pub fn update_apply_enabled(&self) -> bool {
        self.inner.ui.borrow().get_update_apply_enabled()
    }

    #[doc(hidden)]
    pub fn press_update_check(&self) {
        self.inner.ui.borrow().invoke_update_check();
    }

    #[doc(hidden)]
    pub fn press_update_apply(&self) {
        self.inner.ui.borrow().invoke_update_apply();
    }

    #[doc(hidden)]
    pub fn press_save(&self) {
        self.inner.ui.borrow().invoke_save();
    }

    #[doc(hidden)]
    pub fn press_cancel(&self) {
        self.inner.ui.borrow().invoke_cancel();
    }

    #[doc(hidden)]
    pub fn set_update_check_auto(&self, on: bool) {
        self.inner.ui.borrow().set_update_check_auto(on);
    }

    #[doc(hidden)]
    pub fn set_update_channel(&self, channel: Channel) {
        let i = to_visual(&CHANNELS, &channel, self.inner.rtl.get());
        self.inner.ui.borrow().set_update_channel(i);
    }

    #[doc(hidden)]
    pub fn set_update_mode(&self, mode: UpdateMode) {
        let i = to_visual(&MODES, &mode, self.inner.rtl.get());
        self.inner.ui.borrow().set_update_mode(i);
    }

    /// Hides the window without saving.
    pub fn close(&self) {
        self.inner.close();
    }

    /// Snapshot hook: renders the current window content.
    #[doc(hidden)]
    pub fn take_snapshot(
        &self,
    ) -> Result<slint::SharedPixelBuffer<slint::Rgba8Pixel>, slint::PlatformError> {
        self.inner.ui.borrow().window().take_snapshot()
    }

    /// Snapshot hook: opens the window on page `page` without touching the
    /// remembered one.
    #[doc(hidden)]
    pub fn show_page(&self, page: usize) {
        self.inner
            .ui
            .borrow()
            .set_page(i32::try_from(page).unwrap_or(0));
    }
}

#[cfg(feature = "testing-hooks")]
impl SettingsWindow {
    #[doc(hidden)]
    pub fn update_card_labels(&self) -> Vec<String> {
        crate::shell::probe::card_labels(&*self.inner.ui.borrow())
    }

    #[doc(hidden)]
    pub fn update_card_rect(&self, i: usize) -> crate::Rect {
        crate::shell::probe::card_rect(&*self.inner.ui.borrow(), i)
    }

    #[doc(hidden)]
    pub fn update_card_title_rect(&self, i: usize) -> crate::Rect {
        crate::shell::probe::card_title_rect(&*self.inner.ui.borrow(), i)
    }

    /// The automatic install sentence on screen; empty in ask mode.
    #[doc(hidden)]
    pub fn update_auto_notice(&self) -> String {
        self.inner.ui.borrow().get_update_auto_notice().into()
    }

    #[doc(hidden)]
    pub fn update_auto_notice_rect(&self) -> crate::Rect {
        crate::shell::probe::rect_of(&*self.inner.ui.borrow(), "AppShell::auto-notice")
    }

    #[doc(hidden)]
    pub fn update_state_line_rect(&self) -> crate::Rect {
        crate::shell::probe::rect_of(&*self.inner.ui.borrow(), "AppShell::state-line")
    }

    #[doc(hidden)]
    pub fn update_banner_rect(&self) -> crate::Rect {
        crate::shell::probe::rect_of(&*self.inner.ui.borrow(), "AppShell::banner")
    }

    /// The y of every form row of the Updates page.
    #[doc(hidden)]
    pub fn update_row_ys(&self) -> Vec<f32> {
        crate::shell::probe::row_ys(&*self.inner.ui.borrow())
    }
}

crate::shell::about_api!(SettingsWindow);

#[cfg(feature = "testing-hooks")]
impl SettingsWindow {
    #[doc(hidden)]
    pub fn set_reduced_motion(&self, reduced: bool) {
        let ui = self.inner.ui.borrow();
        ui.global::<generated::Tokens>().set_reduced_motion(reduced);
    }
}

#[cfg(feature = "testing-hooks")]
crate::shell::shell_probes!(SettingsWindow, |w, i| w.inner.ui.borrow().set_theme(i));

#[cfg(test)]
mod tests {
    use super::*;
    use kawaro_core::config::{HiveSurface, UpdateMode};
    use kawaro_core::update::Channel;
    use slint::language::ColorScheme;

    fn every_config() -> Vec<Config> {
        let mut all = Vec::new();
        for language in LANGUAGES {
            for theme in THEMES {
                for output in OUTPUTS {
                    for quality in QUALITIES {
                        for (notify, open) in [(false, false), (true, false), (false, true)] {
                            all.push(Config {
                                language,
                                theme,
                                output,
                                output_folder: "نتائج".into(),
                                quality,
                                open_folder_when_done: open,
                                notify,
                                hive_surface: HiveSurface::Auto,
                                hold_ms: 400,
                                portal_shortcut: true,
                                shortcut: "Ctrl+Alt+H".into(),
                                ..Config::default()
                            });
                        }
                    }
                }
            }
        }
        all
    }

    /// Built as a literal on `Config::default()` with assignments after it
    /// (S-084 tester repair): an exhaustive struct literal would break the
    /// whole test module the day a hidden setting is added to `Config`. The
    /// values stay the complete non-default set.
    fn non_default_config(language: LangSetting) -> Config {
        let mut c = Config {
            language,
            ..Config::default()
        };
        c.theme = Theme::Dark;
        c.output = OutputPlace::Subfolder;
        c.output_folder = "Chosen results".into();
        c.quality = Quality::Max;
        c.open_folder_when_done = true;
        c.notify = false;
        c.hive_surface = HiveSurface::Window;
        c.hold_ms = 1500;
        c.portal_shortcut = false;
        c.shortcut = "Ctrl+Alt+F9".into();
        c.update_check = false;
        c.update_channel = Channel::Beta;
        c.update_mode = UpdateMode::Auto;
        c.update_channel_present = true;
        c
    }

    #[test]
    fn s076a_ac3_form_round_trip_preserves_the_complete_non_default_config() {
        for rtl in [false, true] {
            let language = if rtl {
                LangSetting::Ar
            } else {
                LangSetting::En
            };
            let start = non_default_config(language);
            assert!(start.update_channel_present);
            let saved = FormState::from_config(&start, rtl).to_config(rtl);
            assert_eq!(saved, start, "rtl {rtl}");
            assert!(
                saved.update_channel_present,
                "rtl {rtl}: preserve the explicit update channel key"
            );
        }
    }

    #[test]
    fn s084_ac6_save_keeps_a_custom_windows_shortcut() {
        for rtl in [false, true] {
            let language = if rtl {
                LangSetting::Ar
            } else {
                LangSetting::En
            };
            let mut start = non_default_config(language);
            start.shortcut = "Ctrl+Alt+J".into();
            let saved = FormState::from_config(&start, rtl).to_config(rtl);
            assert_eq!(
                saved.shortcut, "Ctrl+Alt+J",
                "rtl {rtl}: a custom Windows shortcut survives the save (S-084 AC6)"
            );
        }
    }

    #[test]
    fn config_round_trips_through_the_form_in_both_directions() {
        for rtl in [false, true] {
            for c in every_config() {
                assert_eq!(
                    FormState::from_config(&c, rtl).to_config(rtl),
                    c,
                    "rtl {rtl}"
                );
            }
        }
    }

    #[test]
    fn form_state_of_the_defaults() {
        let c = Config::default();
        let actual = FormState::from_config(&c, false);
        let ltr = FormState {
            language: 0,
            theme: 0,
            output: 0,
            quality: 1,
            folder: "Kawaro".into(),
            notify: true,
            open_folder: false,
            ..actual.clone()
        };
        assert!(actual.to_config(false).portal_shortcut);
        assert_eq!(actual, ltr);
        // Arabic: the lists are mirrored, the middle stays in the middle.
        let actual = FormState::from_config(&c, true);
        let rtl = FormState {
            language: 2,
            theme: 2,
            output: 1,
            quality: 1,
            folder: "Kawaro".into(),
            notify: true,
            open_folder: false,
            ..actual.clone()
        };
        assert!(actual.to_config(true).portal_shortcut);
        assert_eq!(actual, rtl);
    }

    /// S-076a keeps `portal_shortcut` as hidden state. The form has no
    /// registration control, but a remembered denial still survives a round trip.
    #[test]
    fn s029_ac7_form_round_trip_keeps_portal_shortcut_false() {
        for rtl in [false, true] {
            for mut c in every_config() {
                c.portal_shortcut = false;
                let form = FormState::from_config(&c, rtl);
                let saved = form.to_config(rtl);
                assert!(!saved.portal_shortcut, "rtl {rtl}");
                assert_eq!(saved, c, "rtl {rtl}");
                assert_eq!(FormState::from_config(&saved, rtl), form);
            }
        }
    }

    #[test]
    fn each_field_maps_on_its_own() {
        let base = Config::default();
        let c = Config {
            language: LangSetting::En,
            theme: Theme::Dark,
            output: OutputPlace::Subfolder,
            output_folder: "out".into(),
            quality: Quality::Max,
            open_folder_when_done: true,
            notify: false,
            hive_surface: HiveSurface::Auto,
            hold_ms: 400,
            portal_shortcut: true,
            shortcut: "Ctrl+Alt+H".into(),
            ..Config::default()
        };
        let s = FormState::from_config(&c, false);
        assert_eq!(
            (s.language, s.theme, s.output, s.quality),
            (2, 2, 1, 2),
            "visual indices, left to right"
        );
        assert_eq!(s.folder, "out");
        assert!(s.open_folder && !s.notify);
        let s = FormState::from_config(&c, true);
        assert_eq!((s.language, s.theme, s.output, s.quality), (0, 0, 0, 0));

        // Changing one visual index changes one field only.
        let start = FormState::from_config(&base, false);
        let check = |f: &dyn Fn(&mut FormState), want: Config| {
            let mut s = start.clone();
            f(&mut s);
            assert_eq!(s.to_config(false), want);
        };
        check(
            &|s| s.language = 1,
            Config {
                language: LangSetting::Ar,
                ..base.clone()
            },
        );
        check(
            &|s| s.theme = 1,
            Config {
                theme: Theme::Light,
                ..base.clone()
            },
        );
        check(
            &|s| s.output = 1,
            Config {
                output: OutputPlace::Subfolder,
                ..base.clone()
            },
        );
        check(
            &|s| s.quality = 0,
            Config {
                quality: Quality::Light,
                ..base.clone()
            },
        );
        check(
            &|s| s.notify = false,
            Config {
                notify: false,
                ..base.clone()
            },
        );
        check(
            &|s| s.open_folder = true,
            Config {
                open_folder_when_done: true,
                ..base.clone()
            },
        );
        check(
            &|s| s.folder = "x".into(),
            Config {
                output_folder: "x".into(),
                ..base.clone()
            },
        );
    }

    #[test]
    fn rtl_reverses_the_visual_order() {
        assert_eq!(visual(0, 3, false), 0);
        assert_eq!(visual(2, 3, false), 2);
        assert_eq!(visual(0, 3, true), 2);
        assert_eq!(visual(2, 3, true), 0);
        assert_eq!(visual(1, 3, true), 1);
        assert_eq!(visual(0, 2, true), 1);
        for (i, lang) in LANGUAGES.iter().enumerate() {
            assert_eq!(to_visual(&LANGUAGES, lang, false), i as i32);
            assert_eq!(to_visual(&LANGUAGES, lang, true), 2 - i as i32);
            assert_eq!(from_visual(&LANGUAGES, i as i32, true), LANGUAGES[2 - i]);
        }
        assert_eq!(from_visual(&OUTPUTS, 0, true), OutputPlace::Subfolder);
        assert_eq!(from_visual(&OUTPUTS, 1, true), OutputPlace::Beside);
        assert_eq!(from_visual(&QUALITIES, 0, true), Quality::Max);
        assert_eq!(from_visual(&THEMES, 0, true), Theme::Dark);
        let s = strings(Lang::Ar);
        assert_eq!(
            ordered(&s.outputs, true),
            ["في مجلد فرعي", "بجانب الملف الأصلي"]
        );
        assert_eq!(ordered(&s.qualities, false)[0], "ملفات أصغر");
    }

    #[test]
    fn out_of_range_index_falls_back_to_the_default() {
        assert_eq!(from_visual(&QUALITIES, -1, false), Quality::Balanced);
        assert_eq!(from_visual(&QUALITIES, 3, false), Quality::Balanced);
        assert_eq!(from_visual(&OUTPUTS, 2, true), OutputPlace::Beside);
        assert_eq!(from_visual(&QUALITIES, 2, false), Quality::Max);
    }

    #[test]
    fn folder_name_is_trimmed_and_never_empty() {
        let mut s = FormState::from_config(&Config::default(), false);
        s.folder = "  out  ".into();
        assert_eq!(s.to_config(false).output_folder, "out");
        s.folder = "   ".into();
        assert_eq!(s.to_config(false).output_folder, "Kawaro");
    }

    #[test]
    fn colour_scheme_follows_env_then_theme() {
        assert_eq!(
            forced_scheme(Some("dark"), Theme::Light),
            Some(ColorScheme::Dark)
        );
        assert_eq!(
            forced_scheme(Some("light"), Theme::Dark),
            Some(ColorScheme::Light)
        );
        assert_eq!(forced_scheme(None, Theme::Dark), Some(ColorScheme::Dark));
        assert_eq!(forced_scheme(None, Theme::Light), Some(ColorScheme::Light));
        assert_eq!(forced_scheme(Some("other"), Theme::Auto), None);
        assert_eq!(forced_scheme(None, Theme::Auto), None);
    }

    #[test]
    fn every_string_is_set_and_has_no_long_dash() {
        for lang in [Lang::Ar, Lang::En] {
            let s = strings(lang);
            let t = &s.text;
            let all = [
                &t.title,
                &t.language,
                &t.theme,
                &t.output,
                &t.folder,
                &t.quality,
                &t.notify,
                &t.notify_long_only,
                &t.open_folder,
                &t.save,
                &t.cancel,
                &t.updates,
                &t.update_check,
                &t.update_apply,
                &t.update_auto,
                &t.update_channel,
                &t.update_mode,
            ]
            .map(|x| x.to_string())
            .into_iter()
            .chain(s.languages.iter().map(|x| x.to_string()))
            .chain(s.themes.iter().map(|x| x.to_string()))
            .chain(s.outputs.iter().map(|x| x.to_string()))
            .chain(s.qualities.iter().map(|x| x.to_string()));
            for x in all {
                assert!(!x.trim().is_empty());
                assert!(!x.contains(['\u{2014}', '\u{2013}']), "{x}");
            }
        }
        assert_eq!(strings(Lang::Ar).text.title, "إعدادات كوارو");
        assert_eq!(strings(Lang::En).text.title, "Kawaro settings");
    }

    mod window {
        use super::*;
        use i_slint_backend_testing::{AccessibleRole, ElementHandle, ElementRoot};
        use slint::LogicalPosition;
        use slint::platform::{Key, PointerEventButton, WindowEvent};

        /// S-094: the controls live on five pages, so the helper shows the
        /// page that holds the control before it looks for it. The tests
        /// below are otherwise unchanged.
        fn find(win: &SettingsWindow, label: &str, role: AccessibleRole) -> ElementHandle {
            let ui = win.inner.ui.borrow();
            let mut found: Vec<ElementHandle> = Vec::new();
            for page in 0..crate::shell::SETTINGS_PAGES {
                found = ElementHandle::find_by_accessible_label(&*ui, label)
                    .filter(|e| e.accessible_role() == Some(role))
                    .collect();
                if !found.is_empty() {
                    break;
                }
                ui.set_page(page as i32);
            }
            assert_eq!(found.len(), 1, "{label} {role:?}");
            found.into_iter().next().unwrap()
        }

        fn label_count(win: &SettingsWindow, label: &str) -> usize {
            let ui = win.inner.ui.borrow();
            ElementHandle::find_by_accessible_label(&*ui, label).count()
        }

        fn accessible_labels(win: &SettingsWindow) -> Vec<String> {
            let ui = win.inner.ui.borrow();
            ui.root_element()
                .query_descendants()
                .find_all()
                .into_iter()
                .filter_map(|e| e.accessible_label().map(|label| label.to_string()))
                .collect()
        }

        fn click(win: &SettingsWindow, e: &ElementHandle) {
            let p = e.absolute_position();
            let s = e.size();
            let position = LogicalPosition::new(p.x + s.width / 2.0, p.y + s.height / 2.0);
            let ui = win.inner.ui.borrow();
            let w = ui.window();
            w.dispatch_event(WindowEvent::PointerMoved { position });
            w.dispatch_event(WindowEvent::PointerPressed {
                position,
                button: PointerEventButton::Left,
            });
            w.dispatch_event(WindowEvent::PointerReleased {
                position,
                button: PointerEventButton::Left,
            });
            w.dispatch_event(WindowEvent::PointerExited);
        }

        fn key(win: &SettingsWindow, k: Key) {
            let text: SharedString = k.into();
            let ui = win.inner.ui.borrow();
            let w = ui.window();
            w.dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
            w.dispatch_event(WindowEvent::KeyReleased { text });
        }

        fn wired(lang: Lang) -> (SettingsWindow, Rc<RefCell<Vec<Config>>>) {
            let win = SettingsWindow::new(lang).unwrap();
            let saved = Rc::new(RefCell::new(Vec::new()));
            let s = saved.clone();
            win.on_save(move |c| s.borrow_mut().push(c));
            (win, saved)
        }

        /// S-019a (backlog/S-019-hive-at-pointer.md, AC1, manager decision
        /// 4): `hive_surface` has no control in the settings window, so a
        /// save from the window keeps the value it was opened with instead
        /// of writing the default back to config.toml.
        #[test]
        fn ac1_s019_save_keeps_hive_surface() {
            i_slint_backend_testing::init_no_event_loop();
            for value in [HiveSurface::Window, HiveSurface::Layer, HiveSurface::Auto] {
                for lang in [Lang::En, Lang::Ar] {
                    let (win, saved) = wired(lang);
                    let start = Config {
                        hive_surface: value,
                        quality: Quality::Max,
                        ..Config::default()
                    };
                    win.open(&start).unwrap();
                    let save = if lang == Lang::En { "Save" } else { "حفظ" };
                    click(&win, &find(&win, save, AccessibleRole::Button));
                    assert_eq!(*saved.borrow(), [start], "{value:?} {lang:?}");
                    win.close();
                }
            }
        }

        /// S-019e AC18: `hold_ms` has no control in the settings window
        /// either, so a save keeps the value the window was opened with and
        /// does not put it back to 400.
        #[test]
        fn ac18_s019e_save_keeps_hold_ms() {
            i_slint_backend_testing::init_no_event_loop();
            for text in ["hold_ms = 100\n", "hold_ms = 1500\n", "hold_ms = 2000\n"] {
                let dir = std::env::temp_dir()
                    .join(format!("kawaro-s019e-settings-{}", std::process::id()));
                std::fs::create_dir_all(&dir).unwrap();
                let p = dir.join("config.toml");
                std::fs::write(&p, text).unwrap();
                let (loaded, warn) = Config::load_from(&p);
                let _ = std::fs::remove_dir_all(&dir);
                assert!(warn.is_none(), "{text}: {warn:?}");
                assert_ne!(
                    loaded,
                    Config::default(),
                    "{text}: a value that is not the default"
                );
                for lang in [Lang::En, Lang::Ar] {
                    let (win, saved) = wired(lang);
                    let start = Config {
                        quality: Quality::Max,
                        ..loaded.clone()
                    };
                    win.open(&start).unwrap();
                    let save = if lang == Lang::En { "Save" } else { "حفظ" };
                    click(&win, &find(&win, save, AccessibleRole::Button));
                    assert_eq!(*saved.borrow(), [start], "{text} {lang:?}");
                    win.close();
                }
            }
        }

        /// S-014e1, manager answer 5 and S-076a: with no registration
        /// control in the window, a save keeps a remembered portal denial.
        #[test]
        fn s014e1_save_keeps_a_remembered_portal_denial() {
            i_slint_backend_testing::init_no_event_loop();
            for lang in [Lang::En, Lang::Ar] {
                let (win, saved) = wired(lang);
                let start = Config {
                    portal_shortcut: false,
                    ..Config::default()
                };
                win.open(&start).unwrap();
                let save = if lang == Lang::En { "Save" } else { "حفظ" };
                click(&win, &find(&win, save, AccessibleRole::Button));
                assert_eq!(*saved.borrow(), [start], "{lang:?}");
                win.close();
            }
        }

        #[test]
        fn s076a_ac1_settings_have_no_registration_or_technical_controls() {
            i_slint_backend_testing::init_no_event_loop();
            for lang in [Lang::En, Lang::Ar] {
                let en = lang == Lang::En;
                let (old_registration, output, quality, notify, open, save, cancel) = if en {
                    (
                        "Register the drop shortcut with the desktop",
                        "In a subfolder",
                        "Smaller files",
                        "Notify when a file or operation finishes",
                        "Open the folder when done",
                        "Save",
                        "Cancel",
                    )
                } else {
                    (
                        "تسجيل اختصار الإفلات لدى سطح المكتب",
                        "في مجلد فرعي",
                        "ملفات أصغر",
                        "إشعار عند انتهاء ملف أو عملية",
                        "فتح المجلد عند الانتهاء",
                        "حفظ",
                        "إلغاء",
                    )
                };
                let (win, _saved) = wired(lang);
                win.open(&Config::default()).unwrap();
                assert_eq!(
                    label_count(&win, old_registration),
                    0,
                    "{lang:?}: the superseded registration row is absent"
                );

                for text in accessible_labels(&win) {
                    let lower = text.to_lowercase();
                    for technical in ["portal", "daemon", "registry", "config", "ipc", "token"] {
                        assert!(
                            !lower.contains(technical),
                            "{lang:?}: accessible string {text:?} contains {technical:?}"
                        );
                    }
                }

                let s = strings(lang);
                let t = &s.text;
                let visible = [
                    &t.title,
                    &t.language,
                    &t.theme,
                    &t.output,
                    &t.folder,
                    &t.quality,
                    &t.notify,
                    &t.open_folder,
                    &t.save,
                    &t.cancel,
                ]
                .map(|x| x.to_string())
                .into_iter()
                .chain(s.languages.iter().map(|x| x.to_string()))
                .chain(s.themes.iter().map(|x| x.to_string()))
                .chain(s.outputs.iter().map(|x| x.to_string()))
                .chain(s.qualities.iter().map(|x| x.to_string()));
                for text in visible {
                    let lower = text.to_lowercase();
                    for technical in ["portal", "daemon", "registry", "config", "ipc", "token"] {
                        assert!(
                            !lower.contains(technical),
                            "{lang:?}: visible string {text:?} contains {technical:?}"
                        );
                    }
                }

                find(&win, output, AccessibleRole::ListItem);
                find(&win, quality, AccessibleRole::ListItem);
                find(&win, notify, AccessibleRole::Switch);
                find(&win, open, AccessibleRole::Switch);
                let save_x = find(&win, save, AccessibleRole::Button)
                    .absolute_position()
                    .x;
                let cancel_x = find(&win, cancel, AccessibleRole::Button)
                    .absolute_position()
                    .x;
                assert_eq!(save_x > cancel_x, en, "{lang:?}: action direction");
                win.close();
            }
        }

        #[test]
        fn s076a_ac2_folder_name_is_conditional_retained_and_cancelled() {
            i_slint_backend_testing::init_no_event_loop();
            for lang in [Lang::En, Lang::Ar] {
                let (folder, beside, subfolder, cancel) = if lang == Lang::En {
                    (
                        "Folder name",
                        "Next to the original",
                        "In a subfolder",
                        "Cancel",
                    )
                } else {
                    ("اسم المجلد", "بجانب الملف الأصلي", "في مجلد فرعي", "إلغاء")
                };
                let (win, saved) = wired(lang);
                win.open(&Config::default()).unwrap();
                assert_eq!(label_count(&win, folder), 0, "{lang:?}: beside original");

                click(&win, &find(&win, subfolder, AccessibleRole::ListItem));
                let input = find(&win, folder, AccessibleRole::TextInput);
                assert_eq!(input.accessible_enabled(), Some(true), "{lang:?}");
                input.set_accessible_value("Typed name");

                click(&win, &find(&win, beside, AccessibleRole::ListItem));
                assert_eq!(label_count(&win, folder), 0, "{lang:?}: removed again");
                click(&win, &find(&win, subfolder, AccessibleRole::ListItem));
                let input = find(&win, folder, AccessibleRole::TextInput);
                assert_eq!(input.accessible_value().as_deref(), Some("Typed name"));

                click(&win, &find(&win, cancel, AccessibleRole::Button));
                assert!(!win.inner.visible(), "{lang:?}: Cancel closes");
                assert!(saved.borrow().is_empty(), "{lang:?}: Cancel saves nothing");

                win.open(&Config::default()).unwrap();
                click(&win, &find(&win, subfolder, AccessibleRole::ListItem));
                find(&win, folder, AccessibleRole::TextInput).set_accessible_value("Typed name");
                click(&win, &find(&win, beside, AccessibleRole::ListItem));
                let save = if lang == Lang::En { "Save" } else { "حفظ" };
                click(&win, &find(&win, save, AccessibleRole::Button));
                assert_eq!(
                    saved.borrow()[0].output_folder,
                    "Typed name",
                    "{lang:?}: another output location preserves the folder name"
                );
                win.close();
            }
        }

        #[test]
        fn s076a_ac3_save_preserves_complete_config_and_changes_one_visible_field() {
            i_slint_backend_testing::init_no_event_loop();
            for lang in [Lang::En, Lang::Ar] {
                let language = if lang == Lang::En {
                    LangSetting::En
                } else {
                    LangSetting::Ar
                };
                let start = non_default_config(language);
                let (save, smaller) = if lang == Lang::En {
                    ("Save", "Smaller files")
                } else {
                    ("حفظ", "ملفات أصغر")
                };
                let (win, saved) = wired(lang);

                win.open(&start).unwrap();
                click(&win, &find(&win, save, AccessibleRole::Button));
                assert_eq!(
                    saved.borrow().as_slice(),
                    std::slice::from_ref(&start),
                    "{lang:?}: unedited Save"
                );
                assert!(saved.borrow()[0].update_channel_present, "{lang:?}");
                saved.borrow_mut().clear();

                win.open(&start).unwrap();
                click(&win, &find(&win, smaller, AccessibleRole::ListItem));
                click(&win, &find(&win, save, AccessibleRole::Button));
                let expected = Config {
                    quality: Quality::Light,
                    ..start.clone()
                };
                assert_eq!(*saved.borrow(), [expected], "{lang:?}: quality only");
                assert!(saved.borrow()[0].update_channel_present, "{lang:?}");
                win.close();
            }
        }

        #[test]
        fn s076a_ac4_visible_change_preserves_hidden_portal_denial() {
            i_slint_backend_testing::init_no_event_loop();
            for lang in [Lang::En, Lang::Ar] {
                let (registration, notify, save) = if lang == Lang::En {
                    (
                        "Register the drop shortcut with the desktop",
                        "Notify when a file or operation finishes",
                        "Save",
                    )
                } else {
                    (
                        "تسجيل اختصار الإفلات لدى سطح المكتب",
                        "إشعار عند انتهاء ملف أو عملية",
                        "حفظ",
                    )
                };
                let start = Config {
                    portal_shortcut: false,
                    ..Config::default()
                };
                let (win, saved) = wired(lang);
                win.open(&start).unwrap();
                assert_eq!(label_count(&win, registration), 0, "{lang:?}");
                find(&win, notify, AccessibleRole::Switch).invoke_accessible_default_action();
                click(&win, &find(&win, save, AccessibleRole::Button));
                let expected = Config {
                    notify: !start.notify,
                    ..start.clone()
                };
                assert_eq!(*saved.borrow(), [expected], "{lang:?}");
                assert!(!saved.borrow()[0].portal_shortcut, "{lang:?}");
                win.close();
            }
        }

        #[test]
        fn settings_window_behaviour() {
            i_slint_backend_testing::init_no_event_loop();
            // The hive stays up and untouched while the settings window
            // opens, saves and closes in the same process.
            let hive = crate::Hive::new(Lang::En).unwrap();
            let hive_calls = Rc::new(std::cell::Cell::new(0));
            let h = hive_calls.clone();
            hive.on_choose(move |_| h.set(h.get() + 1));
            let h = hive_calls.clone();
            hive.on_dismiss(move || h.set(h.get() + 1));
            let cell = crate::HiveCell {
                id: "a".into(),
                code: "A".into(),
                label: "a".into(),
                icon: String::new(),
                ring: 1,
                enabled: true,
                reason: String::new(),
            };
            hive.present("1", vec![cell], None).unwrap();

            let (win, saved) = wired(Lang::En);
            let start = Config {
                quality: Quality::Max,
                ..Config::default()
            };

            // open fills the window and shows it.
            win.open(&start).unwrap();
            assert!(win.inner.visible());
            assert_eq!(win.inner.state(), FormState::from_config(&start, false));
            assert_eq!(win.inner.ui.borrow().get_text().title, "Kawaro settings");
            assert_eq!(
                label_count(&win, "Folder name"),
                0,
                "no Folder name beside the original"
            );

            // English: Save on the right of Cancel.
            let save = find(&win, "Save", AccessibleRole::Button);
            assert!(
                save.absolute_position().x
                    > find(&win, "Cancel", AccessibleRole::Button)
                        .absolute_position()
                        .x
            );

            // Cancel closes without saving.
            click(&win, &find(&win, "Cancel", AccessibleRole::Button));
            assert!(!win.inner.visible());
            assert!(saved.borrow().is_empty());

            // Esc closes without saving, also while typing in the text field.
            win.open(&start).unwrap();
            key(&win, Key::Escape);
            assert!(!win.inner.visible());
            let sub = Config {
                output: OutputPlace::Subfolder,
                ..start.clone()
            };
            win.open(&sub).unwrap();
            let folder = find(&win, "Folder name", AccessibleRole::TextInput);
            click(&win, &folder);
            {
                let ui = win.inner.ui.borrow();
                let text: SharedString = "x".into();
                ui.window()
                    .dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
                ui.window()
                    .dispatch_event(WindowEvent::KeyReleased { text });
            }
            assert_eq!(
                folder.accessible_value().as_deref(),
                Some("Kawarox"),
                "focus is in the field"
            );
            key(&win, Key::Escape);
            assert!(!win.inner.visible());
            assert!(saved.borrow().is_empty());

            // Keyboard: Tab reaches the language options, Space picks one.
            win.open(&start).unwrap();
            key(&win, Key::Tab);
            key(&win, Key::Tab);
            key(&win, Key::Space);
            assert_eq!(
                win.inner.ui.borrow().get_language(),
                1,
                "العربية by keyboard"
            );
            win.close();
            assert!(saved.borrow().is_empty(), "changing language does not save");

            // Change every field through the controls, then Save.
            win.open(&start).unwrap();
            assert_eq!(win.inner.ui.borrow().get_text().title, "Kawaro settings");
            for label in ["Dark", "In a subfolder", "Smaller files"] {
                let item = find(&win, label, AccessibleRole::ListItem);
                click(&win, &item);
                assert_eq!(item.accessible_item_selected(), Some(true), "{label}");
            }
            let folder = find(&win, "Folder name", AccessibleRole::TextInput);
            assert_eq!(
                folder.accessible_enabled(),
                Some(true),
                "enabled for a subfolder"
            );
            folder.set_accessible_value("Converted");
            find(
                &win,
                "Only if it took 10 seconds or more",
                AccessibleRole::Switch,
            )
            .invoke_accessible_default_action();
            let notify = find(
                &win,
                "Notify when a file or operation finishes",
                AccessibleRole::Switch,
            );
            notify.invoke_accessible_default_action();
            assert_eq!(notify.accessible_checked(), Some(false));
            find(&win, "Open the folder when done", AccessibleRole::Switch)
                .invoke_accessible_default_action();
            click(&win, &find(&win, "العربية", AccessibleRole::ListItem));
            assert_eq!(win.inner.ui.borrow().get_folder(), "Converted");
            click(&win, &find(&win, "حفظ", AccessibleRole::Button));
            assert!(!win.inner.visible(), "Save closes the window");
            assert_eq!(
                *saved.borrow(),
                [Config {
                    language: LangSetting::Ar,
                    theme: Theme::Dark,
                    output: OutputPlace::Subfolder,
                    output_folder: "Converted".into(),
                    quality: Quality::Light,
                    open_folder_when_done: true,
                    notify: false,
                    notify_long_only: true,
                    hive_surface: HiveSurface::Auto,
                    hold_ms: 400,
                    portal_shortcut: true,
                    shortcut: "Ctrl+Alt+H".into(),
                    ..Config::default()
                }]
            );

            // A second open starts from the given config, not the last edit.
            win.open(&Config::default()).unwrap();
            assert_eq!(
                win.inner.state(),
                FormState::from_config(&Config::default(), false)
            );
            // close() hides without saving; Save on a hidden window does nothing.
            win.close();
            assert!(!win.inner.visible());
            win.inner.save();
            assert_eq!(saved.borrow().len(), 1);

            // The compositor's close button acts like Cancel.
            win.open(&Config::default()).unwrap();
            win.inner
                .ui
                .borrow()
                .window()
                .dispatch_event(WindowEvent::CloseRequested);
            assert!(!win.inner.visible());
            assert_eq!(saved.borrow().len(), 1);
            drop(win);
            assert!(hive.inner.visible(), "the hive is still up");
            assert_eq!(hive_calls.get(), 0, "no hive callback fired");
            hive.close();

            // Arabic: lists mirrored, Save on the left of Cancel.
            let (ar, saved) = wired(Lang::Ar);
            ar.open(&Config::default()).unwrap();
            assert_eq!(ar.inner.ui.borrow().get_text().title, "إعدادات كوارو");
            let save = find(&ar, "حفظ", AccessibleRole::Button);
            let cancel = find(&ar, "إلغاء", AccessibleRole::Button);
            assert!(save.absolute_position().x < cancel.absolute_position().x);
            let auto = find(&ar, "اللغة", AccessibleRole::List);
            let english: Vec<ElementHandle> =
                ElementHandle::find_by_accessible_label(&*ar.inner.ui.borrow(), "English")
                    .filter(|e| e.accessible_role() == Some(AccessibleRole::ListItem))
                    .collect();
            assert_eq!(
                english[0].accessible_item_index(),
                Some(0),
                "English first from the left"
            );
            assert!(auto.is_valid());
            click(&ar, &english[0]);
            click(&ar, &find(&ar, "Save", AccessibleRole::Button));
            assert_eq!(saved.borrow()[0].language, LangSetting::En);
            assert_eq!(saved.borrow()[0].output, OutputPlace::Beside);
        }
    }
}
