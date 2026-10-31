//! The app shell shared by the settings window and the welcome window
//! (S-094, `ui/shell.slint`): the page memory, the About privacy sentence,
//! and, behind the `testing-hooks` feature, the geometry and state probes
//! the behaviour tests read (`tests/s094_shell.rs`).

use crate::Lang;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A handler of the About actions: gets the resolved address or the text to
/// copy. Both are injected by the daemon; this crate never opens or copies.
pub(crate) type AboutFn = Rc<dyn Fn(&str)>;

/// The two injected handlers of the About page (S-094b).
#[derive(Default)]
pub(crate) struct AboutHandlers {
    pub(crate) link: RefCell<Option<AboutFn>>,
    pub(crate) copy: RefCell<Option<AboutFn>>,
}

impl AboutHandlers {
    /// An About link was pressed. An id the core does not know opens nothing.
    pub(crate) fn open_link(&self, id: &str) {
        let (Some(url), Some(f)) = (kawaro_core::about::link_url(id), self.link.borrow().clone())
        else {
            return;
        };
        f(url);
    }

    pub(crate) fn copy_version(&self) {
        let f = self.copy.borrow().clone();
        if let Some(f) = f {
            f(&kawaro_core::about::copy_text(env!("CARGO_PKG_VERSION")));
        }
    }
}

/// The handler setters both windows expose.
macro_rules! about_api {
    ($t:ty) => {
        impl $t {
            /// Called with the address of an About link the user pressed.
            pub fn on_open_link(&self, f: impl Fn(&str) + 'static) {
                *self.inner.about.link.borrow_mut() = Some(std::rc::Rc::new(f));
            }

            /// Called with the text Copy version puts on the clipboard.
            pub fn on_copy_version(&self, f: impl Fn(&str) + 'static) {
                *self.inner.about.copy.borrow_mut() = Some(std::rc::Rc::new(f));
            }
        }
    };
}
pub(crate) use about_api;

/// A rectangle in logical px, in window coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// The page the settings window last showed, in memory only (never written
/// to `config.toml`): the daemon keeps one process, so a new window opens on
/// it. 0 is General, which is also the first ever open.
static PAGE: AtomicUsize = AtomicUsize::new(0);

/// The number of settings pages, General to About.
pub(crate) const SETTINGS_PAGES: usize = 5;

pub(crate) fn remembered_page() -> usize {
    PAGE.load(Ordering::Relaxed)
}

pub(crate) fn remember_page(page: usize) {
    PAGE.store(page.min(SETTINGS_PAGES - 1), Ordering::Relaxed);
}

/// Forgets the remembered page, so the next window opens on General.
#[doc(hidden)]
pub fn reset_page_memory() {
    PAGE.store(0, Ordering::Relaxed);
}

/// The privacy sentence of the About page.
pub fn privacy_statement(lang: Lang) -> String {
    match lang {
        Lang::En => "No telemetry. No file leaves your computer.".to_string(),
        Lang::Ar => "بلا تتبع. لا يغادر أي ملف جهازك.".to_string(),
    }
}

#[cfg(feature = "testing-hooks")]
pub(crate) mod probe {
    use super::Rect;
    use i_slint_backend_testing::{AccessibleRole, ElementHandle, ElementRoot};
    use slint::ComponentHandle;

    fn rect(e: &ElementHandle) -> Rect {
        let p = e.absolute_position();
        let s = e.size();
        Rect {
            x: p.x,
            y: p.y,
            w: s.width,
            h: s.height,
        }
    }

    fn by_id(root: &impl ElementRoot, id: &str) -> Option<ElementHandle> {
        ElementHandle::find_by_element_id(root, id).next()
    }

