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
use sbarhive_core::config::{Config, HiveSurface, LangSetting, OutputPlace, Quality, Theme};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

mod generated {
    include!(concat!(env!("OUT_DIR"), "/settings_ui.rs"));
}

use generated::{Palette, SettingsText, SettingsUi};

/// Options in reading order, as shown to an English reader from left to right.
const LANGUAGES: [LangSetting; 3] = [LangSetting::Auto, LangSetting::Ar, LangSetting::En];
const THEMES: [Theme; 3] = [Theme::Auto, Theme::Light, Theme::Dark];
const OUTPUTS: [OutputPlace; 2] = [OutputPlace::Beside, OutputPlace::Subfolder];
const QUALITIES: [Quality; 3] = [Quality::Light, Quality::Balanced, Quality::Max];

/// Visual position of the option at reading position `i` among `n`.
fn visual(i: usize, n: usize, rtl: bool) -> usize {
    if rtl { n - 1 - i } else { i }
}

/// Visual index of `value` in `options`.
fn to_visual<T: PartialEq>(options: &[T], value: &T, rtl: bool) -> i32 {
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
fn ordered(items: &[&str], rtl: bool) -> Vec<SharedString> {
    let mut v: Vec<SharedString> = items.iter().map(|s| (*s).into()).collect();
    if rtl {
        v.reverse();
    }
    v
}

/// What the window shows: visual indices for the option lists.
#[derive(Clone, Debug, PartialEq)]
struct FormState {
    language: i32,
    theme: i32,
    output: i32,
    quality: i32,
    folder: String,
    notify: bool,
    open_folder: bool,
}

impl FormState {
    fn from_config(c: &Config, rtl: bool) -> Self {
        Self {
            language: to_visual(&LANGUAGES, &c.language, rtl),
            theme: to_visual(&THEMES, &c.theme, rtl),
            output: to_visual(&OUTPUTS, &c.output, rtl),
            quality: to_visual(&QUALITIES, &c.quality, rtl),
            folder: c.output_folder.clone(),
            notify: c.notify,
            open_folder: c.open_folder_when_done,
        }
    }

    /// The folder name is trimmed; an empty one goes back to the default.
    fn to_config(&self, rtl: bool) -> Config {
        let folder = self.folder.trim();
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
            hive_surface: HiveSurface::Auto,
        }
    }
}