    /// The rectangle of the element `id`; empty when it is not on screen.
    pub(crate) fn rect_of(root: &impl ElementRoot, id: &str) -> Rect {
        by_id(root, id).map(|e| rect(&e)).unwrap_or(Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        })
    }

    pub(crate) fn exists(root: &impl ElementRoot, id: &str) -> bool {
        by_id(root, id).is_some()
    }

    pub(crate) fn label_of(root: &impl ElementRoot, id: &str) -> String {
        by_id(root, id)
            .and_then(|e| e.accessible_label())
            .map(|s| s.to_string())
            .unwrap_or_default()
    }

    fn all(root: &impl ElementRoot) -> Vec<ElementHandle> {
        root.root_element().query_descendants().find_all()
    }

    fn tabs(root: &impl ElementRoot) -> Vec<ElementHandle> {
        all(root)
            .into_iter()
            .filter(|e| e.accessible_role() == Some(AccessibleRole::Tab))
            .collect()
    }

    pub(crate) fn rail_labels(root: &impl ElementRoot) -> Vec<String> {
        tabs(root)
            .iter()
            .map(|e| e.accessible_label().unwrap_or_default().to_string())
            .collect()
    }

    pub(crate) fn rail_item_rect(root: &impl ElementRoot, i: usize) -> Rect {
        tabs(root).get(i).map(rect).unwrap_or(Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        })
    }

    pub(crate) fn select_tab(root: &impl ElementRoot, i: usize) {
        if let Some(tab) = tabs(root).get(i) {
            tab.invoke_accessible_default_action();
        }
    }

    /// The `FormRow` elements of the current page, top to bottom.
    fn rows(root: &impl ElementRoot) -> Vec<ElementHandle> {
        let mut rows: Vec<ElementHandle> =
            ElementHandle::find_by_element_type_name(root, "FormRow").collect();
        rows.sort_by(|a, b| {
            let (a, b) = (rect(a), rect(b));
            a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x))
        });
        rows
    }

    /// The `WhatsNewCard` elements, top to bottom.
    fn cards(root: &impl ElementRoot) -> Vec<ElementHandle> {
        let mut cards: Vec<ElementHandle> =
            ElementHandle::find_by_element_type_name(root, "WhatsNewCard").collect();
        cards.sort_by(|a, b| rect(a).y.total_cmp(&rect(b).y));
        cards
    }

    pub(crate) fn card_labels(root: &impl ElementRoot) -> Vec<String> {
        cards(root)
            .iter()
            .map(|e| e.accessible_label().unwrap_or_default().to_string())
            .collect()
    }

    pub(crate) fn card_rect(root: &impl ElementRoot, i: usize) -> Rect {
        cards(root).get(i).map(rect).unwrap_or(Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        })
    }

    /// The title of card `i`, found by position inside the card.
    pub(crate) fn card_title_rect(root: &impl ElementRoot, i: usize) -> Rect {
        let card = card_rect(root, i);
        ElementHandle::find_by_element_id(root, "WhatsNewCard::title-text")
            .map(|e| rect(&e))
            .find(|r| r.y >= card.y - 0.5 && r.y + r.h <= card.y + card.h + 0.5)
            .unwrap_or(card)
    }

    pub(crate) fn row_ys(root: &impl ElementRoot) -> Vec<f32> {
        rows(root).iter().map(|e| rect(e).y).collect()
    }

    pub(crate) fn page_rows(root: &impl ElementRoot) -> Vec<String> {
        rows(root)
            .iter()
            .map(|e| e.accessible_description().unwrap_or_default().to_string())
            .collect()
    }

    pub(crate) fn row_rect(root: &impl ElementRoot, i: usize) -> Rect {
        rows(root).get(i).map(rect).unwrap_or(Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        })
    }

    /// The label or the control slot of row `i`, found by position inside
    /// the row.
    pub(crate) fn row_part(root: &impl ElementRoot, i: usize, part: &str) -> Rect {
        let row = row_rect(root, i);
        let id = format!("FormRow::{part}");
        ElementHandle::find_by_element_id(root, &id)
            .map(|e| rect(&e))
            .find(|r| {
                r.y >= row.y - 0.5
                    && r.y + r.h <= row.y + row.h + 0.5
                    && r.x >= row.x - 0.5
                    && r.x + r.w <= row.x + row.w + 0.5
            })
            .unwrap_or(row)
    }

    /// Accessible labels on screen: every non empty label of an element that
    /// is not plain text, plus the row names.
    pub(crate) fn visible_labels(root: &impl ElementRoot) -> Vec<String> {
        let mut out: Vec<String> = all(root)
            .iter()
            .filter(|e| e.accessible_role() != Some(AccessibleRole::Text))
            .filter_map(|e| e.accessible_label())
            .map(|s| s.to_string())
            .filter(|s| !s.trim().is_empty())
            .collect();
        out.extend(page_rows(root));
        out
    }

    pub(crate) fn visible_texts(root: &impl ElementRoot) -> Vec<String> {
        all(root)
            .iter()
            .filter(|e| e.accessible_role() == Some(AccessibleRole::Text))
            .filter_map(|e| e.accessible_label())
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// The buttons of the About page, in reading order: the two links, Copy
    /// version, Check for updates (the row reverses in Arabic).
    pub(crate) fn about_buttons(
        root: &(impl ElementRoot + ComponentHandle),
        rtl: bool,
    ) -> Vec<(String, Rect)> {
        let footer = rect_of(root, "AppShell::footer");
        let column = rect_of(root, "AppShell::column");
        let mut out: Vec<(String, Rect)> = all(root)
            .iter()
            .filter(|e| e.accessible_role() == Some(AccessibleRole::Button))
            .map(|e| {
                (
                    e.accessible_label().unwrap_or_default().to_string(),
                    rect(e),
                )
            })
            .filter(|(_, r)| r.x >= column.x - 0.5 && r.x + r.w <= column.x + column.w + 0.5)
            .filter(|(_, r)| footer.w == 0.0 || r.y + r.h <= footer.y + 0.5)
            .collect();
        out.sort_by(|a, b| a.1.x.total_cmp(&b.1.x));
        if rtl {
            out.reverse();
        }
        out
    }

    fn button_named(root: &impl ElementRoot, label: &str) -> Option<ElementHandle> {
        ElementHandle::find_by_accessible_label(root, label).next()
    }

    fn centre(e: &ElementHandle) -> slint::LogicalPosition {
        let r = rect(e);
        slint::LogicalPosition::new(r.x + r.w / 2.0, r.y + r.h / 2.0)
    }

    /// A real click at the centre of the element with this label.
    pub(crate) fn press_label(root: &(impl ElementRoot + ComponentHandle), label: &str) -> bool {
        use slint::platform::{PointerEventButton, WindowEvent};
        let Some(e) = button_named(root, label) else {
            return false;
        };
        let position = centre(&e);
        let w = root.window();
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
        true
    }

    /// Moves the focus with Tab to the About button with this label (the
    /// buttons are the first focusable things of the page, in tree order,
    /// which is left to right on screen), then sends Return.
    pub(crate) fn enter_on_label(root: &(impl ElementRoot + ComponentHandle), label: &str) -> bool {
        use slint::platform::{Key, WindowEvent};
        let Some(i) = about_buttons(root, false)
            .iter()
            .position(|(l, _)| l == label)
        else {
            return false;
        };
        let w = root.window();
        let press = |key: Key| {
            let text: slint::SharedString = key.into();
            w.dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
            w.dispatch_event(WindowEvent::KeyReleased { text });
        };
        // From the page scope, the first Tab reaches the first About button.
        for _ in 0..=i {
            press(Key::Tab);
        }
        press(Key::Return);
        true
    }

    /// The third About button is Copy version, the fourth Check for updates.
    pub(crate) fn about_label(
        root: &(impl ElementRoot + ComponentHandle),
        rtl: bool,
        i: usize,
    ) -> String {
        about_buttons(root, rtl)
            .get(i)
            .map(|b| b.0.clone())
            .unwrap_or_default()
    }

    pub(crate) fn about_enabled(
        root: &(impl ElementRoot + ComponentHandle),
        rtl: bool,
        i: usize,
    ) -> bool {
        let Some((label, _)) = about_buttons(root, rtl).into_iter().nth(i) else {
            return false;
        };
        button_named(root, &label).is_some_and(|e| e.accessible_enabled().unwrap_or(true))
    }

    struct ListItem {
        text: String,
        label: String,
        selected: bool,
        centre: (f32, f32),
        x: f32,
    }

    /// The options of the one list on the page (S-109), left to right on
    /// screen: visible text, accessible label, selected, and the centre.
    fn list_items(root: &impl ElementRoot) -> Vec<ListItem> {
        let texts: Vec<(String, Rect)> = all(root)
            .iter()
            .filter(|e| e.accessible_role() == Some(AccessibleRole::Text))
            .map(|e| {
                (
                    e.accessible_label().unwrap_or_default().to_string(),
                    rect(e),
                )
            })
            .collect();
        let mut items: Vec<ListItem> = all(root)
            .iter()
            .filter(|e| e.accessible_role() == Some(AccessibleRole::ListItem))
            .map(|e| {
                let r = rect(e);
                let visible = texts
                    .iter()
                    .find(|(t, tr)| {
                        !t.is_empty()
                            && tr.x >= r.x - 0.5
                            && tr.x + tr.w <= r.x + r.w + 0.5
                            && tr.y >= r.y - 0.5
                            && tr.y + tr.h <= r.y + r.h + 0.5
                    })
                    .map(|(t, _)| t.clone())
                    .unwrap_or_default();
                ListItem {
                    text: visible,
                    label: e.accessible_label().unwrap_or_default().to_string(),
                    selected: e.accessible_item_selected().unwrap_or(false),
                    centre: (r.x + r.w / 2.0, r.y + r.h / 2.0),
                    x: r.x,
                }
            })
            .collect();
        items.sort_by(|a, b| a.x.total_cmp(&b.x));
        items
    }

    /// The options in logical order (Off, All, Long), whatever the direction.
    pub(crate) fn notify_options(
        root: &impl ElementRoot,
        rtl: bool,
    ) -> Vec<(String, String, bool)> {
        let mut items = list_items(root);
        if rtl {
            items.reverse();
        }
        items
            .into_iter()
            .map(|i| (i.text, i.label, i.selected))
            .collect()
    }

    /// A real click on logical option `i`.
    pub(crate) fn pick_notify_option(
        root: &(impl ElementRoot + ComponentHandle),
        rtl: bool,
        i: usize,
    ) {
        use slint::platform::{PointerEventButton, WindowEvent};
        let mut items = list_items(root);
        if rtl {
            items.reverse();
        }
        let Some(item) = items.get(i) else { return };
        let position = slint::LogicalPosition::new(item.centre.0, item.centre.1);
        let w = root.window();
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

    /// Interactive elements, rail items and the logo with an empty label.
    pub(crate) fn unlabelled_controls(root: &impl ElementRoot) -> usize {
        use AccessibleRole::*;
        let empty = |e: &ElementHandle| e.accessible_label().is_none_or(|l| l.trim().is_empty());
        let controls = all(root)
            .iter()
            .filter(|e| {
                matches!(
                    e.accessible_role(),
                    Some(Button | Switch | Tab | ListItem | TextInput | Checkbox | RadioButton)
                )
            })
            .filter(|e| empty(e))
            .count();
        let logos = ElementHandle::find_by_element_id(root, "AppShell::logo")
            .filter(|e| empty(e))
            .count();
        controls + logos
    }
}

/// The probes both windows expose, with the same names. `$set_theme` sets
/// the theme index on the generated window.
#[cfg(feature = "testing-hooks")]
macro_rules! shell_probes {
    ($t:ty, $set_theme:expr) => {
        impl $t {
            #[doc(hidden)]
            pub fn set_window_size(&self, w: f32, h: f32) {
                self.inner
                    .ui
                    .borrow()
                    .window()
                    .set_size(slint::LogicalSize::new(w, h));
            }

            #[doc(hidden)]
            pub fn window_size(&self) -> (f32, f32) {
                let ui = self.inner.ui.borrow();
                let w = ui.window();
                let s = w.size().to_logical(w.scale_factor());
                (s.width, s.height)
            }

            #[doc(hidden)]
            pub fn min_window_size(&self) -> (f32, f32) {
                let ui = self.inner.ui.borrow();
                (ui.get_window_min_width(), ui.get_window_min_height())
            }

            #[doc(hidden)]
            pub fn page(&self) -> usize {
                usize::try_from(self.inner.ui.borrow().get_page()).unwrap_or(0)
            }

            #[doc(hidden)]
            pub fn select_page(&self, i: usize) {
                crate::shell::probe::select_tab(&*self.inner.ui.borrow(), i);
                // Settle the layout (the page's repeaters instantiate on the first
                // reads), so the extents read right after the switch are current.
                slint::platform::update_timers_and_animations();
                let ui = self.inner.ui.borrow();
                for _ in 0..3 {
                    let _ = (
                        ui.get_scroll_content_height(),
                        ui.get_scroll_visible_height(),
                    );
                    let _ = crate::shell::probe::rect_of(&*ui, "AppShell::column");
                }
            }

            #[doc(hidden)]
            pub fn rail_labels(&self) -> Vec<String> {
                crate::shell::probe::rail_labels(&*self.inner.ui.borrow())
            }

            #[doc(hidden)]
            pub fn rail_focus(&self) -> usize {
                usize::try_from(self.inner.ui.borrow().get_rail_focus()).unwrap_or(0)
            }

            #[doc(hidden)]
            pub fn focus_rail(&self) {
                self.inner.ui.borrow().invoke_focus_rail();
            }

            #[doc(hidden)]
            pub fn page_rows(&self) -> Vec<String> {
                crate::shell::probe::page_rows(&*self.inner.ui.borrow())
            }

            #[doc(hidden)]
            pub fn visible_labels(&self) -> Vec<String> {
                crate::shell::probe::visible_labels(&*self.inner.ui.borrow())
            }

            #[doc(hidden)]
            pub fn visible_texts(&self) -> Vec<String> {
                crate::shell::probe::visible_texts(&*self.inner.ui.borrow())
            }

            #[doc(hidden)]
            pub fn notify_options(&self) -> Vec<(String, String, bool)> {
                crate::shell::probe::notify_options(&*self.inner.ui.borrow(), self.inner.is_rtl())
            }

            #[doc(hidden)]
            pub fn pick_notify_option(&self, i: usize) {
                crate::shell::probe::pick_notify_option(
                    &*self.inner.ui.borrow(),
                    self.inner.is_rtl(),
                    i,
                );
            }

            #[doc(hidden)]
            pub fn notify_list_label(&self) -> String {
                crate::shell::probe::label_of(&*self.inner.ui.borrow(), "AppShell::notify-buttons")
            }

            #[doc(hidden)]
            pub fn unlabelled_controls(&self) -> usize {
                crate::shell::probe::unlabelled_controls(&*self.inner.ui.borrow())
            }

            #[doc(hidden)]
            pub fn header_rect(&self) -> crate::Rect {
                crate::shell::probe::rect_of(&*self.inner.ui.borrow(), "AppShell::header")
            }

            #[doc(hidden)]
            pub fn logo_rect(&self) -> crate::Rect {
                crate::shell::probe::rect_of(&*self.inner.ui.borrow(), "AppShell::logo")
            }

            #[doc(hidden)]
            pub fn name_rect(&self) -> crate::Rect {
                crate::shell::probe::rect_of(&*self.inner.ui.borrow(), "AppShell::name")
            }

            #[doc(hidden)]
            pub fn version_rect(&self) -> crate::Rect {
                crate::shell::probe::rect_of(&*self.inner.ui.borrow(), "AppShell::version")
            }

            #[doc(hidden)]
            pub fn rail_rect(&self) -> crate::Rect {
                crate::shell::probe::rect_of(&*self.inner.ui.borrow(), "AppShell::rail")
            }

            #[doc(hidden)]
            pub fn rail_item_rect(&self, i: usize) -> crate::Rect {
                crate::shell::probe::rail_item_rect(&*self.inner.ui.borrow(), i)
            }

            #[doc(hidden)]
            pub fn page_area_rect(&self) -> crate::Rect {
                crate::shell::probe::rect_of(&*self.inner.ui.borrow(), "AppShell::scroll")
            }

            #[doc(hidden)]
            pub fn content_rect(&self) -> crate::Rect {
                crate::shell::probe::rect_of(&*self.inner.ui.borrow(), "AppShell::column")
            }

            #[doc(hidden)]
            pub fn footer_rect(&self) -> crate::Rect {
                crate::shell::probe::rect_of(&*self.inner.ui.borrow(), "AppShell::footer")
            }

            #[doc(hidden)]
            pub fn row_rect(&self, i: usize) -> crate::Rect {
                crate::shell::probe::row_rect(&*self.inner.ui.borrow(), i)
            }

            #[doc(hidden)]
            pub fn row_label_rect(&self, i: usize) -> crate::Rect {
                crate::shell::probe::row_part(&*self.inner.ui.borrow(), i, "lab")
            }

            #[doc(hidden)]
            pub fn row_control_rect(&self, i: usize) -> crate::Rect {
                crate::shell::probe::row_part(&*self.inner.ui.borrow(), i, "slot")
            }

            #[doc(hidden)]
            pub fn film_rect(&self) -> Option<crate::Rect> {
                let ui = self.inner.ui.borrow();
                crate::shell::probe::exists(&*ui, "FilmCard::film")
                    .then(|| crate::shell::probe::rect_of(&*ui, "FilmCard::film"))
            }

            #[doc(hidden)]
            pub fn scroll_extent(&self) -> (f32, f32) {
                let ui = self.inner.ui.borrow();
                (
                    ui.get_scroll_content_height(),
                    ui.get_scroll_visible_height(),
                )
            }

            #[doc(hidden)]
            pub fn header_version_text(&self) -> String {
                crate::shell::probe::label_of(&*self.inner.ui.borrow(), "AppShell::version-text")
            }

            #[doc(hidden)]
            pub fn header_name_text(&self) -> String {
                crate::shell::probe::label_of(&*self.inner.ui.borrow(), "AppShell::name")
            }

            #[doc(hidden)]
            pub fn logo_label(&self) -> String {
                crate::shell::probe::label_of(&*self.inner.ui.borrow(), "AppShell::logo")
            }

            #[doc(hidden)]
            pub fn unsaved_marker_visible(&self) -> bool {
                crate::shell::probe::exists(&*self.inner.ui.borrow(), "AppShell::unsaved")
            }

            #[doc(hidden)]
            pub fn save_row_visible(&self) -> bool {
                crate::shell::probe::exists(&*self.inner.ui.borrow(), "AppShell::footer")
            }

            #[doc(hidden)]
            pub fn open_link(&self, id: &str) {
                self.inner.ui.borrow().invoke_open_link(id.into());
            }

            #[doc(hidden)]
            pub fn about_buttons(&self) -> Vec<(String, crate::Rect)> {
                crate::shell::probe::about_buttons(&*self.inner.ui.borrow(), self.inner.is_rtl())
            }

            #[doc(hidden)]
            pub fn press_label(&self, label: &str) -> bool {
                crate::shell::probe::press_label(&*self.inner.ui.borrow(), label)
            }

            #[doc(hidden)]
            pub fn enter_on_label(&self, label: &str) -> bool {
                // The page scope is a known starting point for the Tab walk.
                self.inner.ui.borrow().invoke_focus_page();
                crate::shell::probe::enter_on_label(&*self.inner.ui.borrow(), label)
            }

            #[doc(hidden)]
            pub fn press_copy_version(&self) {
                let label = self.copy_button_text();
                self.press_label(&label);
            }

            #[doc(hidden)]
            pub fn copy_button_text(&self) -> String {
                crate::shell::probe::about_label(&*self.inner.ui.borrow(), self.inner.is_rtl(), 2)
            }

            #[doc(hidden)]
            pub fn press_about_check(&self) {
                let label = crate::shell::probe::about_label(
                    &*self.inner.ui.borrow(),
                    self.inner.is_rtl(),
                    3,
                );
                self.press_label(&label);
            }

            #[doc(hidden)]
            pub fn about_check_enabled(&self) -> bool {
                crate::shell::probe::about_enabled(&*self.inner.ui.borrow(), self.inner.is_rtl(), 3)
            }

            #[doc(hidden)]
            pub fn set_theme(&self, theme: kawaro_core::config::Theme) {
                let rtl = self.inner.is_rtl();
                let i = crate::settings::to_visual(&crate::settings::THEMES, &theme, rtl);
                let set: fn(&Self, i32) = $set_theme;
                set(self, i);
            }

            /// Sends a key as the real keyboard does: the modifier keys go
            /// down first and come up last.
            #[doc(hidden)]
            pub fn send_key(&self, text: &str, ctrl: bool, shift: bool) {
                use slint::platform::{Key, WindowEvent};
                let ui = self.inner.ui.borrow();
                let w = ui.window();
                let modifier = |key: Key| slint::SharedString::from(key);
                if ctrl {
                    w.dispatch_event(WindowEvent::KeyPressed {
                        text: modifier(Key::Control),
                    });
                }
                if shift {
                    w.dispatch_event(WindowEvent::KeyPressed {
                        text: modifier(Key::Shift),
                    });
                }
                let text: slint::SharedString = text.into();
                w.dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
                w.dispatch_event(WindowEvent::KeyReleased { text });
                if shift {
                    w.dispatch_event(WindowEvent::KeyReleased {
                        text: modifier(Key::Shift),
                    });
                }
                if ctrl {
                    w.dispatch_event(WindowEvent::KeyReleased {
                        text: modifier(Key::Control),
                    });
                }
            }
        }
    };
}

#[cfg(feature = "testing-hooks")]
pub(crate) use shell_probes;

#[cfg(test)]
mod tests {
    use super::*;

    /// A remembered page never leaves the five settings pages.
    #[test]
    fn the_page_memory_is_clamped_to_the_last_settings_page() {
        remember_page(99);
        assert_eq!(remembered_page(), SETTINGS_PAGES - 1);
        remember_page(SETTINGS_PAGES - 1);
        assert_eq!(remembered_page(), 4);
        remember_page(0);
        assert_eq!(remembered_page(), 0);
    }
}