/// All strings of the window in reading order.
struct Strings {
    text: SettingsText,
    languages: [&'static str; 3],
    themes: [&'static str; 3],
    outputs: [&'static str; 2],
    qualities: [&'static str; 3],
}

fn strings(lang: Lang) -> Strings {
    match lang {
        Lang::Ar => Strings {
            text: SettingsText {
                title: "إعدادات صبار هايف".into(),
                language: "اللغة".into(),
                theme: "المظهر".into(),
                output: "مكان النتائج".into(),
                folder: "اسم المجلد".into(),
                quality: "الجودة".into(),
                notify: "إشعار عند الانتهاء".into(),
                open_folder: "فتح المجلد عند الانتهاء".into(),
                save: "حفظ".into(),
                cancel: "إلغاء".into(),
            },
            languages: ["تلقائي", "العربية", "English"],
            themes: ["تلقائي", "فاتح", "داكن"],
            outputs: ["بجانب الملف الأصلي", "في مجلد فرعي"],
            qualities: ["ملفات أصغر", "متوازنة", "أفضل جودة"],
        },
        Lang::En => Strings {
            text: SettingsText {
                title: "SbarHive settings".into(),
                language: "Language".into(),
                theme: "Theme".into(),
                output: "Where results go".into(),
                folder: "Folder name".into(),
                quality: "Quality".into(),
                notify: "Notify when done".into(),
                open_folder: "Open the folder when done".into(),
                save: "Save".into(),
                cancel: "Cancel".into(),
            },
            languages: ["Automatic", "العربية", "English"],
            themes: ["Automatic", "Light", "Dark"],
            outputs: ["Next to the original", "In a subfolder"],
            qualities: ["Smaller files", "Balanced", "Best quality"],
        },
    }
}

/// Colour scheme the window is drawn in: `SBARHIVE_COLOR_SCHEME` first (as
/// for the hive), then the theme being edited. `None` follows the system.
fn forced_scheme(env: Option<&str>, theme: Theme) -> Option<slint::language::ColorScheme> {
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

struct Inner {
    rtl: bool,
    strings: Strings,
    ui: RefCell<SettingsUi>,
    on_save: RefCell<Option<SaveFn>>,
    /// `hive_surface` has no control here (S-019 decision 4): a save keeps
    /// the value the window was opened with.
    hive_surface: Cell<HiveSurface>,
}

impl Inner {
    fn new_ui(self: &Rc<Self>) -> Result<SettingsUi, slint::PlatformError> {
        let ui = SettingsUi::new()?;
        let s = &self.strings;
        let list = |items: &[&str]| ModelRc::new(VecModel::from(ordered(items, self.rtl)));
        ui.set_rtl(self.rtl);
        ui.set_text(s.text.clone());
        ui.set_language_items(list(&s.languages));
        ui.set_theme_items(list(&s.themes));
        ui.set_output_items(list(&s.outputs));
        ui.set_quality_items(list(&s.qualities));
        ui.set_subfolder_index(to_visual(&OUTPUTS, &OutputPlace::Subfolder, self.rtl));

        let w = Rc::downgrade(self);
        ui.on_save(move || with(&w, Inner::save));
        let w = Rc::downgrade(self);
        ui.on_cancel(move || with(&w, Inner::close));
        let w = Rc::downgrade(self);
        ui.window().on_close_requested(move || {
            with(&w, Inner::close);
            slint::CloseRequestResponse::HideWindow
        });
        Ok(ui)
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
            open_folder: ui.get_open_folder(),
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
        ui.set_open_folder(s.open_folder);
    }

    fn visible(&self) -> bool {
        self.ui.borrow().window().is_visible()
    }

    fn save(&self) {
        if !self.visible() {
            return;
        }
        let config = Config {
            hive_surface: self.hive_surface.get(),
            ..self.state().to_config(self.rtl)
        };
        let f = self.on_save.borrow().clone();
        if let Some(f) = f {
            f(config);
        }
        self.close();
    }

    fn close(&self) {
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
    /// The window language is fixed here; a language change applies the next
    /// time the daemon creates the window.
    pub fn new(lang: Lang) -> Result<Self, slint::PlatformError> {
        let ui = SettingsUi::new()?;
        let inner = Rc::new(Inner {
            rtl: lang.rtl(),
            strings: strings(lang),
            ui: RefCell::new(ui),
            on_save: RefCell::new(None),
            hive_surface: Cell::new(HiveSurface::Auto),
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
        let ui = inner.new_ui()?;
        let env = std::env::var("SBARHIVE_COLOR_SCHEME").ok();
        if let Some(scheme) = forced_scheme(env.as_deref(), config.theme) {
            ui.global::<Palette>().set_color_scheme(scheme);
        }
        *inner.ui.borrow_mut() = ui;
        inner.hive_surface.set(config.hive_surface);
        inner.set_state(&FormState::from_config(config, inner.rtl));
        inner.ui.borrow().show()
    }

    /// Called with the edited settings when Save is pressed, before the
    /// window closes. Cancel, Esc and the window close button do not call it.
    pub fn on_save(&self, f: impl Fn(Config) + 'static) {
        *self.inner.on_save.borrow_mut() = Some(Rc::new(f));
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use sbarhive_core::config::HiveSurface;
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
                            });
                        }
                    }
                }
            }
        }
        all
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
        let ltr = FormState {
            language: 0,
            theme: 0,
            output: 0,
            quality: 1,
            folder: "SbarHive".into(),
            notify: true,
            open_folder: false,
        };
        assert_eq!(FormState::from_config(&c, false), ltr);
        // Arabic: the lists are mirrored, the middle stays in the middle.
        let rtl = FormState {
            language: 2,
            theme: 2,
            output: 1,
            ..ltr
        };
        assert_eq!(FormState::from_config(&c, true), rtl);
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
        assert_eq!(s.to_config(false).output_folder, "SbarHive");
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
            for x in all {
                assert!(!x.trim().is_empty());
                assert!(!x.contains(['\u{2014}', '\u{2013}']), "{x}");
            }
        }
        assert_eq!(strings(Lang::Ar).text.title, "إعدادات صبار هايف");
        assert_eq!(strings(Lang::En).text.title, "SbarHive settings");
    }

    mod window {
        use super::*;
        use i_slint_backend_testing::{AccessibleRole, ElementHandle};
        use slint::LogicalPosition;
        use slint::platform::{Key, PointerEventButton, WindowEvent};

        fn find(win: &SettingsWindow, label: &str, role: AccessibleRole) -> ElementHandle {
            let ui = win.inner.ui.borrow();
            let found: Vec<ElementHandle> = ElementHandle::find_by_accessible_label(&*ui, label)
                .filter(|e| e.accessible_role() == Some(role))
                .collect();
            assert_eq!(found.len(), 1, "{label} {role:?}");
            found.into_iter().next().unwrap()
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
            assert_eq!(win.inner.ui.borrow().get_text().title, "SbarHive settings");
            let folder = find(&win, "Folder name", AccessibleRole::TextInput);
            assert_eq!(
                folder.accessible_enabled(),
                Some(false),
                "disabled next to the original"
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
                Some("SbarHivex"),
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

            // Change every field through the controls, then Save.
            win.open(&start).unwrap();
            for label in ["Dark", "العربية", "In a subfolder", "Smaller files"] {
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
            let notify = find(&win, "Notify when done", AccessibleRole::Switch);
            notify.invoke_accessible_default_action();
            assert_eq!(notify.accessible_checked(), Some(false));
            find(&win, "Open the folder when done", AccessibleRole::Switch)
                .invoke_accessible_default_action();
            click(&win, &find(&win, "Save", AccessibleRole::Button));
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
                    hive_surface: HiveSurface::Auto,
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
            assert_eq!(ar.inner.ui.borrow().get_text().title, "إعدادات صبار هايف");
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
            click(&ar, &save);
            assert_eq!(saved.borrow()[0].language, LangSetting::En);
            assert_eq!(saved.borrow()[0].output, OutputPlace::Beside);
        }
    }
}
